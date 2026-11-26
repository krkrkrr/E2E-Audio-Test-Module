"""ハーネス側の復号器のプロパティ（doc/context/capture-link/constraints.md の P-CAP-1〜4）。"""

import json
import struct
from pathlib import Path

from hypothesis import given, strategies as st

from e2e_audio.link import (
    Data,
    Decoder,
    Ended,
    Error,
    Info,
    Recording,
    Started,
    crc16,
    encode,
)

VECTORS = json.loads((Path(__file__).parents[1] / "fixtures" / "link_vectors.json").read_text(encoding="utf-8"))

frames_bytes = st.integers(1, 256).flatmap(lambda n: st.binary(min_size=4 * n, max_size=4 * n))

link_frames = st.one_of(
    st.builds(Started, st.integers(0, 2**32 - 1)),
    st.builds(Data, st.integers(0, 2**64 - 1), frames_bytes),
    st.builds(Ended, st.integers(0, 2**32 - 1), st.integers(0, 2**64 - 1), st.integers(0, 2**64 - 1)),
    st.builds(Info, st.text(max_size=200)),
    st.builds(Error, st.integers(0, 255)),
)


def test_crc16_is_ccitt_false():
    assert crc16(b"123456789") == 0x29B1


@given(link_frames)
def test_p_cap_1_roundtrip(frame):
    """P-CAP-1: decode(encode(f)) == f"""
    assert Decoder().feed(encode(frame)) == [frame]


@given(link_frames, st.data())
def test_p_cap_2_single_bit_flip_is_detected(frame, data):
    """P-CAP-2: 1 ビットの破損で、元のフレームは決して返らない。"""
    raw = bytearray(encode(frame))
    bit = data.draw(st.integers(0, len(raw) * 8 - 1))
    raw[bit // 8] ^= 1 << (bit % 8)
    assert frame not in Decoder().feed(bytes(raw))


@given(st.binary(max_size=300), st.lists(link_frames, min_size=1, max_size=8))
def test_p_cap_3_resync_after_garbage(garbage, frames):
    """P-CAP-3: 前置のゴミがあっても、後続のフレームは末尾として復号できる。"""
    decoded = Decoder().feed(garbage + b"".join(encode(f) for f in frames))
    assert decoded[-len(frames):] == frames


@given(st.lists(link_frames, max_size=8), st.lists(st.integers(0, 4000), max_size=10))
def test_decoder_is_independent_of_chunking(frames, cuts):
    raw = b"".join(encode(f) for f in frames)
    points = sorted({0, len(raw), *(c % (len(raw) + 1) for c in cuts)})
    d = Decoder()
    out = []
    for a, b in zip(points, points[1:]):
        out += d.feed(raw[a:b])
    assert out == frames


def _expected(v):
    f = v["fields"]
    match v["kind"]:
        case 0x01:
            return Started(f["session_id"])
        case 0x02:
            return Data(f["position"], b"".join(struct.pack("<hh", l, r) for l, r in f["frames"]))
        case 0x03:
            return Ended(f["session_id"], f["total_frames"], f["dropped_frames"])
        case 0x04:
            return Info(f["text"])
        case 0x7F:
            return Error(f["code"])


def test_p_cap_4_decoder_matches_shared_vectors():
    """P-CAP-4: Rust の符号化と同じ試験ベクタを、Python の復号器も満たす。"""
    for v in VECTORS["vectors"]:
        assert Decoder().feed(bytes.fromhex(v["hex"])) == [_expected(v)], v["name"]
        assert encode(_expected(v)).hex() == v["hex"], v["name"]


@given(st.lists(st.tuples(st.integers(0, 50), st.integers(1, 64)), max_size=20))
def test_recording_reports_gaps_as_drops(chunks):
    """位置の飛びの合計が、Recording の検出する欠落数になる（P-CAP-6 のハーネス側）。"""
    rec = Recording()
    rec.add(Started(1))
    pos = 0
    total_gap = 0
    for gap, n in chunks:
        pos += gap
        total_gap += gap
        rec.add(Data(pos, bytes(4 * n)))
        pos += n
    assert rec.detected_dropped == total_gap
    assert len(rec.pcm(fill_gaps=True)) == 4 * pos
