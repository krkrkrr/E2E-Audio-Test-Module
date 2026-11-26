//! capture-core::session の単体テスト。

use capture_core::audio_in::AudioFrame;
use capture_core::link::LinkError;
use capture_core::session::{CAPTURE_BUFFER_FRAMES, CaptureSession, Ended, SessionState};

fn frames(from: i16, n: usize) -> Vec<AudioFrame> {
    (0..n)
        .map(|i| AudioFrame {
            left: from + i as i16,
            right: 0,
        })
        .collect()
}

/// take の結果を (位置, フレーム列) にする。
fn take(s: &mut CaptureSession, max: usize) -> Option<(u64, Vec<AudioFrame>)> {
    let mut out = vec![0u8; max * 4];
    s.take(max, &mut out).map(|(pos, n)| {
        (
            pos,
            out[..n * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&c| AudioFrame::from_le_bytes(c))
                .collect(),
        )
    })
}

fn recording() -> Box<CaptureSession> {
    let mut s = Box::new(CaptureSession::new());
    s.start().unwrap();
    s
}

#[test]
fn a_new_session_is_idle_with_id_0() {
    let s = CaptureSession::new();
    assert_eq!(s.view().state, SessionState::Idle);
    assert_eq!(s.view().session_id, 0);
}

#[test]
fn start_records_and_ids_increase_by_one() {
    let mut s = CaptureSession::new();
    assert_eq!(s.start(), Ok(1));
    assert_eq!(s.view().state, SessionState::Recording);
    assert_eq!(s.start(), Err(LinkError::AlreadyRecording));
    s.abort();
    assert_eq!(s.start(), Ok(2));
}

#[test]
fn frames_accepted_while_idle_are_not_counted() {
    let mut s = CaptureSession::new();
    s.accept(frames(0, 48).into_iter());
    assert_eq!(s.view().next_position, 0);
    assert_eq!(s.view().buffered_frames, 0);
}

#[test]
fn frames_accepted_while_recording_are_taken_from_position_0() {
    let mut s = recording();
    s.accept(frames(10, 3).into_iter());
    assert_eq!(take(&mut s, 256), Some((0, frames(10, 3))));
    assert_eq!(take(&mut s, 256), None);
}

#[test]
fn take_returns_at_most_the_requested_number_of_frames() {
    let mut s = recording();
    s.accept(frames(0, 300).into_iter());
    assert_eq!(take(&mut s, 256).map(|t| (t.0, t.1.len())), Some((0, 256)));
    assert_eq!(take(&mut s, 256).map(|t| (t.0, t.1.len())), Some((256, 44)));
}

#[test]
fn a_full_buffer_drops_the_newest_and_still_advances_the_position() {
    let mut s = recording();
    s.accept(frames(0, CAPTURE_BUFFER_FRAMES + 5).into_iter());
    let v = s.view();
    assert_eq!(v.buffered_frames, CAPTURE_BUFFER_FRAMES);
    assert_eq!(v.dropped_frames, 5);
    assert_eq!(v.next_position, (CAPTURE_BUFFER_FRAMES + 5) as u64);
}

#[test]
fn frames_after_a_drop_are_taken_from_the_skipped_position() {
    let mut s = recording();
    s.accept(frames(0, CAPTURE_BUFFER_FRAMES + 5).into_iter());
    while s.view().buffered_frames > 0 {
        take(&mut s, 256);
    }
    s.accept(frames(7, 2).into_iter());
    assert_eq!(
        take(&mut s, 256),
        Some(((CAPTURE_BUFFER_FRAMES + 5) as u64, frames(7, 2)))
    );
}

#[test]
fn stop_moves_to_stopping_and_later_frames_are_ignored() {
    let mut s = recording();
    s.accept(frames(0, 4).into_iter());
    assert_eq!(s.stop(), Ok(()));
    assert_eq!(s.view().state, SessionState::Stopping);
    s.accept(frames(0, 4).into_iter());
    assert_eq!(s.view().next_position, 4);
    assert_eq!(s.stop(), Err(LinkError::NotRecording));
}

#[test]
fn stopping_ends_with_totals_once_the_buffer_is_empty() {
    let mut s = recording();
    s.accept(frames(0, 4).into_iter());
    s.stop().unwrap();
    assert_eq!(s.finish_if_drained(), None);
    take(&mut s, 256);
    assert_eq!(
        s.finish_if_drained(),
        Some(Ended {
            session_id: 1,
            total_frames: 4,
            dropped_frames: 0
        })
    );
    assert_eq!(s.view().state, SessionState::Idle);
    assert_eq!(s.finish_if_drained(), None);
}

#[test]
fn abort_returns_to_idle_and_discards_the_buffer() {
    let mut s = recording();
    s.accept(frames(0, 4).into_iter());
    s.abort();
    assert_eq!(s.view().state, SessionState::Idle);
    assert_eq!(s.view().buffered_frames, 0);
    assert_eq!(take(&mut s, 256), None);
    assert_eq!(s.finish_if_drained(), None);
}
