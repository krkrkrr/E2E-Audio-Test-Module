---
type: Use Case Catalog
title: E2E-Audio-Test-Module — ユースケース一覧
description: リポジトリ全体のユースケース。ID にはコンテキストの接頭辞を付ける。
---

# ユースケース一覧

**リポジトリに 1 つだけ。** 各コンテキストの `object-model.md` は、自分の行だけを再掲する。
コンテキストは [system-context.md](system-context.md) を見よ。

| ID | ユースケース | アクター | コンテキスト | 追加した増分 |
|---|---|---|---|---|
| AUD-1 | 音声ストリームを開く・閉じる | ホストのオーディオ経路 | audio-in | usb-audio-capture |
| AUD-2 | 音声パケットを受け、音声フレームとして公開する | ホストのオーディオ経路 | audio-in | usb-audio-capture |
| AUD-3 | 送出レートのフィードバックを返す | ホストのオーディオ経路 | audio-in | usb-audio-capture |
| CAP-1 | 録音セッションを開始する | テストハーネス | capture-link | usb-audio-capture |
| CAP-2 | 録音データを受け取る | テストハーネス | capture-link | usb-audio-capture |
| CAP-3 | 録音セッションを停止する | テストハーネス | capture-link | usb-audio-capture |
| CAP-4 | デバイス情報を問い合わせる | テストハーネス | capture-link | usb-audio-capture |

```mermaid
flowchart LR
  Engine(["ホストのオーディオ経路"])
  Harness(["テストハーネス"])
  Engine --- AUD1["AUD-1 ストリームを開く・閉じる"]
  Engine --- AUD2["AUD-2 音声パケットを受ける"]
  Engine --- AUD3["AUD-3 フィードバックを返す"]
  Harness --- CAP1["CAP-1 録音を開始する"]
  Harness --- CAP2["CAP-2 録音データを受け取る"]
  Harness --- CAP3["CAP-3 録音を停止する"]
  Harness --- CAP4["CAP-4 デバイス情報を問い合わせる"]
```
