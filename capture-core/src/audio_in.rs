//! audio-in: USB Audio 1.0 で受けたパケットを音声フレームとして公開する。
//! 設計: doc/context/audio-in/domain-model.md

/// 音声フレーム 1 個のバイト数（16 bit × 2 ch）。
pub const BYTES_PER_FRAME: usize = 4;

/// 全チャンネル分のサンプルの組（L, R）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioFrame {
    pub left: i16,
    pub right: i16,
}

impl AudioFrame {
    pub fn to_le_bytes(self) -> [u8; 4] {
        let [l0, l1] = self.left.to_le_bytes();
        let [r0, r1] = self.right.to_le_bytes();
        [l0, l1, r0, r1]
    }

    pub fn from_le_bytes(b: [u8; 4]) -> Self {
        AudioFrame {
            left: i16::from_le_bytes([b[0], b[1]]),
            right: i16::from_le_bytes([b[2], b[3]]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamState {
    Closed,
    Open,
}

/// ストリーミングインタフェースに存在しない代替設定が選ばれた。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AltSettingRejected(pub u8);

/// 音声ストリーム集約。
#[derive(Debug)]
pub struct AudioStream {
    state: StreamState,
    frames_received: u64,
    partial_packets: u32,
    controls: HostControls,
}

impl AudioStream {
    pub const fn new() -> Self {
        AudioStream {
            state: StreamState::Closed,
            frames_received: 0,
            partial_packets: 0,
            controls: HostControls {
                muted: [false; 2],
                volume_8q8_db: [0; 2],
            },
        }
    }

    pub fn state(&self) -> StreamState {
        self.state
    }

    /// ホストが代替設定を選ぶ。0 = 閉、1 = 開。それ以外は存在しない。
    pub fn set_alt_setting(&mut self, alt: u8) -> Result<(), AltSettingRejected> {
        self.state = match alt {
            0 => StreamState::Closed,
            1 => StreamState::Open,
            other => return Err(AltSettingRejected(other)),
        };
        Ok(())
    }
}

/// 1 パケット分の音声フレームを先頭から順に返す。
pub struct Frames<'a> {
    chunks: core::slice::Iter<'a, [u8; BYTES_PER_FRAME]>,
}

impl Iterator for Frames<'_> {
    type Item = AudioFrame;

    fn next(&mut self) -> Option<AudioFrame> {
        self.chunks.next().map(|&c| AudioFrame::from_le_bytes(c))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.size_hint()
    }
}

impl ExactSizeIterator for Frames<'_> {}

impl Frames<'_> {
    fn empty() -> Frames<'static> {
        Frames { chunks: [].iter() }
    }
}

impl AudioStream {
    /// 音声パケットを受け、4 B ずつ音声フレームにして返す（AUD-R3: 値は加工しない）。
    /// 端数バイトは捨てて数える（AUD-R5）。閉じている間は何も返さない。
    pub fn receive<'a>(&mut self, packet: &'a [u8]) -> Frames<'a> {
        if self.state == StreamState::Closed {
            return Frames::empty();
        }
        let (frames, trailing) = packet.as_chunks::<BYTES_PER_FRAME>();
        self.frames_received += frames.len() as u64;
        if !trailing.is_empty() {
            self.partial_packets += 1;
        }
        Frames {
            chunks: frames.iter(),
        }
    }

    pub fn frames_received(&self) -> u64 {
        self.frames_received
    }

    pub fn partial_packets(&self) -> u32 {
        self.partial_packets
    }
}

/// 48.0 kHz を 10.14 固定小数点（kHz）で表した 3 バイト（LE）。ADR-0003 により常にこの値を返す。
pub const FEEDBACK_48K: [u8; 3] = {
    let q10_14 = (FORMAT.sample_rate_hz / 1000) << 14;
    let b = q10_14.to_le_bytes();
    [b[0], b[1], b[2]]
};

/// 公開する唯一のフォーマット（AUD-R2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate_hz: u32,
    pub bits_per_sample: u8,
    pub channels: u8,
}

pub const FORMAT: AudioFormat = AudioFormat {
    sample_rate_hz: 48_000,
    bits_per_sample: 16,
    channels: 2,
};

/// ホストが設定する音量とミュート。記録するが適用しない（ADR-0003）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostControls {
    pub muted: [bool; 2],
    /// 1/256 dB 単位
    pub volume_8q8_db: [i16; 2],
}

impl AudioStream {
    /// フィードバック値。閉じている間はエンドポイントが無効なので返さない（AUD-R4）。
    pub fn feedback(&self) -> Option<[u8; 3]> {
        (self.state == StreamState::Open).then_some(FEEDBACK_48K)
    }

    pub fn host_controls(&self) -> HostControls {
        self.controls
    }

    /// ホストが設定した音量とミュートを記録する。サンプルには作用しない（AUD-R3）。
    pub fn set_host_controls(&mut self, c: HostControls) {
        self.controls = c;
    }

    pub fn usb_reset(&mut self) {
        self.state = StreamState::Closed;
    }
}

impl Default for AudioStream {
    fn default() -> Self {
        Self::new()
    }
}
