# E2E-Audio-Test-Module

> Raspberry Pi Pico（RP2040）を USB Audio の出力先にし、ホストのアプリが出した音をビット一致のまま USB CDC で取り出す、E2E 音声テスト用の計測器。

ホストからは「E2E Audio Capture」というスピーカと、シリアルポートの複合デバイスに見えます。
シリアルポートに `START` を送ってから `STOP` を送るまでに受けた音声が、そのままシリアルで返ってきます。

## 構成

| ディレクトリ | 中身 |
|---|---|
| `capture-core/` | ファームの中核（`no_std`）。USB に依存しない状態機械で、PC 上でテストできる |
| `firmware/` | RP2040 用ファーム（Rust + embassy）。USB の配線だけを持つ |
| `host/` | Python のハーネス（`e2e_audio`）: 録音の取り出し、復号、WASAPI 排他モードでの再生 |
| `test/` | 単体・結合（プロパティ）・フィーチャー・E2E のテスト |
| `doc/` | 設計・決定・履歴 |

## ファームの書き込み

```sh
cd firmware
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/e2e-audio-firmware dist/e2e-audio-capture.uf2
```

Pico の BOOTSEL ボタンを押しながら USB に挿し、現れたドライブ `RPI-RP2` に `e2e-audio-capture.uf2` をコピーします。

必要なもの: Rust（`rustup target add thumbv6m-none-eabi`）、`cargo install elf2uf2-rs`。

## 録音の取り出し（Python）

```python
from e2e_audio.client import CaptureClient

with CaptureClient.open("COM6") as c:
    with c.session() as s:
        ...  # ここでテスト対象アプリに音を出させる
    rec = s.recording
    print(rec.ended, rec.detected_dropped)
    pcm = rec.pcm()  # 48 kHz / 16 bit / 2 ch のリトルエンディアン PCM
```

## テスト

```sh
cargo test                                   # capture-core の単体・プロパティ・フィーチャー
uv run --project host --extra test pytest    # ハーネス・文書・@harness シナリオ
```

実機での E2E は Windows で実行します。

```bat
set E2E_AUDIO_PORT=COM6
uv run --project host --extra test --extra play pytest -m device test/e2e
```

## ドキュメント

設計・決定・履歴: [doc/README.md](doc/README.md)
