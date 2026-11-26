"""`@harness` タグのシナリオを、ハーネス側の復号器に対して実行する（ADR-0005）。

それ以外のシナリオは test/integration/features.rs（cucumber-rs）が実行する。
"""

from pathlib import Path

from pytest_bdd import given, parsers, scenario, then, when

from e2e_audio.link import Data, Decoder, Recording, Started, encode

FEATURE = str(Path(__file__).parents[2] / "doc/context/capture-link/features/receive-capture-data.feature")


@scenario(FEATURE, "Error — CRC が合わないリンクフレームはハーネスで捨てられ、位置の飛びとして検出される")
def test_crc_mismatch_is_reported_as_gap():
    pass


@scenario(FEATURE, "Error — 途中から読み始めても、同期語と CRC で同期を取り直せる")
def test_resync_from_middle():
    pass


def _frames(n, seed):
    return bytes((seed + i) % 256 for i in range(4 * n))


@given("CDC が開かれている")
def cdc_open():
    pass


@given("録音セッションが Recording である", target_fixture="link")
def recording():
    return {"sent": [Started(1)]}


@given("ハーネスが受けた DATA のうち 1 つのペイロードが 1 ビット壊れている")
def one_corrupted(link):
    chunks = [Data(0, _frames(48, 0)), Data(48, _frames(48, 1)), Data(96, _frames(48, 2))]
    link["sent"] += chunks
    raw = [bytearray(encode(f)) for f in link["sent"]]
    raw[2][20] ^= 0x01  # 2 つ目の DATA のペイロード
    link["raw"] = b"".join(raw)
    link["corrupted"] = chunks[1]
    link["after"] = [chunks[2]]


@given("CDC の受信列の先頭に、無関係なバイト列と DATA の後半が残っている")
def garbage_prefix(link):
    stale = encode(Data(0, _frames(48, 9)))
    fresh = [Data(48, _frames(48, 3)), Data(96, _frames(48, 4))]
    link["raw"] = b"\x00\xa5>>> \xa5\x5a\x02garbage" + stale[len(stale) // 2 :] + b"".join(encode(f) for f in fresh)
    link["after"] = fresh


@when("ハーネスがリンクフレームを復号する")
def decode(link):
    decoder = Decoder()
    link["decoder"] = decoder
    link["decoded"] = decoder.feed(link["raw"])
    rec = Recording()
    for f in link["decoded"]:
        rec.add(f)
    link["recording"] = rec


@then("壊れたフレームは CRC 不一致として捨てられる")
def corrupted_dropped(link):
    assert link["corrupted"] not in link["decoded"]
    assert link["decoder"].crc_errors >= 1


@then(parsers.re("後続の DATA は正しく復号される|最初の完全なリンクフレームから正しく復号される"))
def following_decoded(link):
    assert link["decoded"][-len(link["after"]) :] == link["after"]


@then("その区間は欠落として報告される")
def reported_as_gap(link):
    assert link["recording"].gaps == [(48, 48)]
    assert link["recording"].detected_dropped == 48
