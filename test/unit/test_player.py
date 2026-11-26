"""e2e_audio.player の単体テスト。sounddevice の代わりに偽物を渡す。"""

import types

import pytest

from e2e_audio.player import find_capture_output, play_exclusive


def fake_sd(devices, hostapis):
    calls = []
    sd = types.SimpleNamespace(
        query_devices=lambda: devices,
        query_hostapis=lambda: hostapis,
        WasapiSettings=lambda exclusive: ("wasapi", exclusive),
        play=lambda *a, **k: calls.append((a, k)),
        calls=calls,
    )
    return sd


HOSTAPIS = [{"name": "MME"}, {"name": "Windows WASAPI"}]


def test_find_capture_output_picks_the_wasapi_output_named_e2e_audio_capture():
    devices = [
        {"name": "スピーカー (E2E Audio Capture)", "hostapi": 0, "max_output_channels": 2},
        {"name": "Speakers (Realtek)", "hostapi": 1, "max_output_channels": 2},
        {"name": "スピーカー (E2E Audio Capture)", "hostapi": 1, "max_output_channels": 2},
    ]
    assert find_capture_output(sd=fake_sd(devices, HOSTAPIS)) == 2


def test_find_capture_output_raises_when_absent():
    devices = [{"name": "Speakers (Realtek)", "hostapi": 1, "max_output_channels": 2}]
    with pytest.raises(LookupError):
        find_capture_output(sd=fake_sd(devices, HOSTAPIS))


def test_play_exclusive_uses_wasapi_exclusive_and_blocks():
    sd = fake_sd([], HOSTAPIS)
    play_exclusive(3, "signal", 48000, sd=sd)
    assert sd.calls == [(("signal", 48000), {"device": 3, "extra_settings": ("wasapi", True), "blocking": True})]
