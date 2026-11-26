"""e2e_audio.link の単体テスト。"""

from e2e_audio.link import Data, Decoder, Ended, Error, Info, Recording, Started, crc16, encode


def test_crc16_check_value_is_29b1():
    assert crc16(b"123456789") == 0x29B1


def test_each_kind_round_trips():
    for frame in [Started(1), Data(5, bytes(8)), Ended(1, 10, 2), Info("state=idle\n"), Error(3)]:
        assert Decoder().feed(encode(frame)) == [frame]


def test_leading_garbage_is_skipped():
    d = Decoder()
    assert d.feed(b"\x00\x01\xa5garbage" + encode(Started(7))) == [Started(7)]
    assert d.skipped_bytes > 0


def test_crc_mismatch_is_counted_and_skipped():
    bad = bytearray(encode(Started(1)))
    bad[-1] ^= 0xFF
    d = Decoder()
    assert d.feed(bytes(bad) + encode(Started(2))) == [Started(2)]
    assert d.crc_errors == 1


def test_a_fake_header_with_a_long_length_does_not_swallow_following_frames():
    fake = b"\xa5\x5a\x02\x00\x04"  # DATA、長さ 1024 と称する偽の見出し
    assert Decoder().feed(fake + encode(Started(3)) + encode(Error(1))) == [Started(3), Error(1)]


def test_recording_records_position_gaps_as_drops_and_builds_pcm():
    rec = Recording()
    for f in [Started(1), Data(0, b"\x01\x00\x02\x00"), Data(3, b"\x03\x00\x04\x00"), Ended(1, 4, 2)]:
        rec.add(f)
    assert rec.session_id == 1
    assert rec.gaps == [(1, 2)]
    assert rec.detected_dropped == 2
    assert rec.delivered_frames == 2
    assert rec.ended == Ended(1, 4, 2)
    assert rec.pcm() == b"\x01\x00\x02\x00\x03\x00\x04\x00"
    assert rec.pcm(fill_gaps=True) == b"\x01\x00\x02\x00" + bytes(8) + b"\x03\x00\x04\x00"
