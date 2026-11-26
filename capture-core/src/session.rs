//! capture-link: 録音セッション集約とキャプチャバッファ。
//! 設計: doc/context/capture-link/domain-model.md

use crate::audio_in::AudioFrame;
use crate::link::LinkError;

pub const CAPTURE_BUFFER_FRAMES: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Recording,
    Stopping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionView {
    pub state: SessionState,
    pub session_id: u32,
    pub next_position: u64,
    pub dropped_frames: u64,
    pub buffered_frames: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ended {
    pub session_id: u32,
    pub total_frames: u64,
    pub dropped_frames: u64,
}

/// 位置が連続するフレームの区間。欠落で位置が飛ぶたびに新しい区間になる。
#[derive(Clone, Copy, Debug, Default)]
struct Segment {
    start: u64,
    len: usize,
}

/// 同時に持てる区間の数。使い切ったら、新しい区間を作るフレームは欠落として数える。
const MAX_SEGMENTS: usize = 64;

/// 録音セッション集約。キャプチャバッファを含む。
pub struct CaptureSession {
    state: SessionState,
    session_id: u32,
    next_position: u64,
    dropped_frames: u64,
    frames: [AudioFrame; CAPTURE_BUFFER_FRAMES],
    head: usize,
    len: usize,
    segments: [Segment; MAX_SEGMENTS],
    seg_head: usize,
    seg_len: usize,
}

impl CaptureSession {
    pub const fn new() -> Self {
        CaptureSession {
            state: SessionState::Idle,
            session_id: 0,
            next_position: 0,
            dropped_frames: 0,
            frames: [AudioFrame { left: 0, right: 0 }; CAPTURE_BUFFER_FRAMES],
            head: 0,
            len: 0,
            segments: [Segment { start: 0, len: 0 }; MAX_SEGMENTS],
            seg_head: 0,
            seg_len: 0,
        }
    }

    pub fn view(&self) -> SessionView {
        SessionView {
            state: self.state,
            session_id: self.session_id,
            next_position: self.next_position,
            dropped_frames: self.dropped_frames,
            buffered_frames: self.len,
        }
    }

    /// Idle なら新しいセッションを始め、その id を返す。
    pub fn start(&mut self) -> Result<u32, LinkError> {
        if self.state != SessionState::Idle {
            return Err(LinkError::AlreadyRecording);
        }
        self.clear();
        self.session_id = self.session_id.wrapping_add(1);
        self.next_position = 0;
        self.dropped_frames = 0;
        self.state = SessionState::Recording;
        Ok(self.session_id)
    }

    /// Recording なら Stopping に移る。以後のフレームは受けない（CAP-R6）。
    pub fn stop(&mut self) -> Result<(), LinkError> {
        if self.state != SessionState::Recording {
            return Err(LinkError::NotRecording);
        }
        self.state = SessionState::Stopping;
        Ok(())
    }

    /// audio-in が公開した音声フレームを受ける。満杯なら新しい側を捨てる（CAP-R5）。
    pub fn accept(&mut self, frames: impl Iterator<Item = AudioFrame>) {
        if self.state != SessionState::Recording {
            return;
        }
        for frame in frames {
            let position = self.next_position;
            self.next_position += 1;
            if self.len == CAPTURE_BUFFER_FRAMES || !self.extend_segment(position) {
                self.dropped_frames += 1;
                continue;
            }
            self.frames[(self.head + self.len) % CAPTURE_BUFFER_FRAMES] = frame;
            self.len += 1;
        }
    }

    /// 先頭の区間から最大 `max` 個のフレームを取り出し、LE のバイト列として `out` に書く。
    /// 先頭の位置と、書いたフレーム数を返す。欠落をまたがない。
    pub fn take(&mut self, max: usize, out: &mut [u8]) -> Option<(u64, usize)> {
        if self.seg_len == 0 {
            return None;
        }
        let seg = &mut self.segments[self.seg_head];
        let n = max.min(seg.len).min(out.len() / 4);
        let position = seg.start;
        for (i, chunk) in out[..n * 4].as_chunks_mut::<4>().0.iter_mut().enumerate() {
            chunk.copy_from_slice(
                &self.frames[(self.head + i) % CAPTURE_BUFFER_FRAMES].to_le_bytes(),
            );
        }
        self.head = (self.head + n) % CAPTURE_BUFFER_FRAMES;
        self.len -= n;
        seg.start += n as u64;
        seg.len -= n;
        if seg.len == 0 {
            self.seg_head = (self.seg_head + 1) % MAX_SEGMENTS;
            self.seg_len -= 1;
        }
        Some((position, n))
    }

    /// Stopping でバッファが空なら、セッションを終えて ENDED の値を返す。
    pub fn finish_if_drained(&mut self) -> Option<Ended> {
        if self.state != SessionState::Stopping || self.len != 0 {
            return None;
        }
        self.state = SessionState::Idle;
        Some(Ended {
            session_id: self.session_id,
            total_frames: self.next_position,
            dropped_frames: self.dropped_frames,
        })
    }

    /// CDC の切断や USB リセットで、何も送らずにセッションを捨てる。
    pub fn abort(&mut self) {
        self.state = SessionState::Idle;
        self.clear();
    }

    fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
        self.seg_head = 0;
        self.seg_len = 0;
    }

    /// `position` のフレームを最後の区間に足す。続かなければ新しい区間を作る。作れなければ false。
    fn extend_segment(&mut self, position: u64) -> bool {
        if self.seg_len > 0 {
            let last = &mut self.segments[(self.seg_head + self.seg_len - 1) % MAX_SEGMENTS];
            if last.start + last.len as u64 == position {
                last.len += 1;
                return true;
            }
        }
        if self.seg_len == MAX_SEGMENTS {
            return false;
        }
        self.segments[(self.seg_head + self.seg_len) % MAX_SEGMENTS] = Segment {
            start: position,
            len: 1,
        };
        self.seg_len += 1;
        true
    }
}

impl Default for CaptureSession {
    fn default() -> Self {
        Self::new()
    }
}
