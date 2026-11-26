//! capture-link のワイヤ形式（ADR-0004）。
//! 設計: doc/context/capture-link/domain-model.md
//!
//! デバイス → ハーネス: `A5 5A | 種別 u8 | 長さ u16 LE | ペイロード | CRC-16/CCITT-FALSE u16 LE`
//! ハーネス → デバイス: `\n` で終わる ASCII の行（`START` / `STOP` / `INFO`）

use crate::audio_in::BYTES_PER_FRAME;

pub const SYNC: [u8; 2] = [0xA5, 0x5A];
/// 同期語、種別、長さ
pub const HEADER_LEN: usize = 5;
pub const CRC_LEN: usize = 2;
/// 1 つの DATA が運ぶ音声フレームの上限
pub const MAX_FRAMES_PER_DATA: usize = 256;
/// DATA のペイロードの先頭にある位置（u64）
pub const DATA_POSITION_LEN: usize = 8;
pub const MAX_PAYLOAD_LEN: usize = DATA_POSITION_LEN + MAX_FRAMES_PER_DATA * BYTES_PER_FRAME;
/// リンクフレーム 1 つの最大長
pub const MAX_FRAME_LEN: usize = HEADER_LEN + MAX_PAYLOAD_LEN + CRC_LEN;
/// コマンド行の最大長（終端の `\n` を除く）
pub const MAX_LINE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    Started = 0x01,
    Data = 0x02,
    Ended = 0x03,
    Info = 0x04,
    Error = 0x7F,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LinkError {
    AlreadyRecording = 1,
    NotRecording = 2,
    UnknownCommand = 3,
    LineTooLong = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    Info,
}

const CRC_TABLE: [u16; 256] = {
    let mut table = [0u16; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// CRC-16/CCITT-FALSE（多項式 0x1021、初期値 0xFFFF、反転なし）。
pub fn crc16(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0xFFFF, |crc, &b| {
        (crc << 8) ^ CRC_TABLE[((crc >> 8) as u8 ^ b) as usize]
    })
}

/// `out[HEADER_LEN..HEADER_LEN + payload_len]` に置いたペイロードに、見出しと CRC を付ける。
/// 書いたリンクフレームの長さを返す。
pub fn finish(kind: FrameKind, payload_len: usize, out: &mut [u8]) -> usize {
    assert!(payload_len <= MAX_PAYLOAD_LEN, "payload too long");
    out[..2].copy_from_slice(&SYNC);
    out[2] = kind as u8;
    out[3..5].copy_from_slice(&(payload_len as u16).to_le_bytes());
    let end = HEADER_LEN + payload_len;
    let crc = crc16(&out[2..end]);
    out[end..end + CRC_LEN].copy_from_slice(&crc.to_le_bytes());
    end + CRC_LEN
}

/// リンクフレームを `out` に書き、その長さを返す。
pub fn encode(kind: FrameKind, payload: &[u8], out: &mut [u8]) -> usize {
    out[HEADER_LEN..HEADER_LEN + payload.len()].copy_from_slice(payload);
    finish(kind, payload.len(), out)
}

/// ハーネスから届くバイト列を行に区切り、コマンドとして解釈する。
#[derive(Debug)]
pub struct LineReader {
    buf: [u8; MAX_LINE],
    len: usize,
    too_long: bool,
}

impl LineReader {
    pub const fn new() -> Self {
        LineReader {
            buf: [0; MAX_LINE],
            len: 0,
            too_long: false,
        }
    }

    /// 1 バイト受ける。行が終われば、そのコマンドか拒否の理由を返す。
    /// `\r` は無視し、空行は何も返さない。
    pub fn push(&mut self, byte: u8) -> Option<Result<Command, LinkError>> {
        match byte {
            b'\r' => None,
            b'\n' => {
                let result = if self.too_long {
                    Some(Err(LinkError::LineTooLong))
                } else if self.len == 0 {
                    None
                } else {
                    Some(parse(&self.buf[..self.len]))
                };
                self.reset();
                result
            }
            _ if self.len < MAX_LINE => {
                self.buf[self.len] = byte;
                self.len += 1;
                None
            }
            _ => {
                self.too_long = true;
                None
            }
        }
    }

    pub fn reset(&mut self) {
        self.len = 0;
        self.too_long = false;
    }
}

impl Default for LineReader {
    fn default() -> Self {
        Self::new()
    }
}

fn parse(line: &[u8]) -> Result<Command, LinkError> {
    match line {
        b"START" => Ok(Command::Start),
        b"STOP" => Ok(Command::Stop),
        b"INFO" => Ok(Command::Info),
        _ => Err(LinkError::UnknownCommand),
    }
}
