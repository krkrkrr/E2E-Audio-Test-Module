"""e2e_audio.client の単体テスト。デバイスの代わりに、台本どおりに応える偽のポートを使う。"""

import threading

import pytest

from e2e_audio.client import CaptureClient, CaptureError
from e2e_audio.link import Data, Ended, Error, Info, Started, encode


class FakePort:
    """書かれたコマンド行に、台本の応答を返す偽のシリアルポート。"""

    def __init__(self, script):
        self.script = script
        self.written = []
        self._rx = bytearray()
        self._cv = threading.Condition()
        self.closed = False

    def write(self, data):
        self.written.append(data)
        with self._cv:
            for frame in self.script.get(data, []):
                self._rx += encode(frame)
            self._cv.notify_all()
        return len(data)

    def read(self, n):
        with self._cv:
            if not self._rx and not self.closed:
                self._cv.wait(0.01)
            out, self._rx[:] = bytes(self._rx[:n]), self._rx[n:]
            return out

    def close(self):
        with self._cv:
            self.closed = True
            self._cv.notify_all()


def test_start_sends_start_and_returns_the_session_id():
    port = FakePort({b"START\n": [Started(4)]})
    with CaptureClient(port) as c:
        assert c.start() == 4
    assert port.written == [b"START\n"]


def test_stop_reads_until_ended_and_returns_the_recording():
    port = FakePort({
        b"START\n": [Started(1)],
        b"STOP\n": [Data(0, bytes(8)), Data(4, bytes(4)), Ended(1, 5, 2)],
    })
    with CaptureClient(port) as c:
        c.start()
        rec = c.stop()
    assert rec.session_id == 1
    assert rec.delivered_frames == 3
    assert rec.gaps == [(2, 2)]
    assert rec.ended == Ended(1, 5, 2)


def test_an_error_response_raises():
    port = FakePort({b"START\n": [Error(1)]})
    with CaptureClient(port) as c, pytest.raises(CaptureError, match="AlreadyRecording"):
        c.start()


def test_info_returns_key_value_fields():
    port = FakePort({b"INFO\n": [Info("state=idle\nformat=48000/16/2\n")]})
    with CaptureClient(port) as c:
        assert c.info()["format"] == "48000/16/2"


def test_session_context_starts_and_stops_around_the_block():
    port = FakePort({b"START\n": [Started(2)], b"STOP\n": [Data(0, bytes(4)), Ended(2, 1, 0)]})
    with CaptureClient(port) as c:
        with c.session() as s:
            assert port.written == [b"START\n"]
        assert s.recording.ended == Ended(2, 1, 0)
