"""実機 E2E（doc/context/capture-link/constraints.md の P-E2E-1、P-E2E-2）。

Windows 上で、キャプチャデバイスを接続した状態で実行する。
WASAPI 排他モードで任意の信号を鳴らし、CDC で受けた録音とビット一致することを確かめる。

    $env:E2E_AUDIO_PORT = "COM7"   # PowerShell（cmd.exe なら set E2E_AUDIO_PORT=COM7）
    pytest -m device test/e2e

ビット一致が成り立つ Windows 側の条件は Q-0001 で調べている。
"""

import os
import sys

import numpy as np
import pytest
from hypothesis import HealthCheck, given, settings, strategies as st

pytestmark = [
    pytest.mark.device,
    pytest.mark.skipif(sys.platform != "win32", reason="Windows の音声経路が要る"),
    pytest.mark.skipif("E2E_AUDIO_PORT" not in os.environ, reason="E2E_AUDIO_PORT が未設定"),
]

RATE = 48_000


@pytest.fixture(scope="module")
def client():
    from e2e_audio.client import CaptureClient

    with CaptureClient.open(os.environ["E2E_AUDIO_PORT"]) as c:
        yield c


@pytest.fixture(scope="module")
def output_device():
    from e2e_audio.player import find_capture_output

    return find_capture_output()


def signals():
    """0.1〜1.0 秒の任意のステレオ int16 信号。先頭と末尾は無音と区別できるよう 0 以外にする。"""
    return st.integers(int(0.1 * RATE), RATE).flatmap(
        lambda n: st.integers(0, 2**32 - 1).map(lambda seed: _noise(n, seed))
    )


def _noise(n, seed):
    rng = np.random.default_rng(seed)
    x = rng.integers(-32768, 32768, size=(n, 2), dtype=np.int16)
    x[0] = x[-1] = (1234, -1234)
    return x


def _find(haystack: bytes, needle: bytes) -> int:
    return haystack.find(needle)


@settings(max_examples=5, deadline=None, suppress_health_check=[HealthCheck.function_scoped_fixture])
@given(signals())
def test_p_e2e_played_signal_is_captured_bit_exact(client, output_device, signal):
    from e2e_audio.player import play_exclusive

    with client.session() as session:
        play_exclusive(output_device, signal, RATE)
    rec = session.recording

    # P-E2E-2: 保存則と欠落 0
    assert rec.ended is not None
    assert rec.delivered_frames + rec.ended.dropped_frames == rec.ended.total_frames
    assert rec.ended.dropped_frames == 0
    assert rec.detected_dropped == 0

    # P-E2E-1: 鳴らした信号が、録音のどこかに連続してビット一致で現れる
    played = signal.astype("<i2").tobytes()
    at = _find(rec.pcm(), played)
    assert at >= 0, "played signal not found bit-exact in the capture"
    assert at % 4 == 0
