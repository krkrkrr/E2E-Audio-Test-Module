//! audio-in のプロパティ（doc/context/audio-in/constraints.md の P-AUD-1〜6）。

use capture_core::audio_in::{AudioFrame, AudioStream, FEEDBACK_48K, HostControls, StreamState};
use proptest::prelude::*;

fn open_stream() -> AudioStream {
    let mut s = AudioStream::new();
    s.set_alt_setting(1).expect("alt 1 exists");
    s
}

fn host_controls() -> impl Strategy<Value = HostControls> {
    (any::<[bool; 2]>(), any::<[i16; 2]>()).prop_map(|(muted, volume_8q8_db)| HostControls {
        muted,
        volume_8q8_db,
    })
}

/// ストリームに対する操作。USB からの入力そのもの。
#[derive(Debug, Clone)]
enum Op {
    Alt(u8),
    Reset,
    Packet(Vec<u8>),
    Controls(HostControls),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u8..=2).prop_map(Op::Alt),
        Just(Op::Reset),
        prop::collection::vec(any::<u8>(), 0..=392).prop_map(Op::Packet),
        host_controls().prop_map(Op::Controls),
    ]
}

fn apply(s: &mut AudioStream, op: &Op) -> Vec<AudioFrame> {
    match op {
        Op::Alt(a) => {
            let _ = s.set_alt_setting(*a);
            vec![]
        }
        Op::Reset => {
            s.usb_reset();
            vec![]
        }
        Op::Packet(p) => s.receive(p).collect(),
        Op::Controls(c) => {
            s.set_host_controls(*c);
            vec![]
        }
    }
}

proptest! {
    /// P-AUD-1: フレーム化はバイト列を保存する。
    #[test]
    fn p_aud_1_framing_preserves_bytes(packet in prop::collection::vec(any::<u8>(), 0..=392)) {
        let mut s = open_stream();
        let bytes: Vec<u8> = s.receive(&packet).flat_map(|f| f.to_le_bytes()).collect();
        prop_assert_eq!(&bytes[..], &packet[..packet.len() - packet.len() % 4]);
    }

    /// P-AUD-2: 受けたフレーム数と端数パケット数は、パケット長どおりに増える。
    #[test]
    fn p_aud_2_counters_follow_packet_lengths(
        packets in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..=392), 0..20)
    ) {
        let mut s = open_stream();
        for p in &packets {
            let (frames0, partial0) = (s.frames_received(), s.partial_packets());
            let n = s.receive(p).count();
            prop_assert_eq!(n, p.len() / 4);
            prop_assert_eq!(s.frames_received() - frames0, (p.len() / 4) as u64);
            prop_assert_eq!(s.partial_packets() - partial0, (p.len() % 4 != 0) as u32);
        }
    }

    /// P-AUD-3: ホスト制御値はサンプルに作用しない。
    #[test]
    fn p_aud_3_host_controls_never_touch_samples(
        controls in host_controls(),
        packet in prop::collection::vec(any::<u8>(), 0..=392),
    ) {
        let mut plain = open_stream();
        let mut controlled = open_stream();
        controlled.set_host_controls(controls);
        let a: Vec<AudioFrame> = plain.receive(&packet).collect();
        let b: Vec<AudioFrame> = controlled.receive(&packet).collect();
        prop_assert_eq!(a, b);
        prop_assert_eq!(controlled.host_controls(), controls);
    }

    /// P-AUD-4: 閉じている間は何も公開しない。
    #[test]
    fn p_aud_4_closed_stream_publishes_nothing(ops in prop::collection::vec(op(), 0..40)) {
        let mut s = AudioStream::new();
        for op in &ops {
            let was_closed = s.state() == StreamState::Closed;
            let before = s.frames_received();
            let frames = apply(&mut s, op);
            if was_closed && matches!(op, Op::Packet(_)) {
                prop_assert!(frames.is_empty());
                prop_assert_eq!(s.frames_received(), before);
            }
        }
    }

    /// P-AUD-5: 代替設定の適用は冪等。2 回目は状態も数も変えない。
    #[test]
    fn p_aud_5_alt_setting_is_idempotent(ops in prop::collection::vec(op(), 0..40), alt in 0u8..=2) {
        let mut s = AudioStream::new();
        for op in &ops {
            apply(&mut s, op);
        }
        let first = s.set_alt_setting(alt).is_ok();
        let (state, frames) = (s.state(), s.frames_received());
        let second = s.set_alt_setting(alt).is_ok();
        prop_assert_eq!(first, second);
        prop_assert_eq!(s.state(), state);
        prop_assert_eq!(s.frames_received(), frames);
    }

    /// P-AUD-6: フィードバックは閉じていれば無く、開いていれば常に 48.0（00 00 0C）。
    /// 状態は代替設定とリセットだけから決まる（モデルとの比較）。
    #[test]
    fn p_aud_6_feedback_follows_stream_state(ops in prop::collection::vec(op(), 0..60)) {
        let mut s = AudioStream::new();
        let mut model_open = false;
        for op in &ops {
            apply(&mut s, op);
            match op {
                Op::Alt(0) | Op::Reset => model_open = false,
                Op::Alt(1) => model_open = true,
                _ => {}
            }
            prop_assert_eq!(s.state() == StreamState::Open, model_open);
            let expected = if model_open { Some(FEEDBACK_48K) } else { None };
            prop_assert_eq!(s.feedback(), expected);
        }
    }
}

#[test]
fn feedback_constant_is_48_khz_in_10_14_format() {
    assert_eq!(FEEDBACK_48K, [0x00, 0x00, 0x0C]);
    let q10_14 = u32::from_le_bytes([FEEDBACK_48K[0], FEEDBACK_48K[1], FEEDBACK_48K[2], 0]);
    assert_eq!(q10_14, 48 << 14);
}
