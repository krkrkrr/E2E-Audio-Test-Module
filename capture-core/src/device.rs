//! キャプチャデバイスの中核の状態機械。USB の入力を受け、CDC へ送るリンクフレームを作る。
//! ファームは USB の配線だけを持ち、振る舞いはすべてここにある（ADR-0005）。
//!
//! 送る順序: 応答（STARTED / ERROR / INFO）→ DATA → ENDED。
//! ENDED は STOP 以前のフレームを送り切ってから送る（CAP-R6）。

use core::fmt::Write;

use crate::audio_in::{AltSettingRejected, AudioStream, FORMAT, HostControls, StreamState};
use crate::link::{
    self, Command, DATA_POSITION_LEN, FrameKind, HEADER_LEN, LineReader, LinkError,
    MAX_FRAMES_PER_DATA, MAX_PAYLOAD_LEN,
};
use crate::session::CaptureSession;
pub use crate::session::{SessionState, SessionView};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 送信待ちの応答の上限。超えた応答は捨てる（1 回の CDC 受信は最大 64 B で、これを超えない）。
const MAX_PENDING: usize = 32;

#[derive(Clone, Copy, Debug)]
enum Pending {
    Started(u32),
    Error(LinkError),
    Info,
}

pub struct Device {
    audio: AudioStream,
    session: CaptureSession,
    lines: LineReader,
    dtr: bool,
    pending: [Pending; MAX_PENDING],
    pending_head: usize,
    pending_len: usize,
}

impl Device {
    pub const fn new() -> Self {
        Device {
            audio: AudioStream::new(),
            session: CaptureSession::new(),
            lines: LineReader::new(),
            dtr: false,
            // 埋め草は全ビット 0 の値にする。Device 全体が .bss に置かれ、起動時に 64 KiB をコピーせずに済む。
            pending: [Pending::Started(0); MAX_PENDING],
            pending_head: 0,
            pending_len: 0,
        }
    }

    pub fn audio(&self) -> &AudioStream {
        &self.audio
    }

    pub fn session(&self) -> SessionView {
        self.session.view()
    }

    // --- audio-in からの入力 ---

    pub fn set_alt_setting(&mut self, alt: u8) -> Result<(), AltSettingRejected> {
        self.audio.set_alt_setting(alt)
    }

    /// USB リセット。ストリームを閉じ、セッションを破棄する。
    pub fn usb_reset(&mut self) {
        self.audio.usb_reset();
        self.disconnect();
    }

    /// 音声パケット 1 つ（アイソクロナス OUT）。
    pub fn iso_out(&mut self, packet: &[u8]) {
        let frames = self.audio.receive(packet);
        self.session.accept(frames);
    }

    pub fn feedback(&self) -> Option<[u8; 3]> {
        self.audio.feedback()
    }

    pub fn set_host_controls(&mut self, c: HostControls) {
        self.audio.set_host_controls(c);
    }

    // --- capture-link からの入力 ---

    /// DTR の変化。下がったら（ハーネスがポートを閉じたら）セッションを破棄する。
    pub fn set_dtr(&mut self, on: bool) {
        if self.dtr && !on {
            self.disconnect();
        }
        self.dtr = on;
    }

    /// CDC で受けたバイト列。
    pub fn cdc_rx(&mut self, bytes: &[u8]) {
        for &b in bytes {
            match self.lines.push(b) {
                Some(Ok(Command::Start)) => match self.session.start() {
                    Ok(id) => self.respond(Pending::Started(id)),
                    Err(e) => self.respond(Pending::Error(e)),
                },
                Some(Ok(Command::Stop)) => {
                    if let Err(e) = self.session.stop() {
                        self.respond(Pending::Error(e));
                    }
                }
                Some(Ok(Command::Info)) => self.respond(Pending::Info),
                Some(Err(e)) => self.respond(Pending::Error(e)),
                None => {}
            }
        }
    }

    /// 次に送るリンクフレームを `out`（`link::MAX_FRAME_LEN` 以上）に書き、その長さを返す。無ければ 0。
    pub fn poll_tx(&mut self, out: &mut [u8]) -> usize {
        if let Some(p) = self.next_pending() {
            return self.encode_pending(p, out);
        }
        let frames_area = &mut out[HEADER_LEN + DATA_POSITION_LEN..HEADER_LEN + MAX_PAYLOAD_LEN];
        if let Some((position, n)) = self.session.take(MAX_FRAMES_PER_DATA, frames_area) {
            out[HEADER_LEN..HEADER_LEN + DATA_POSITION_LEN]
                .copy_from_slice(&position.to_le_bytes());
            return link::finish(FrameKind::Data, DATA_POSITION_LEN + n * 4, out);
        }
        if let Some(e) = self.session.finish_if_drained() {
            let mut payload = [0u8; 20];
            payload[..4].copy_from_slice(&e.session_id.to_le_bytes());
            payload[4..12].copy_from_slice(&e.total_frames.to_le_bytes());
            payload[12..].copy_from_slice(&e.dropped_frames.to_le_bytes());
            return link::encode(FrameKind::Ended, &payload, out);
        }
        0
    }

    fn disconnect(&mut self) {
        self.session.abort();
        self.lines.reset();
        self.pending_len = 0;
    }

    fn respond(&mut self, p: Pending) {
        if self.pending_len < MAX_PENDING {
            self.pending[(self.pending_head + self.pending_len) % MAX_PENDING] = p;
            self.pending_len += 1;
        }
    }

    fn next_pending(&mut self) -> Option<Pending> {
        if self.pending_len == 0 {
            return None;
        }
        let p = self.pending[self.pending_head];
        self.pending_head = (self.pending_head + 1) % MAX_PENDING;
        self.pending_len -= 1;
        Some(p)
    }

    fn encode_pending(&self, p: Pending, out: &mut [u8]) -> usize {
        match p {
            Pending::Started(id) => link::encode(FrameKind::Started, &id.to_le_bytes(), out),
            Pending::Error(e) => link::encode(FrameKind::Error, &[e as u8], out),
            Pending::Info => {
                let mut w = SliceWriter {
                    buf: &mut out[HEADER_LEN..HEADER_LEN + MAX_PAYLOAD_LEN],
                    len: 0,
                };
                let _ = self.write_info(&mut w);
                let n = w.len;
                link::finish(FrameKind::Info, n, out)
            }
        }
    }

    fn write_info(&self, w: &mut impl Write) -> core::fmt::Result {
        let s = self.session.view();
        let state = match s.state {
            SessionState::Idle => "idle",
            SessionState::Recording => "recording",
            SessionState::Stopping => "stopping",
        };
        let stream = match self.audio.state() {
            StreamState::Open => "open",
            StreamState::Closed => "closed",
        };
        let c = self.audio.host_controls();
        writeln!(w, "version={VERSION}")?;
        writeln!(
            w,
            "format={}/{}/{}",
            FORMAT.sample_rate_hz, FORMAT.bits_per_sample, FORMAT.channels
        )?;
        writeln!(w, "state={state}")?;
        writeln!(w, "stream={stream}")?;
        writeln!(w, "session={}", s.session_id)?;
        writeln!(w, "position={}", s.next_position)?;
        writeln!(w, "dropped={}", s.dropped_frames)?;
        writeln!(
            w,
            "volume_8q8={},{}",
            c.volume_8q8_db[0], c.volume_8q8_db[1]
        )?;
        writeln!(w, "mute={},{}", c.muted[0] as u8, c.muted[1] as u8)
    }
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Device {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Device")
            .field("session", &self.session.view())
            .finish()
    }
}

struct SliceWriter<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl Write for SliceWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len + s.len();
        if end > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
