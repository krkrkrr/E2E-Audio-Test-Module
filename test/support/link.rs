//! テスト専用: リンクフレームの厳密な復号器と、Device を駆動する小さな道具。
//!
//! 製品の復号器はハーネス側（Python）にある。これはデバイスの出力を検査するためだけのもので、
//! 同期のずれや CRC 不一致を許さない（デバイスは壊れたフレームを出してはならないから）。
#![allow(dead_code)]

use capture_core::audio_in::AudioFrame;
use capture_core::device::Device;
use capture_core::link::MAX_FRAME_LEN;

/// デバイスがハーネスへ送ったリンクフレーム 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Started(u32),
    Data {
        position: u64,
        frames: Vec<AudioFrame>,
    },
    Ended {
        session_id: u32,
        total_frames: u64,
        dropped_frames: u64,
    },
    Info(String),
    Error(u8),
}

impl Out {
    pub fn is_data(&self) -> bool {
        matches!(self, Out::Data { .. })
    }
}

/// CRC-16/CCITT-FALSE の参照実装（ビットごと）。製品の実装とは独立に書く。
pub fn crc16_reference(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in bytes {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// 連続したリンクフレームのバイト列を復号する。形式違反は panic する。
pub fn decode_all(mut bytes: &[u8]) -> Vec<Out> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        assert!(bytes.len() >= 7, "truncated frame: {bytes:02x?}");
        assert_eq!(&bytes[..2], &[0xA5, 0x5A], "bad sync");
        let kind = bytes[2];
        let len = u16::from_le_bytes([bytes[3], bytes[4]]) as usize;
        let end = 5 + len;
        assert!(bytes.len() >= end + 2, "truncated payload");
        let payload = &bytes[5..end];
        let crc = u16::from_le_bytes([bytes[end], bytes[end + 1]]);
        assert_eq!(crc, crc16_reference(&bytes[2..end]), "crc mismatch");
        out.push(parse(kind, payload));
        bytes = &bytes[end + 2..];
    }
    out
}

fn parse(kind: u8, p: &[u8]) -> Out {
    let u32_at = |i: usize| u32::from_le_bytes(p[i..i + 4].try_into().unwrap());
    let u64_at = |i: usize| u64::from_le_bytes(p[i..i + 8].try_into().unwrap());
    match kind {
        0x01 => {
            assert_eq!(p.len(), 4);
            Out::Started(u32_at(0))
        }
        0x02 => {
            assert!(
                p.len() >= 8 && (p.len() - 8).is_multiple_of(4),
                "bad DATA length {}",
                p.len()
            );
            let frames = p[8..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&c| AudioFrame::from_le_bytes(c))
                .collect();
            Out::Data {
                position: u64_at(0),
                frames,
            }
        }
        0x03 => {
            assert_eq!(p.len(), 20);
            Out::Ended {
                session_id: u32_at(0),
                total_frames: u64_at(4),
                dropped_frames: u64_at(12),
            }
        }
        0x04 => Out::Info(String::from_utf8(p.to_vec()).expect("INFO is UTF-8")),
        0x7F => {
            assert_eq!(p.len(), 1);
            Out::Error(p[0])
        }
        k => panic!("unknown kind {k:#x}"),
    }
}

/// ハーネスが CDC を読み切る。デバイスが出すものが無くなるまで poll する。
pub fn drain(dev: &mut Device) -> Vec<Out> {
    let mut buf = [0u8; MAX_FRAME_LEN];
    let mut bytes = Vec::new();
    loop {
        let n = dev.poll_tx(&mut buf);
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    decode_all(&bytes)
}

/// 音声フレーム列をパケットのバイト列にする。
pub fn packet_of(frames: &[AudioFrame]) -> Vec<u8> {
    frames.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// 音声フレームを 1 ms ぶん（48 個）ずつパケットにして届ける。
pub fn publish(dev: &mut Device, frames: &[AudioFrame]) {
    for chunk in frames.chunks(48) {
        dev.iso_out(&packet_of(chunk));
    }
}

/// 位置 k の値が分かる決定的なフレーム列。
pub fn ramp(from: u64, n: usize) -> Vec<AudioFrame> {
    (0..n as u64)
        .map(|i| {
            let k = from + i;
            AudioFrame {
                left: k as i16,
                right: !(k as i16),
            }
        })
        .collect()
}
