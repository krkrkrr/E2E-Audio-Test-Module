"""capture-link のワイヤ形式（ADR-0004）の、ハーネス側の符号化と復号。

デバイス → ハーネス: ``A5 5A | 種別 u8 | 長さ u16 LE | ペイロード | CRC-16/CCITT-FALSE u16 LE``
設計: doc/context/capture-link/domain-model.md
"""

from __future__ import annotations

import binascii
import struct
from dataclasses import dataclass, field

SYNC = b"\xa5\x5a"
HEADER_LEN = 5
CRC_LEN = 2
BYTES_PER_FRAME = 4
MAX_FRAMES_PER_DATA = 256
MAX_PAYLOAD_LEN = 8 + MAX_FRAMES_PER_DATA * BYTES_PER_FRAME

STARTED, DATA, ENDED, INFO, ERROR = 0x01, 0x02, 0x03, 0x04, 0x7F


@dataclass(frozen=True)
class Started:
    session_id: int


@dataclass(frozen=True)
class Data:
    position: int
    frames: bytes

    @property
    def n_frames(self) -> int:
        return len(self.frames) // BYTES_PER_FRAME


@dataclass(frozen=True)
class Ended:
    session_id: int
    total_frames: int
    dropped_frames: int


@dataclass(frozen=True)
class Info:
    text: str

    def fields(self) -> dict[str, str]:
        return dict(line.split("=", 1) for line in self.text.splitlines() if "=" in line)


@dataclass(frozen=True)
class Error:
    code: int


LinkFrame = Started | Data | Ended | Info | Error

ERROR_NAMES = {1: "AlreadyRecording", 2: "NotRecording", 3: "UnknownCommand", 4: "LineTooLong"}


def crc16(data: bytes) -> int:
    """CRC-16/CCITT-FALSE（多項式 0x1021、初期値 0xFFFF）。"""
    return binascii.crc_hqx(data, 0xFFFF)


def _payload(frame: LinkFrame) -> tuple[int, bytes]:
    match frame:
        case Started(session_id):
            return STARTED, struct.pack("<I", session_id)
        case Data(position, frames):
            return DATA, struct.pack("<Q", position) + frames
        case Ended(session_id, total, dropped):
            return ENDED, struct.pack("<IQQ", session_id, total, dropped)
        case Info(text):
            return INFO, text.encode("utf-8")
        case Error(code):
            return ERROR, bytes([code])
    raise TypeError(frame)


def encode(frame: LinkFrame) -> bytes:
    kind, payload = _payload(frame)
    body = bytes([kind]) + struct.pack("<H", len(payload)) + payload
    return SYNC + body + struct.pack("<H", crc16(body))


def _plausible(kind: int, length: int) -> bool:
    """種別と長さの組が、ADR-0004 の形式としてありうるか。"""
    match kind:
        case 0x01:
            return length == 4
        case 0x02:
            return 12 <= length <= MAX_PAYLOAD_LEN and (length - 8) % BYTES_PER_FRAME == 0
        case 0x03:
            return length == 20
        case 0x04:
            return length <= MAX_PAYLOAD_LEN
        case 0x7F:
            return length == 1
    return False


def _parse(kind: int, p: bytes) -> LinkFrame:
    match kind:
        case 0x01:
            return Started(*struct.unpack("<I", p))
        case 0x02:
            return Data(struct.unpack_from("<Q", p)[0], bytes(p[8:]))
        case 0x03:
            return Ended(*struct.unpack("<IQQ", p))
        case 0x04:
            return Info(p.decode("utf-8", errors="replace"))
        case _:
            return Error(p[0])


class Decoder:
    """バイト列の任意の位置から、同期語と CRC でリンクフレームを取り出す。

    壊れたフレームは捨てて ``crc_errors`` に数える。読み飛ばしたバイトは ``skipped_bytes`` に数える。
    """

    def __init__(self) -> None:
        self._buf = bytearray()
        self.crc_errors = 0
        self.skipped_bytes = 0

    def feed(self, data: bytes) -> list[LinkFrame]:
        self._buf += data
        out: list[LinkFrame] = []
        while True:
            if not self._seek_sync():
                break
            if len(self._buf) < HEADER_LEN:
                break
            kind, length = self._buf[2], int.from_bytes(self._buf[3:5], "little")
            if not _plausible(kind, length):
                self._skip(1)
                continue
            end = HEADER_LEN + length
            if len(self._buf) < end + CRC_LEN:
                # 長さを偽る見出しで止まらないよう、後ろに完全なフレームがあればそちらを採る。
                later = self._find_complete_frame(start=1)
                if later is None:
                    break
                self._skip(later)
                continue
            crc = int.from_bytes(self._buf[end : end + CRC_LEN], "little")
            if crc != crc16(bytes(self._buf[2:end])):
                self.crc_errors += 1
                self._skip(1)
                continue
            out.append(_parse(kind, bytes(self._buf[HEADER_LEN:end])))
            del self._buf[: end + CRC_LEN]
        return out

    def _seek_sync(self) -> bool:
        i = self._buf.find(SYNC)
        if i < 0:
            keep = 1 if self._buf[-1:] == SYNC[:1] else 0
            self._skip(len(self._buf) - keep)
            return False
        self._skip(i)
        return True

    def _skip(self, n: int) -> None:
        self.skipped_bytes += n
        del self._buf[:n]

    def _find_complete_frame(self, start: int) -> int | None:
        i = self._buf.find(SYNC, start)
        while i >= 0:
            if len(self._buf) >= i + HEADER_LEN:
                kind, length = self._buf[i + 2], int.from_bytes(self._buf[i + 3 : i + 5], "little")
                end = i + HEADER_LEN + length
                if _plausible(kind, length) and len(self._buf) >= end + CRC_LEN:
                    crc = int.from_bytes(self._buf[end : end + CRC_LEN], "little")
                    if crc == crc16(bytes(self._buf[i + 2 : end])):
                        return i
            i = self._buf.find(SYNC, i + 1)
        return None


@dataclass
class Recording:
    """1 つの録音セッションの DATA を位置の順に集め、欠落（位置の飛び）を記録する。"""

    session_id: int | None = None
    chunks: list[Data] = field(default_factory=list)
    gaps: list[tuple[int, int]] = field(default_factory=list)
    ended: Ended | None = None
    infos: list[Info] = field(default_factory=list)
    _next: int = 0

    def add(self, frame: LinkFrame) -> None:
        match frame:
            case Started(session_id):
                self.__init__(session_id=session_id)  # type: ignore[misc]
            case Data(position, _):
                if position < self._next:
                    raise ValueError(f"DATA position went backwards: {position} < {self._next}")
                if position > self._next:
                    self.gaps.append((self._next, position - self._next))
                self.chunks.append(frame)
                self._next = position + frame.n_frames
            case Ended():
                self.ended = frame
            case Info():
                self.infos.append(frame)

    @property
    def detected_dropped(self) -> int:
        return sum(n for _, n in self.gaps)

    @property
    def delivered_frames(self) -> int:
        return sum(c.n_frames for c in self.chunks)

    def pcm(self, fill_gaps: bool = False) -> bytes:
        """届いたフレームを連結した 16 bit LE ステレオの PCM。``fill_gaps`` なら欠落を無音で埋める。"""
        if not fill_gaps:
            return b"".join(c.frames for c in self.chunks)
        out = bytearray()
        for c in self.chunks:
            out += bytes(c.position * BYTES_PER_FRAME - len(out))
            out += c.frames
        return bytes(out)
