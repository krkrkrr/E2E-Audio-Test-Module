//! capture-core::audio_in の単体テスト。

use capture_core::audio_in::AudioFrame;

#[test]
fn audio_frame_is_left_then_right_as_i16_le_and_round_trips() {
    // Arrange
    let frame = AudioFrame {
        left: 0x0102,
        right: -2,
    };
    // Act
    let bytes = frame.to_le_bytes();
    // Assert
    assert_eq!(bytes, [0x02, 0x01, 0xFE, 0xFF]);
    assert_eq!(AudioFrame::from_le_bytes(bytes), frame);
}

use capture_core::audio_in::{AudioStream, StreamState};

#[test]
fn new_stream_is_closed_and_alt_1_opens_and_alt_0_closes() {
    let mut s = AudioStream::new();
    assert_eq!(s.state(), StreamState::Closed);
    s.set_alt_setting(1).unwrap();
    assert_eq!(s.state(), StreamState::Open);
    s.set_alt_setting(0).unwrap();
    assert_eq!(s.state(), StreamState::Closed);
}

#[test]
fn alt_setting_2_or_more_is_rejected_without_changing_state() {
    let mut s = AudioStream::new();
    s.set_alt_setting(1).unwrap();
    assert!(s.set_alt_setting(2).is_err());
    assert_eq!(s.state(), StreamState::Open);
}

fn open_stream() -> AudioStream {
    let mut s = AudioStream::new();
    s.set_alt_setting(1).unwrap();
    s
}

#[test]
fn a_192_byte_packet_becomes_48_frames_and_counts_them() {
    let mut s = open_stream();
    let packet: Vec<u8> = (0..192).map(|i| i as u8).collect();
    let frames: Vec<AudioFrame> = s.receive(&packet).collect();
    assert_eq!(frames.len(), 48);
    assert_eq!(frames[1], AudioFrame::from_le_bytes([4, 5, 6, 7]));
    assert_eq!(s.frames_received(), 48);
    assert_eq!(s.partial_packets(), 0);
}

#[test]
fn trailing_bytes_are_discarded_and_counted_as_a_partial_packet() {
    let mut s = open_stream();
    let frames: Vec<AudioFrame> = s.receive(&[1, 0, 2, 0, 9, 9]).collect();
    assert_eq!(frames, vec![AudioFrame { left: 1, right: 2 }]);
    assert_eq!(s.frames_received(), 1);
    assert_eq!(s.partial_packets(), 1);
}

#[test]
fn a_closed_stream_publishes_nothing() {
    let mut s = AudioStream::new();
    assert_eq!(s.receive(&[0; 192]).count(), 0);
    assert_eq!(s.frames_received(), 0);
    assert_eq!(s.partial_packets(), 0);
}

use capture_core::audio_in::{FEEDBACK_48K, HostControls};

#[test]
fn feedback_is_none_when_closed_and_48_khz_when_open() {
    let mut s = AudioStream::new();
    assert_eq!(s.feedback(), None);
    s.set_alt_setting(1).unwrap();
    assert_eq!(s.feedback(), Some([0x00, 0x00, 0x0C]));
    assert_eq!(FEEDBACK_48K, [0x00, 0x00, 0x0C]);
}

#[test]
fn host_controls_are_recorded() {
    let mut s = AudioStream::new();
    assert_eq!(s.host_controls(), HostControls::default());
    let c = HostControls {
        muted: [true, false],
        volume_8q8_db: [-5120, 256],
    };
    s.set_host_controls(c);
    assert_eq!(s.host_controls(), c);
}

#[test]
fn usb_reset_closes_the_stream() {
    let mut s = open_stream();
    s.usb_reset();
    assert_eq!(s.state(), StreamState::Closed);
}
