//! capture-core::device の単体テスト。

#[path = "../support/link.rs"]
mod link_support;

use capture_core::audio_in::{AudioFrame, HostControls, StreamState};
use capture_core::device::{Device, SessionState};
use link_support::{Out, drain, packet_of, ramp};

fn connected() -> Box<Device> {
    let mut d = Box::new(Device::new());
    d.set_dtr(true);
    d.set_alt_setting(1).unwrap();
    d
}

#[test]
fn start_while_idle_sends_started_1() {
    let mut d = connected();
    d.cdc_rx(b"START\n");
    assert_eq!(drain(&mut d), vec![Out::Started(1)]);
}

#[test]
fn start_while_recording_and_stop_while_idle_are_rejected() {
    let mut d = connected();
    d.cdc_rx(b"STOP\nSTART\nSTART\n");
    assert_eq!(
        drain(&mut d),
        vec![Out::Error(2), Out::Started(1), Out::Error(1)]
    );
}

#[test]
fn responses_are_sent_before_data() {
    let mut d = connected();
    d.cdc_rx(b"START\n");
    d.iso_out(&packet_of(&ramp(0, 2)));
    d.cdc_rx(b"START\n");
    let outs = drain(&mut d);
    assert_eq!(outs[0], Out::Started(1));
    assert_eq!(outs[1], Out::Error(1));
    assert!(outs[2].is_data());
}

#[test]
fn audio_received_while_recording_is_sent_as_data() {
    let mut d = connected();
    d.cdc_rx(b"START\n");
    drain(&mut d);
    d.iso_out(&packet_of(&ramp(0, 48)));
    assert_eq!(
        drain(&mut d),
        vec![Out::Data {
            position: 0,
            frames: ramp(0, 48)
        }]
    );
}

#[test]
fn stop_sends_remaining_data_then_ended() {
    let mut d = connected();
    d.cdc_rx(b"START\n");
    d.iso_out(&packet_of(&ramp(0, 48)));
    d.cdc_rx(b"STOP\n");
    assert_eq!(d.session().state, SessionState::Stopping);
    assert_eq!(
        drain(&mut d),
        vec![
            Out::Started(1),
            Out::Data {
                position: 0,
                frames: ramp(0, 48)
            },
            Out::Ended {
                session_id: 1,
                total_frames: 48,
                dropped_frames: 0
            },
        ]
    );
    assert_eq!(d.session().state, SessionState::Idle);
}

#[test]
fn info_reports_state_format_position_drops_and_host_controls() {
    let mut d = connected();
    d.set_host_controls(HostControls {
        muted: [false, true],
        volume_8q8_db: [-5120, -5120],
    });
    d.cdc_rx(b"START\n");
    d.iso_out(&packet_of(&ramp(0, 3)));
    d.cdc_rx(b"INFO\n");
    let info = drain(&mut d)
        .into_iter()
        .find_map(|o| if let Out::Info(t) = o { Some(t) } else { None })
        .unwrap();
    for line in [
        "format=48000/16/2",
        "state=recording",
        "stream=open",
        "session=1",
        "position=3",
        "dropped=0",
        "volume_8q8=-5120,-5120",
        "mute=0,1",
    ] {
        assert!(info.lines().any(|l| l == line), "{line} not in {info:?}");
    }
    assert!(info.lines().any(|l| l.starts_with("version=")));
}

#[test]
fn dtr_low_aborts_the_session_and_discards_pending_output() {
    let mut d = connected();
    d.cdc_rx(b"START\n");
    d.iso_out(&packet_of(&ramp(0, 48)));
    d.set_dtr(false);
    assert_eq!(d.session().state, SessionState::Idle);
    d.set_dtr(true);
    assert_eq!(drain(&mut d), vec![]);
}

#[test]
fn usb_reset_closes_the_stream_and_aborts_the_session() {
    let mut d = connected();
    d.cdc_rx(b"START\n");
    d.iso_out(&packet_of(&[AudioFrame::default()]));
    d.usb_reset();
    assert_eq!(d.audio().state(), StreamState::Closed);
    assert_eq!(d.session().state, SessionState::Idle);
    assert_eq!(drain(&mut d), vec![]);
}
