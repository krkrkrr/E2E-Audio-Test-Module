"""キャプチャデバイスへ、WASAPI 排他モードで信号を鳴らす。

排他モードにすると Windows のミキサ（音量、リサンプル）を通らない。
ビット一致がこの条件で成り立つかは Q-0001 で確かめる。
"""

from __future__ import annotations

DEVICE_NAME = "E2E Audio Capture"


def _sounddevice(sd):
    if sd is not None:
        return sd
    import sounddevice

    return sounddevice


def find_capture_output(sd=None) -> int:
    """名前に ``E2E Audio Capture`` を含む、WASAPI の 2 ch 出力デバイスの番号を返す。"""
    sd = _sounddevice(sd)
    hostapis = sd.query_hostapis()
    for index, dev in enumerate(sd.query_devices()):
        if (
            DEVICE_NAME in dev["name"]
            and dev["max_output_channels"] >= 2
            and "WASAPI" in hostapis[dev["hostapi"]]["name"]
        ):
            return index
    raise LookupError(f"no WASAPI output named {DEVICE_NAME!r}")


def play_exclusive(device: int, signal, rate: int, sd=None) -> None:
    """``signal``（int16、形状 (n, 2)）を排他モードで鳴らし、鳴り終わるまで待つ。"""
    sd = _sounddevice(sd)
    sd.play(signal, rate, device=device, extra_settings=sd.WasapiSettings(exclusive=True), blocking=True)
