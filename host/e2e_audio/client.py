"""キャプチャデバイスを CDC で操作するクライアント。

デバイスのバッファは約 341 ms 分しかない（Q-0003）。読み遅れると欠落するので、
ポートを開いている間は裏のスレッドが読み続け、DATA を録音に積む。
設計: doc/context/capture-link/object-model.md
"""

from __future__ import annotations

import queue
import threading
import time
from contextlib import contextmanager
from dataclasses import dataclass
from typing import Iterator, Protocol

from .link import ERROR_NAMES, Data, Decoder, Ended, Error, Info, LinkFrame, Recording, Started


class Port(Protocol):
    def write(self, data: bytes) -> int | None: ...
    def read(self, n: int) -> bytes: ...
    def close(self) -> None: ...


class CaptureError(Exception):
    """デバイスが ERROR を返した、または応答が来なかった。"""


@dataclass
class SessionHandle:
    session_id: int
    recording: Recording | None = None


class CaptureClient:
    def __init__(self, port: Port, read_size: int = 4096) -> None:
        self.port = port
        self._read_size = read_size
        self._decoder = Decoder()
        self._controls: queue.Queue[LinkFrame] = queue.Queue()
        self._recording = Recording()
        self._ended = threading.Event()
        self._lock = threading.Lock()
        self._closing = False
        self._reader = threading.Thread(target=self._read_loop, name="capture-link-reader", daemon=True)
        self._reader.start()

    @classmethod
    def open(cls, device: str, timeout: float = 0.05) -> CaptureClient:
        import serial

        return cls(serial.Serial(device, timeout=timeout))

    def __enter__(self) -> CaptureClient:
        return self

    def __exit__(self, *exc) -> None:
        self.close()

    def close(self) -> None:
        self._closing = True
        self.port.close()
        self._reader.join(timeout=1)

    # --- コマンド ---

    def start(self, timeout: float = 2.0) -> int:
        """START を送り、新しいセッションの id を返す。"""
        self._send(b"START\n")
        reply = self._next_control(timeout)
        if not isinstance(reply, Started):
            raise CaptureError(f"unexpected reply to START: {reply}")
        return reply.session_id

    def stop(self, timeout: float = 5.0) -> Recording:
        """STOP を送り、ENDED まで読んで録音を返す。"""
        self._send(b"STOP\n")
        deadline = time.monotonic() + timeout
        while not self._ended.wait(0.01):
            self._raise_if_error()
            if time.monotonic() > deadline:
                raise CaptureError("ENDED did not arrive")
        with self._lock:
            return self._recording

    def info(self, timeout: float = 2.0) -> dict[str, str]:
        self._send(b"INFO\n")
        reply = self._next_control(timeout)
        if not isinstance(reply, Info):
            raise CaptureError(f"unexpected reply to INFO: {reply}")
        return reply.fields()

    @contextmanager
    def session(self) -> Iterator[SessionHandle]:
        """ブロックの間だけ録音する。抜けたあと ``handle.recording`` に録音が入る。"""
        handle = SessionHandle(self.start())
        try:
            yield handle
        finally:
            handle.recording = self.stop()

    # --- 内部 ---

    def _send(self, line: bytes) -> None:
        if line == b"START\n":
            self._ended.clear()
        self.port.write(line)

    def _next_control(self, timeout: float) -> LinkFrame:
        try:
            reply = self._controls.get(timeout=timeout)
        except queue.Empty:
            raise CaptureError("no reply from the device") from None
        if isinstance(reply, Error):
            raise CaptureError(ERROR_NAMES.get(reply.code, f"error {reply.code}"))
        return reply

    def _raise_if_error(self) -> None:
        try:
            reply = self._controls.get_nowait()
        except queue.Empty:
            return
        if isinstance(reply, Error):
            raise CaptureError(ERROR_NAMES.get(reply.code, f"error {reply.code}"))

    def _read_loop(self) -> None:
        while not self._closing:
            try:
                data = self.port.read(self._read_size)
            except Exception:
                if self._closing:
                    return
                raise
            for frame in self._decoder.feed(data):
                self._dispatch(frame)

    def _dispatch(self, frame: LinkFrame) -> None:
        with self._lock:
            match frame:
                case Started():
                    self._recording = Recording()
                    self._recording.add(frame)
                    self._controls.put(frame)
                case Data():
                    self._recording.add(frame)
                case Ended():
                    self._recording.add(frame)
                    self._ended.set()
                case _:
                    self._controls.put(frame)
