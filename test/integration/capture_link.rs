//! capture-link のプロパティ（doc/context/capture-link/constraints.md の P-CAP-4〜13）。
//!
//! Device はファームの中核の状態機械で、USB の入力（代替設定、音声パケット、CDC の受信、DTR）を受け、
//! CDC へ送るバイト列を poll_tx で返す。ここではハードウェア無しで駆動する。

#[path = "../support/link.rs"]
mod link_support;

use std::collections::HashMap;

use capture_core::audio_in::AudioFrame;
use capture_core::device::{Device, SessionState};
use capture_core::link::{self, FrameKind, MAX_FRAMES_PER_DATA};
use link_support::{Out, crc16_reference, drain, publish, ramp};
use proptest::prelude::*;

const CAPACITY: usize = capture_core::session::CAPTURE_BUFFER_FRAMES;

fn connected() -> Box<Device> {
    let mut dev = Box::new(Device::new());
    dev.set_dtr(true);
    dev.set_alt_setting(1).unwrap();
    dev
}

// ---------------------------------------------------------------------------
// P-CAP-4: Rust の符号化は共通の試験ベクタと一致する（Python の復号も同じベクタを満たす）

#[test]
fn p_cap_4_encoder_matches_shared_vectors() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/link_vectors.json"
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for v in json["vectors"].as_array().unwrap() {
        let f = &v["fields"];
        let mut payload = Vec::new();
        let kind = match v["kind"].as_u64().unwrap() {
            0x01 => {
                payload.extend((f["session_id"].as_u64().unwrap() as u32).to_le_bytes());
                FrameKind::Started
            }
            0x02 => {
                payload.extend(f["position"].as_u64().unwrap().to_le_bytes());
                for fr in f["frames"].as_array().unwrap() {
                    let l = fr[0].as_i64().unwrap() as i16;
                    let r = fr[1].as_i64().unwrap() as i16;
                    payload.extend(AudioFrame { left: l, right: r }.to_le_bytes());
                }
                FrameKind::Data
            }
            0x03 => {
                payload.extend((f["session_id"].as_u64().unwrap() as u32).to_le_bytes());
                payload.extend(f["total_frames"].as_u64().unwrap().to_le_bytes());
                payload.extend(f["dropped_frames"].as_u64().unwrap().to_le_bytes());
                FrameKind::Ended
            }
            0x04 => {
                payload.extend(f["text"].as_str().unwrap().as_bytes());
                FrameKind::Info
            }
            0x7F => {
                payload.push(f["code"].as_u64().unwrap() as u8);
                FrameKind::Error
            }
            k => panic!("unknown kind {k}"),
        };
        let mut out = [0u8; link::MAX_FRAME_LEN];
        let n = link::encode(kind, &payload, &mut out);
        let hex: String = out[..n].iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, v["hex"].as_str().unwrap(), "vector {}", v["name"]);
    }
}

#[test]
fn crc16_is_ccitt_false() {
    assert_eq!(link::crc16(b"123456789"), 0x29B1);
}

proptest! {
    #[test]
    fn crc16_matches_reference(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        prop_assert_eq!(link::crc16(&bytes), crc16_reference(&bytes));
    }
}

// ---------------------------------------------------------------------------
// P-CAP-5〜7, 9〜11: 状態を持つモデルとの比較

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cmd {
    Start,
    Stop,
    Info,
}

impl Cmd {
    fn line(self) -> &'static [u8] {
        match self {
            Cmd::Start => b"START\n",
            Cmd::Stop => b"STOP\n",
            Cmd::Info => b"INFO\n",
        }
    }
}

#[derive(Debug, Clone)]
enum Op {
    /// audio-in が n 個の音声フレームを公開する
    Packet(usize),
    Cmd(Cmd),
    /// ハーネスが CDC を読み切る
    Read,
    /// ストリームの代替設定を切り替える
    Alt(u8),
    /// ハーネスが読み切ってから CDC を閉じ、開き直す
    Reconnect,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0usize..=60).prop_map(Op::Packet),
        3 => prop_oneof![Just(Cmd::Start), Just(Cmd::Stop), Just(Cmd::Info)].prop_map(Op::Cmd),
        3 => Just(Op::Read),
        1 => (0u8..=1).prop_map(Op::Alt),
        1 => Just(Op::Reconnect),
    ]
}

#[derive(Debug, PartialEq, Eq)]
enum Expect {
    Started(u32),
    Error(u8),
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelState {
    Idle,
    Recording(u32),
    Stopping(u32),
}

/// デバイスの外から見える振る舞いのモデル。DATA の分け方と欠落は予測せず、事後に検査する。
struct Model {
    state: ModelState,
    next_id: u32,
    stream_open: bool,
    next_frame_value: u64,
    accepted: HashMap<u32, Vec<AudioFrame>>,
    expected_controls: Vec<Expect>,
    observed_controls: Vec<Expect>,
    /// セッションごとの出力（STARTED と ENDED の間の DATA、ENDED）
    data: HashMap<u32, Vec<(u64, Vec<AudioFrame>)>>,
    ended: HashMap<u32, (u64, u64)>,
    out_session: Option<u32>,
    aborted: Vec<u32>,
}

impl Model {
    fn new() -> Self {
        Model {
            state: ModelState::Idle,
            next_id: 1,
            stream_open: true,
            next_frame_value: 0,
            accepted: HashMap::new(),
            expected_controls: vec![],
            observed_controls: vec![],
            data: HashMap::new(),
            ended: HashMap::new(),
            out_session: None,
            aborted: vec![],
        }
    }

    fn step(&mut self, dev: &mut Device, op: &Op) {
        match op {
            Op::Packet(n) => {
                let frames = ramp(self.next_frame_value, *n);
                self.next_frame_value += *n as u64;
                publish(dev, &frames);
                if let (true, ModelState::Recording(id)) = (self.stream_open, self.state) {
                    self.accepted.entry(id).or_default().extend(frames);
                }
            }
            Op::Cmd(c) => {
                dev.cdc_rx(c.line());
                match (c, self.state) {
                    (Cmd::Start, ModelState::Idle) => {
                        self.expected_controls.push(Expect::Started(self.next_id));
                        self.state = ModelState::Recording(self.next_id);
                        self.accepted.insert(self.next_id, vec![]);
                        self.next_id += 1;
                    }
                    (Cmd::Start, _) => self.expected_controls.push(Expect::Error(1)),
                    (Cmd::Stop, ModelState::Recording(id)) => self.state = ModelState::Stopping(id),
                    (Cmd::Stop, _) => self.expected_controls.push(Expect::Error(2)),
                    (Cmd::Info, _) => self.expected_controls.push(Expect::Info),
                }
            }
            Op::Read => self.read(dev),
            Op::Alt(a) => {
                dev.set_alt_setting(*a).unwrap();
                self.stream_open = *a == 1;
            }
            Op::Reconnect => {
                self.read(dev);
                dev.set_dtr(false);
                dev.set_dtr(true);
                if let ModelState::Recording(id) | ModelState::Stopping(id) = self.state {
                    self.aborted.push(id);
                }
                self.state = ModelState::Idle;
                self.out_session = None;
            }
        }
    }

    fn read(&mut self, dev: &mut Device) {
        for out in drain(dev) {
            match out {
                Out::Started(id) => {
                    self.observed_controls.push(Expect::Started(id));
                    self.out_session = Some(id);
                }
                Out::Error(c) => self.observed_controls.push(Expect::Error(c)),
                Out::Info(_) => self.observed_controls.push(Expect::Info),
                Out::Data { position, frames } => {
                    let id = self.out_session.expect("DATA outside a session (P-CAP-10)");
                    self.data.entry(id).or_default().push((position, frames));
                }
                Out::Ended {
                    session_id,
                    total_frames,
                    dropped_frames,
                } => {
                    assert_eq!(
                        self.out_session,
                        Some(session_id),
                        "ENDED for another session (P-CAP-10)"
                    );
                    assert!(
                        self.ended
                            .insert(session_id, (total_frames, dropped_frames))
                            .is_none(),
                        "ENDED twice"
                    );
                    assert_eq!(
                        self.state,
                        ModelState::Stopping(session_id),
                        "ENDED without STOP"
                    );
                    self.state = ModelState::Idle;
                    self.out_session = None;
                }
            }
        }
    }

    fn finish(&mut self, dev: &mut Device) {
        self.read(dev);
        if let ModelState::Recording(_) = self.state {
            self.step(dev, &Op::Cmd(Cmd::Stop));
        }
        self.read(dev);
        assert_eq!(self.state, ModelState::Idle, "STOP never completed");
        assert_eq!(dev.session().state, SessionState::Idle);
    }

    fn check(&self) {
        assert_eq!(
            self.observed_controls, self.expected_controls,
            "control responses"
        );
        for (id, accepted) in &self.accepted {
            let chunks = self.data.get(id).map(Vec::as_slice).unwrap_or(&[]);
            let mut end = 0u64;
            let mut delivered = 0u64;
            for (pos, frames) in chunks {
                // P-CAP-9
                assert!(
                    (1..=MAX_FRAMES_PER_DATA).contains(&frames.len()),
                    "DATA size {}",
                    frames.len()
                );
                // P-CAP-6
                assert!(*pos >= end, "position went backwards: {pos} < {end}");
                // P-CAP-7
                let k = *pos as usize;
                assert_eq!(
                    &accepted[k..k + frames.len()],
                    &frames[..],
                    "frames at {pos}"
                );
                end = pos + frames.len() as u64;
                delivered += frames.len() as u64;
            }
            if let Some((total, dropped)) = self.ended.get(id) {
                // P-CAP-11
                assert_eq!(
                    *total,
                    accepted.len() as u64,
                    "total_frames of session {id}"
                );
                // P-CAP-5
                assert_eq!(delivered + dropped, *total, "conservation of session {id}");
                assert_eq!(*dropped, 0, "no overflow expected in this test");
            } else {
                assert!(
                    self.aborted.contains(id),
                    "session {id} neither ended nor aborted"
                );
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// P-CAP-5, 6, 7, 9, 10, 11: どんな操作列でも、出力はモデルと矛盾しない。
    #[test]
    fn p_cap_session_matches_model(ops in prop::collection::vec(op(), 0..60)) {
        let mut dev = connected();
        let mut model = Model::new();
        for op in &ops {
            model.step(&mut dev, op);
        }
        model.finish(&mut dev);
        model.check();
    }

    /// P-CAP-8: 読まれない間に n 個受けたら、届くのは先頭の min(n, 容量) 個で、残りは欠落として数えられる。
    #[test]
    fn p_cap_8_overflow_drops_newest(n in 0usize..=(CAPACITY + 3000)) {
        let mut dev = connected();
        dev.cdc_rx(b"START\n");
        let frames = ramp(0, n);
        publish(&mut dev, &frames);
        dev.cdc_rx(b"STOP\n");
        let outs = drain(&mut dev);
        let delivered: Vec<AudioFrame> = outs.iter().flat_map(|o| match o {
            Out::Data { frames, .. } => frames.clone(),
            _ => vec![],
        }).collect();
        let kept = n.min(CAPACITY);
        prop_assert_eq!(&delivered[..], &frames[..kept]);
        let ended = outs.iter().find_map(|o| match o {
            Out::Ended { total_frames, dropped_frames, .. } => Some((*total_frames, *dropped_frames)),
            _ => None,
        });
        prop_assert_eq!(ended, Some((n as u64, (n - kept) as u64)));
    }

    /// P-CAP-12: コマンドのバイト列をどこで分割して届けても、応答は変わらない。
    #[test]
    fn p_cap_12_command_split_does_not_matter(
        lines in prop::collection::vec(prop_oneof![
            Just("START\n"), Just("STOP\n"), Just("INFO\n"), Just("START\r\n"),
            Just("start\n"), Just("\n"), Just("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX\n"),
        ], 0..12),
        cuts in prop::collection::vec(any::<prop::sample::Index>(), 0..8),
    ) {
        let bytes: Vec<u8> = lines.concat().into_bytes();
        let mut whole = connected();
        whole.cdc_rx(&bytes);
        let expected = drain(&mut whole);

        let mut points: Vec<usize> = cuts.iter().map(|i| i.index(bytes.len() + 1)).collect();
        points.extend([0, bytes.len()]);
        points.sort();
        let mut split = connected();
        for w in points.windows(2) {
            split.cdc_rx(&bytes[w[0]..w[1]]);
        }
        prop_assert_eq!(drain(&mut split), expected);
    }

    /// P-CAP-13: INFO を任意の位置に挟んでも、INFO 以外の出力列は変わらない。
    #[test]
    fn p_cap_13_info_does_not_change_capture(
        ops in prop::collection::vec(op().prop_filter("no INFO", |o| !matches!(o, Op::Cmd(Cmd::Info))), 0..40),
        infos in prop::collection::vec(any::<prop::sample::Index>(), 0..6),
    ) {
        let run = |with_info: bool| {
            let mut dev = connected();
            let mut out = vec![];
            let at: Vec<usize> = infos.iter().map(|i| i.index(ops.len() + 1)).collect();
            for (i, op) in ops.iter().enumerate() {
                if with_info {
                    for _ in at.iter().filter(|&&k| k == i) {
                        dev.cdc_rx(b"INFO\n");
                    }
                }
                match op {
                    Op::Packet(n) => publish(&mut dev, &ramp(i as u64 * 100, *n)),
                    Op::Cmd(c) => dev.cdc_rx(c.line()),
                    Op::Read => out.extend(drain(&mut dev)),
                    Op::Alt(a) => dev.set_alt_setting(*a).unwrap(),
                    Op::Reconnect => {
                        out.extend(drain(&mut dev));
                        dev.set_dtr(false);
                        dev.set_dtr(true);
                    }
                }
            }
            out.extend(drain(&mut dev));
            out.retain(|o| !matches!(o, Out::Info(_)));
            out
        };
        prop_assert_eq!(run(true), run(false));
    }
}
