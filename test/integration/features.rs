//! doc/context/*/features/*.feature を capture-core のシミュレーションに対して実行する（ADR-0005）。
//!
//! `@harness` のシナリオはハーネス側の復号器の振る舞いなので、ここでは実行せず pytest-bdd で実行する。
//! 定義の無いステップはスキップではなく失敗として扱う。

#[path = "../support/link.rs"]
mod link_support;

use std::fmt;

use capture_core::audio_in::{AudioFrame, FEEDBACK_48K, HostControls, StreamState};
use capture_core::device::{Device, SessionState};
use capture_core::session::CAPTURE_BUFFER_FRAMES;
use cucumber::{World, given, then, when};
use futures::FutureExt;
use link_support::{Out, decode_all, packet_of};

#[derive(World)]
#[world(init = Self::new)]
struct W {
    dev: Box<Device>,
    /// ハーネスがこれまでに読んだリンクフレーム
    outs: Vec<Out>,
    /// 「〜が返る」の照合をどこまで済ませたか
    control_cursor: usize,
    /// 直近の When が始まった時点の outs の長さ
    mark: usize,
    /// ハーネスが CDC を読んでいるか
    reading: bool,
    last_packet: Vec<u8>,
    /// 今のセッションで、位置の順に audio-in が公開したフレーム（欠落したものも含む）
    session_published: Vec<AudioFrame>,
    /// セッションの外も含め、直近に公開したフレーム
    last_published: Vec<AudioFrame>,
    frames_before: u64,
    partial_before: u32,
    state_before: StreamState,
    alt_ok: Option<bool>,
    feedback: Vec<Option<[u8; 3]>>,
    controls: HostControls,
    next_value: u64,
    rng: u64,
}

impl fmt::Debug for W {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("W").field("outs", &self.outs.len()).field("session", &self.dev.session()).finish()
    }
}

impl W {
    fn new() -> Self {
        W {
            dev: Box::new(Device::new()),
            outs: vec![],
            control_cursor: 0,
            mark: 0,
            reading: true,
            last_packet: vec![],
            session_published: vec![],
            last_published: vec![],
            frames_before: 0,
            partial_before: 0,
            state_before: StreamState::Closed,
            alt_ok: None,
            feedback: vec![],
            controls: HostControls::default(),
            next_value: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn random_u16(&mut self) -> u16 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 24) as u16
    }

    fn random_frames(&mut self, n: usize) -> Vec<AudioFrame> {
        (0..n).map(|_| AudioFrame { left: self.random_u16() as i16, right: self.random_u16() as i16 }).collect()
    }

    fn ramp_frames(&mut self, n: usize) -> Vec<AudioFrame> {
        let v = link_support::ramp(self.next_value, n);
        self.next_value += n as u64;
        v
    }

    fn poll_one(&mut self) -> Option<Out> {
        let mut buf = [0u8; capture_core::link::MAX_FRAME_LEN];
        let n = self.dev.poll_tx(&mut buf);
        (n > 0).then(|| decode_all(&buf[..n]).pop().unwrap())
    }

    fn drain(&mut self) {
        while let Some(o) = self.poll_one() {
            self.outs.push(o);
        }
    }

    fn drain_if_reading(&mut self) {
        if self.reading {
            self.drain();
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.mark = self.outs.len();
        self.dev.cdc_rx(bytes);
    }

    fn ensure_open(&mut self) {
        if self.dev.audio().state() != StreamState::Open {
            self.dev.set_alt_setting(1).unwrap();
        }
    }

    /// audio-in が音声フレームを公開する。1 ms ぶん（48 個）ずつパケットにして届ける。
    fn publish(&mut self, frames: Vec<AudioFrame>) {
        self.ensure_open();
        let recording = self.dev.session().state == SessionState::Recording;
        for chunk in frames.chunks(48) {
            self.dev.iso_out(&packet_of(chunk));
            self.drain_if_reading();
        }
        if recording {
            self.session_published.extend(&frames);
        }
        self.last_published = frames;
    }

    fn deliver_packet(&mut self, packet: Vec<u8>) {
        self.drain();
        self.mark = self.outs.len();
        self.frames_before = self.dev.audio().frames_received();
        self.partial_before = self.dev.audio().partial_packets();
        self.dev.iso_out(&packet);
        self.last_packet = packet;
    }

    /// セッション id を開始する（必要なら先に空のセッションを開始・停止して番号を進める）。
    fn start_session(&mut self, id: u32) {
        self.dev.set_dtr(true);
        self.drain();
        while self.dev.session().session_id + 1 < id {
            self.dev.cdc_rx(b"START\nSTOP\n");
            self.drain();
        }
        self.dev.cdc_rx(b"START\n");
        self.drain();
        assert_eq!(self.dev.session().session_id, id);
        assert_eq!(self.dev.session().state, SessionState::Recording);
        self.control_cursor = self.outs.len();
        self.session_published.clear();
    }

    /// 現在（または直近）のセッションの出力。最後の STARTED から後。
    fn session_outs(&self) -> &[Out] {
        let start = self.outs.iter().rposition(|o| matches!(o, Out::Started(_))).map_or(0, |i| i + 1);
        &self.outs[start..]
    }

    fn session_data(&self) -> Vec<(u64, Vec<AudioFrame>)> {
        self.session_outs()
            .iter()
            .filter_map(|o| match o {
                Out::Data { position, frames } => Some((*position, frames.clone())),
                _ => None,
            })
            .collect()
    }

    fn frames_since_mark(&self) -> Vec<AudioFrame> {
        self.outs[self.mark..]
            .iter()
            .flat_map(|o| match o {
                Out::Data { frames, .. } => frames.clone(),
                _ => vec![],
            })
            .collect()
    }

    fn ended(&self) -> Option<(u32, u64, u64)> {
        self.outs.iter().rev().find_map(|o| match o {
            Out::Ended { session_id, total_frames, dropped_frames } => {
                Some((*session_id, *total_frames, *dropped_frames))
            }
            _ => None,
        })
    }

    fn last_infos(&self, n: usize) -> Vec<String> {
        let infos: Vec<String> =
            self.outs.iter().filter_map(|o| if let Out::Info(t) = o { Some(t.clone()) } else { None }).collect();
        assert!(infos.len() >= n, "expected {n} INFO, got {}", infos.len());
        infos[infos.len() - n..].to_vec()
    }

    /// 「STARTED(session_id=1)」「ERROR(UnknownCommand=3)」「INFO」のような記述を、次の応答と照合する。
    fn expect_control(&mut self, spec: &str) {
        self.drain();
        let (name, arg) = match spec.split_once('(') {
            Some((n, rest)) => (n, Some(rest.trim_end_matches(')'))),
            None => (spec, None),
        };
        let value = arg.and_then(|a| a.split('=').nth(1)).map(|v| v.parse::<u64>().unwrap());
        let idx = self.outs[self.control_cursor..]
            .iter()
            .position(|o| matches!(o, Out::Started(_) | Out::Error(_) | Out::Info(_)))
            .map(|i| i + self.control_cursor)
            .unwrap_or_else(|| panic!("no response; expected {spec}; outs={:?}", self.outs));
        let got = &self.outs[idx];
        match (name, got) {
            ("STARTED", Out::Started(id)) => assert!(value.is_none_or(|v| v == *id as u64), "{got:?} vs {spec}"),
            ("ERROR", Out::Error(c)) => assert!(value.is_none_or(|v| v == *c as u64), "{got:?} vs {spec}"),
            ("INFO", Out::Info(_)) => {}
            _ => panic!("expected {spec}, got {got:?}"),
        }
        self.control_cursor = idx + 1;
    }
}

fn unescape(s: &str) -> Vec<u8> {
    s.replace("\\n", "\n").replace("\\r", "\r").into_bytes()
}

fn info_line<'a>(info: &'a str, key: &str) -> Option<&'a str> {
    info.lines().find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

// ---------------------------------------------------------------------------
// audio-in

#[given("キャプチャデバイスが USB に接続され構成済みである")]
fn configured(_w: &mut W) {}

#[given(regex = r"^音声ストリームは (Open|Closed) である$")]
fn given_stream(w: &mut W, state: String) {
    w.dev.set_alt_setting(if state == "Open" { 1 } else { 0 }).unwrap();
    w.frames_before = w.dev.audio().frames_received();
}

#[when(regex = r"^ホストが代替設定 (\d+) を選ぶ$")]
fn select_alt(w: &mut W, alt: u8) {
    w.state_before = w.dev.audio().state();
    w.frames_before = w.dev.audio().frames_received();
    w.alt_ok = Some(w.dev.set_alt_setting(alt).is_ok());
}

#[when("USB がリセットされる")]
fn usb_reset(w: &mut W) {
    w.state_before = w.dev.audio().state();
    w.dev.usb_reset();
}

#[then(regex = r"^音声ストリームは (Open|Closed) (?:になる|のままである)$")]
fn then_stream(w: &mut W, state: String) {
    let expected = if state == "Open" { StreamState::Open } else { StreamState::Closed };
    assert_eq!(w.dev.audio().state(), expected);
}

#[then(regex = r"^(StreamOpened|StreamClosed) が発行される$")]
fn event_emitted(w: &mut W, event: String) {
    let (from, to) = if event == "StreamOpened" {
        (StreamState::Closed, StreamState::Open)
    } else {
        (StreamState::Open, StreamState::Closed)
    };
    assert_eq!((w.state_before, w.dev.audio().state()), (from, to));
}

#[then(regex = r"^(?:それまでに)?受けた音声フレームの数は変わらない$")]
fn frames_unchanged(w: &mut W) {
    assert_eq!(w.dev.audio().frames_received(), w.frames_before);
}

#[then("その要求は拒否される")]
fn rejected(w: &mut W) {
    assert_eq!(w.alt_ok, Some(false));
}

#[given(regex = r"^(\d+) フレーム分の音声パケットが届いている$")]
fn packets_arrived(w: &mut W, n: usize) {
    let frames = w.random_frames(n);
    w.deliver_packet(packet_of(&frames));
}

#[when(regex = r"^その後に音声パケットが 1 つ届く$")]
fn one_more_packet(w: &mut W) {
    let frames = w.random_frames(48);
    w.dev.iso_out(&packet_of(&frames));
}

#[then(regex = r"^公開された音声フレームは (\d+) 個である$")]
fn published_total(w: &mut W, n: u64) {
    assert_eq!(w.dev.audio().frames_received(), n);
}

#[when(regex = r"^(\d+) フレーム分の任意のサンプルを含む(?: (\d+) B の)?パケットが届く$")]
fn random_packet(w: &mut W, n: usize, bytes: String) {
    if !bytes.is_empty() {
        assert_eq!(bytes.parse::<usize>().unwrap(), n * 4);
    }
    let frames = w.random_frames(n);
    w.deliver_packet(packet_of(&frames));
}

#[when(regex = r"^(\d+) B のパケットが届く$")]
fn sized_packet(w: &mut W, n: usize) {
    let packet: Vec<u8> = (0..n).map(|_| w.random_u16() as u8).collect();
    w.deliver_packet(packet);
}

#[when(regex = r"^L が (-?\d+)、R が (-?\d+) のフレームを含むパケットが届く$")]
fn extreme_packet(w: &mut W, left: i16, right: i16) {
    w.deliver_packet(packet_of(&[AudioFrame { left, right }]));
}

#[when(regex = r"^同じ (\d+) B のパケットが 2 回続けて届く$")]
fn same_packet_twice(w: &mut W, n: usize) {
    let packet: Vec<u8> = (0..n).map(|_| w.random_u16() as u8).collect();
    w.deliver_packet(packet.clone());
    w.dev.iso_out(&packet);
}

#[then(regex = r"^(\d+) 個の音声フレームが公開される$")]
fn published_delta(w: &mut W, n: u64) {
    w.drain();
    assert_eq!(w.dev.audio().frames_received() - w.frames_before, n);
    assert_eq!(w.frames_since_mark().len() as u64, n);
}

#[then(regex = r"^(?:各フレームの L と R は、パケットのバイト列をリトルエンディアンで読んだ値と一致する|公開された音声フレームは、パケットのバイト列と完全に一致する)$")]
fn published_bytes_match(w: &mut W) {
    w.drain();
    let bytes: Vec<u8> = w.frames_since_mark().iter().flat_map(|f| f.to_le_bytes()).collect();
    let p = &w.last_packet;
    assert_eq!(&bytes[..], &p[..p.len() - p.len() % 4]);
}

#[then(regex = r"^端数バイトのあったパケット数は (\d+) 増える$")]
fn partial_delta(w: &mut W, n: u32) {
    assert_eq!(w.dev.audio().partial_packets() - w.partial_before, n);
}

#[then(regex = r"^公開されたフレームの L は (-?\d+)、R は (-?\d+) である$")]
fn published_values(w: &mut W, left: i16, right: i16) {
    w.drain();
    assert_eq!(w.frames_since_mark(), vec![AudioFrame { left, right }]);
}

#[then("音声フレームは公開されない")]
fn nothing_published(w: &mut W) {
    w.drain();
    assert!(w.frames_since_mark().is_empty());
    assert_eq!(w.dev.audio().frames_received(), w.frames_before);
}

#[given(regex = r"^ホストが両チャンネルの音量を (-?\d+) dB に設定(?:している|し、(L|R) をミュートしている)$")]
fn set_volume(w: &mut W, db: i16, mute: String) {
    w.controls.volume_8q8_db = [db * 256; 2];
    match mute.as_str() {
        "L" => w.controls.muted[0] = true,
        "R" => w.controls.muted[1] = true,
        _ => {}
    }
    w.dev.set_host_controls(w.controls);
}

#[given(regex = r"^ホストが (L|R|両) ?チャンネルをミュートしている$")]
fn set_mute(w: &mut W, ch: String) {
    match ch.as_str() {
        "L" => w.controls.muted[0] = true,
        "R" => w.controls.muted[1] = true,
        _ => w.controls.muted = [true, true],
    }
    w.dev.set_host_controls(w.controls);
}

#[then(regex = r"^ホスト制御値には音量 (-?\d+) dB と (L|R) のミュートが記録されている$")]
fn controls_recorded(w: &mut W, db: i16, ch: String) {
    let c = w.dev.audio().host_controls();
    assert_eq!(c.volume_8q8_db, [db * 256; 2]);
    assert!(c.muted[if ch == "L" { 0 } else { 1 }]);
}

#[given("録音セッション中でキャプチャバッファが満杯である")]
fn buffer_full(w: &mut W) {
    w.reading = false;
    let frames = w.ramp_frames(CAPTURE_BUFFER_FRAMES);
    w.publish(frames);
    assert_eq!(w.dev.session().buffered_frames, CAPTURE_BUFFER_FRAMES);
}

#[when(regex = r"^ホストがフィードバックを(?: (\d+) 回)?要求する$")]
fn request_feedback(w: &mut W, times: String) {
    let n = times.parse::<usize>().unwrap_or(1);
    for _ in 0..n {
        w.feedback.push(w.dev.feedback());
    }
}

#[then(regex = r"^(?:返る 3 バイトは|すべての応答は) 00 00 0C である$")]
fn feedback_is_48k(w: &mut W) {
    assert!(!w.feedback.is_empty());
    assert!(w.feedback.iter().all(|f| *f == Some(FEEDBACK_48K)), "{:?}", w.feedback);
}

#[then("フィードバックは返らない")]
fn no_feedback(w: &mut W) {
    assert_eq!(w.feedback.last(), Some(&None));
}

// ---------------------------------------------------------------------------
// capture-link: 前提

#[given("CDC が開かれている")]
fn cdc_open(w: &mut W) {
    w.dev.set_dtr(true);
}

#[given("録音セッションは Idle である")]
fn session_idle(w: &mut W) {
    assert_eq!(w.dev.session().state, SessionState::Idle);
}

#[given("これまでに開始したセッションは無い")]
fn no_sessions_yet(w: &mut W) {
    assert_eq!(w.dev.session().session_id, 0);
}

#[given(regex = r"^セッション (\d+) を開始して停止した$")]
fn started_and_stopped(w: &mut W, id: u32) {
    w.start_session(id);
    w.dev.cdc_rx(b"STOP\n");
    w.drain();
    assert_eq!(w.ended().map(|e| e.0), Some(id));
    w.control_cursor = w.outs.len();
}

#[given(regex = r"^セッション (\d+) が Recording である$")]
fn recording_id(w: &mut W, id: u32) {
    w.start_session(id);
}

#[given("録音セッションが Recording である")]
fn recording(w: &mut W) {
    let id = w.dev.session().session_id + 1;
    w.start_session(id);
}

#[given(regex = r"^セッション (\d+) が Recording で、位置 (\d+) まで録音している$")]
fn recording_to(w: &mut W, id: u32, pos: usize) {
    w.start_session(id);
    let frames = w.ramp_frames(pos);
    w.publish(frames);
    w.drain();
}

#[given(regex = r"^位置 (\d+) まで録音している$")]
fn recorded_to(w: &mut W, pos: usize) {
    let frames = w.ramp_frames(pos);
    w.publish(frames);
    w.drain();
}

#[given(regex = r"^セッション (\d+) が Recording で、(\d+) 個のフレームを受けている$")]
fn recording_received(w: &mut W, id: u32, n: usize) {
    w.start_session(id);
    w.reading = false;
    let frames = w.ramp_frames(n);
    w.publish(frames);
}

#[given(regex = r"^そのうち (\d+) 個はまだ送られていない$")]
fn some_unsent(w: &mut W, k: usize) {
    while w.dev.session().buffered_frames > k {
        let o = w.poll_one().expect("data to read");
        w.outs.push(o);
    }
    assert_eq!(w.dev.session().buffered_frames, k);
}

#[given(regex = r"^セッション (\d+) が Recording で、位置 (\d+) まで受け、欠落は (\d+) である$")]
fn recording_with_drops(w: &mut W, id: u32, pos: usize, dropped: u64) {
    w.start_session(id);
    w.reading = false;
    let frames = w.ramp_frames(pos);
    w.publish(frames);
    assert_eq!(w.dev.session().dropped_frames, dropped);
}

#[given(regex = r"^セッション 1 が Recording で、バッファ溢れにより (\d+) フレームが欠落している$")]
fn recording_overflowed(w: &mut W, dropped: usize) {
    w.start_session(1);
    w.reading = false;
    let frames = w.ramp_frames(CAPTURE_BUFFER_FRAMES + dropped);
    w.publish(frames);
    assert_eq!(w.dev.session().dropped_frames, dropped as u64);
}

#[given(regex = r"^セッション (\d+) が Stopping である$")]
fn stopping(w: &mut W, id: u32) {
    w.start_session(id);
    w.reading = false;
    let frames = w.ramp_frames(48);
    w.publish(frames);
    w.dev.cdc_rx(b"STOP\n");
    assert_eq!(w.dev.session().state, SessionState::Stopping);
}

#[given(regex = r"^セッション (\d+) を開始し、音声フレームはまだ届いていない$")]
fn started_no_frames(w: &mut W, id: u32) {
    w.start_session(id);
}

#[given(regex = r"^CDC が(?:読まれていない|しばらく読まれず、(\d+) 個のフレームがバッファに溜まっている)$")]
fn not_reading(w: &mut W, n: String) {
    w.reading = false;
    if let Ok(n) = n.parse::<usize>() {
        let frames = w.ramp_frames(n);
        w.publish(frames);
    }
}

#[given(regex = r"^audio-in が (\d+) 個の音声フレームを公開した$")]
fn published_before(w: &mut W, n: usize) {
    let frames = w.ramp_frames(n);
    w.publish(frames);
}

// ---------------------------------------------------------------------------
// capture-link: 操作

#[when(regex = r#"^ハーネスが "([^"]*)" を(?: 1 回で)?送る$"#)]
fn harness_sends(w: &mut W, line: String) {
    w.send(&unescape(&line));
}

#[when(regex = r#"^ハーネスが "([^"]*)" を (\d+) 回送る$"#)]
fn harness_sends_times(w: &mut W, line: String, n: usize) {
    for _ in 0..n {
        w.send(&unescape(&line));
    }
}

#[when(regex = r#"^ハーネスが "([^"]*)" を送り、続けて "([^"]*)" を送る$"#)]
fn harness_sends_split(w: &mut W, a: String, b: String) {
    w.send(&unescape(&a));
    w.dev.cdc_rx(&unescape(&b));
}

#[when(regex = r"^ハーネスが (\d+) バイト(?:ちょうど)?の(?:不明な)?行 を送る$")]
fn harness_sends_long(w: &mut W, n: usize) {
    let mut line = vec![b'X'; n];
    line.push(b'\n');
    w.send(&line);
}

#[when(regex = r#"^ハーネスが "STOP\\n" を送り、ENDED まで読む$"#)]
fn stop_and_read(w: &mut W) {
    w.send(b"STOP\n");
    w.reading = true;
    w.drain();
}

#[when(regex = r"^ハーネスが CDC を閉じる（DTR が下がる）$")]
fn close_cdc(w: &mut W) {
    w.dev.set_dtr(false);
}

#[when(regex = r#"^ハーネスが CDC を開き直して "([^"]*)" を送る$"#)]
fn reopen_and_send(w: &mut W, line: String) {
    w.dev.set_dtr(true);
    w.control_cursor = w.outs.len();
    w.session_published.clear();
    w.send(&unescape(&line));
}

#[when(regex = r"^audio-in が(?:さらに)? ?(\d+) 個の音声フレームを公開する$")]
fn publish_frames(w: &mut W, n: usize) {
    w.drain_if_reading();
    w.mark = w.outs.len();
    let frames = w.ramp_frames(n);
    w.publish(frames);
}

#[when(regex = r"^audio-in が任意のサンプルの音声フレームを (\d+) 個公開する$")]
fn publish_random(w: &mut W, n: usize) {
    w.mark = w.outs.len();
    let frames = w.random_frames(n);
    w.publish(frames);
}

#[when(regex = r"^ハーネスが(?:すべての DATA を読む|読み出しを再開する)$")]
fn harness_reads(w: &mut W) {
    w.mark = w.outs.len();
    w.reading = true;
    w.drain();
}

#[when("audio-in のストリームが閉じ、しばらくして再び開く")]
fn stream_bounces(w: &mut W) {
    w.dev.set_alt_setting(0).unwrap();
    w.dev.set_alt_setting(1).unwrap();
}

#[when(regex = r#"^audio-in が音声フレームを公開し続けている間にハーネスが "INFO\\n" を送る$"#)]
fn info_while_streaming(w: &mut W) {
    let a = w.ramp_frames(240);
    w.publish(a);
    w.dev.cdc_rx(b"INFO\n");
    let b = w.ramp_frames(240);
    w.publish(b);
    w.drain();
}

// ---------------------------------------------------------------------------
// capture-link: 結果

#[then(regex = r"^(STARTED|ERROR|INFO)(\([^)]*\))? が返る$")]
fn response(w: &mut W, name: String, args: String) {
    w.expect_control(&format!("{name}{args}"));
}

#[then(regex = r"^(\S+) と (\S+) がこの順で返る$")]
fn responses_in_order(w: &mut W, a: String, b: String) {
    w.expect_control(&a);
    w.expect_control(&b);
}

#[then("リンクフレームは返らない")]
fn nothing_returned(w: &mut W) {
    let before = w.outs.len();
    w.drain();
    assert_eq!(w.outs.len(), before, "{:?}", &w.outs[before..]);
}

#[then(regex = r"^その先頭のフレームの位置は (\d+) である$")]
fn first_position(w: &mut W, pos: u64) {
    w.drain();
    assert_eq!(w.session_data().first().map(|d| d.0), Some(pos));
}

#[then(regex = r"^録音される音声フレームは後の (\d+) 個だけである$")]
fn only_later_frames(w: &mut W, n: u64) {
    w.dev.cdc_rx(b"STOP\n");
    w.drain();
    let got: Vec<AudioFrame> = w.session_data().into_iter().flat_map(|d| d.1).collect();
    assert_eq!(got, w.last_published);
    assert_eq!(w.ended().map(|e| e.1), Some(n));
}

#[then(regex = r"^セッション ID は (\d+) のままである$")]
fn session_id_kept(w: &mut W, id: u32) {
    assert_eq!(w.dev.session().session_id, id);
    assert_eq!(w.dev.session().state, SessionState::Recording);
}

#[then(regex = r"^次に録音される音声フレームの位置は (\d+) である$")]
fn next_recorded_position(w: &mut W, pos: u64) {
    w.drain();
    let frames = w.ramp_frames(48);
    w.publish(frames);
    w.drain();
    assert!(w.session_data().iter().any(|d| d.0 == pos), "{:?}", w.session_data().iter().map(|d| d.0).collect::<Vec<_>>());
}

#[then(regex = r"^次の DATA の位置は (\d+) である$")]
fn next_data_position(w: &mut W, pos: u64) {
    w.drain();
    assert!(w.session_data().iter().any(|d| d.0 == pos), "{:?}", w.session_data().iter().map(|d| d.0).collect::<Vec<_>>());
}

#[then(regex = r"^セッション (\d+) の ENDED は通常どおり送られる$")]
fn ended_as_usual(w: &mut W, id: u32) {
    w.drain();
    assert_eq!(w.ended().map(|e| e.0), Some(id));
}

#[then(regex = r"^DATA の位置は(?: (\d+) から)?欠けずに続いている$")]
fn contiguous(w: &mut W, from: String) {
    let from = from.parse::<u64>().ok();
    w.drain();
    let data = w.session_data();
    assert!(!data.is_empty());
    if let Some(f) = from {
        assert_eq!(data[0].0, f);
    }
    for pair in data.windows(2) {
        assert_eq!(pair[1].0, pair[0].0 + pair[0].1.len() as u64);
    }
}

#[then(regex = r"^DATA の音声フレームを連結した列は、公開された (\d+) 個とバイト単位で一致する$")]
fn concatenation_matches(w: &mut W, n: usize) {
    let got: Vec<AudioFrame> = w.session_data().into_iter().flat_map(|d| d.1).collect();
    assert_eq!(got.len(), n);
    assert_eq!(got, w.session_published);
}

#[then(regex = r"^DATA は (\d+) 個に分かれて届く$")]
fn chunk_count(w: &mut W, n: usize) {
    assert_eq!(w.outs[w.mark..].iter().filter(|o| o.is_data()).count(), n);
}

#[then("どの DATA も 1〜256 個のフレームを持つ")]
fn chunk_sizes(w: &mut W) {
    assert!(w.session_data().iter().all(|d| (1..=256).contains(&d.1.len())));
}

#[then(regex = r"^位置 (\d+)〜(\d+) (?:がすべて届く|のフレームは元の値のまま届く)$")]
fn positions_delivered(w: &mut W, from: u64, to: u64) {
    w.drain();
    let mut by_pos = std::collections::HashMap::new();
    for (pos, frames) in w.session_data() {
        for (i, f) in frames.into_iter().enumerate() {
            by_pos.insert(pos + i as u64, f);
        }
    }
    for k in from..=to {
        assert_eq!(by_pos.get(&k), Some(&w.session_published[k as usize]), "position {k}");
    }
}

#[then(regex = r"^欠落は (\d+) である$")]
fn dropped_is(w: &mut W, n: u64) {
    assert_eq!(w.dev.session().dropped_frames, n);
}

#[then(regex = r"^ハーネスは欠落 (\d+) フレームを検出する$")]
fn harness_detects_gap(w: &mut W, n: u64) {
    w.drain();
    let data = w.session_data();
    let gaps: u64 = data.windows(2).map(|p| p[1].0 - (p[0].0 + p[0].1.len() as u64)).sum();
    assert_eq!(gaps, n);
}

#[then("INFO が DATA と DATA の間に 1 つ届く")]
fn info_between_data(w: &mut W) {
    let outs = w.session_outs();
    let infos: Vec<usize> = outs.iter().enumerate().filter(|(_, o)| matches!(o, Out::Info(_))).map(|(i, _)| i).collect();
    assert_eq!(infos.len(), 1);
    assert!(outs[..infos[0]].iter().any(Out::is_data) && outs[infos[0]..].iter().any(Out::is_data));
}

#[then(regex = r"^残りの (\d+) 個が DATA で届く$")]
fn remaining_delivered(w: &mut W, n: usize) {
    w.reading = true;
    w.drain();
    assert_eq!(w.frames_since_mark().len(), n);
}

#[then(regex = r"^(?:その後に )?ENDED\(session_id=(\d+), total_frames=(\d+), dropped_frames=(\d+)\) が届く$")]
fn ended_with(w: &mut W, id: u32, total: u64, dropped: u64) {
    w.drain();
    assert!(matches!(w.outs.last(), Some(Out::Ended { .. })), "ENDED must come after the DATA");
    assert_eq!(w.ended(), Some((id, total, dropped)));
}

#[then("録音セッションは Idle に戻る")]
fn back_to_idle(w: &mut W) {
    assert_eq!(w.dev.session().state, SessionState::Idle);
}

#[then(regex = r"^ENDED の total_frames は (\d+) である$")]
fn ended_total(w: &mut W, n: u64) {
    w.drain();
    assert_eq!(w.ended().map(|e| e.1), Some(n));
}

#[then(regex = r"^DATA のフレーム数の合計 \+ dropped_frames = total_frames である$")]
fn conservation(w: &mut W) {
    let delivered: u64 = w.session_data().iter().map(|d| d.1.len() as u64).sum();
    let (_, total, dropped) = w.ended().expect("ENDED");
    assert_eq!(delivered + dropped, total);
}

#[then(regex = r"^dropped_frames は (\d+) である$")]
fn ended_dropped(w: &mut W, n: u64) {
    assert_eq!(w.ended().map(|e| e.2), Some(n));
}

#[then("DATA は届かない")]
fn no_data(w: &mut W) {
    w.drain();
    assert!(w.session_data().is_empty());
}

#[then("ENDED はちょうど 1 回届く")]
fn ended_once(w: &mut W) {
    w.drain();
    assert_eq!(w.outs.iter().filter(|o| matches!(o, Out::Ended { .. })).count(), 1);
}

#[then(regex = r"^2 回目の STOP には ERROR\(NotRecording=2\) が返る$")]
fn second_stop_rejected(w: &mut W) {
    w.drain();
    assert!(w.outs[w.mark..].contains(&Out::Error(2)));
}

#[then(regex = r"^INFO には (\S+) が含まれる$")]
fn info_contains(w: &mut W, items: String) {
    w.drain();
    let info = w.last_infos(1).remove(0);
    for item in items.split('、') {
        match item.split_once('=') {
            Some((k, v)) => assert_eq!(info_line(&info, k), Some(v), "{k} in {info:?}"),
            None => assert!(info_line(&info, item).is_some(), "{item} in {info:?}"),
        }
    }
}

#[then(regex = r"^INFO には L と R の音量 (-?\d+) dB と、(L|R) のミュートが含まれる$")]
fn info_controls(w: &mut W, db: i32, ch: String) {
    w.drain();
    let info = w.last_infos(1).remove(0);
    let v = db * 256;
    assert_eq!(info_line(&info, "volume_8q8"), Some(format!("{v},{v}").as_str()));
    let mute = if ch == "L" { "1,0" } else { "0,1" };
    assert_eq!(info_line(&info, "mute"), Some(mute));
}

#[then("2 つの INFO の state と session は同じである")]
fn infos_agree(w: &mut W) {
    w.drain();
    let infos = w.last_infos(2);
    for key in ["state", "session"] {
        assert_eq!(info_line(&infos[0], key), info_line(&infos[1], key));
    }
}

#[then("録音セッションは Recording のままである")]
fn still_recording(w: &mut W) {
    assert_eq!(w.dev.session().state, SessionState::Recording);
}

fn main() {
    let features = concat!(env!("CARGO_MANIFEST_DIR"), "/../doc/context");
    futures::executor::block_on(
        W::cucumber()
            .fail_on_skipped()
            .before(|feature, _rule, _scenario, w| {
                // audio-in のフィーチャーでは、公開された音声フレームを観測するために裏で録音を走らせておく。
                let audio_in = feature.tags.iter().any(|t| t.starts_with("AUD-"));
                async move {
                    if audio_in {
                        w.dev.set_dtr(true);
                        w.dev.cdc_rx(b"START\n");
                        w.drain();
                        w.control_cursor = w.outs.len();
                    }
                }
                .boxed_local()
            })
            .filter_run_and_exit(features, |_feature, _rule, scenario| {
                !scenario.tags.iter().any(|t| t == "harness")
            }),
    );
}
