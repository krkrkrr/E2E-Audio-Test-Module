---
type: System Context
title: E2E-Audio-Test-Module — システムコンテキスト
description: キャプチャデバイス（RP2040 ファーム）と、それが接続する相手。コンテキストはここから数える。
---

# システムコンテキスト

**リポジトリに 1 枚だけ。** 増分ごとに描き直さず、この図を更新する。

システムは **キャプチャデバイス**（RP2040 上の Rust ファーム）である。
接続する相手は 2 つあり、相手ごとに 1 つずつコンテキストを置く。

```mermaid
flowchart LR
  App["テスト対象アプリ"] -->|音を出す| Engine
  subgraph Host["ホスト PC（Windows）"]
    App
    Engine["ホストのオーディオ経路<br/>（Windows オーディオエンジン + USB Audio ドライバ）"]
    Harness["テストハーネス<br/>（Python）"]
  end
  subgraph Device["キャプチャデバイス（RP2040）"]
    AudioIn["audio-in"]
    Link["capture-link"]
    AudioIn -->|音声フレーム| Link
  end
  Engine -->|"USB Audio 1.0<br/>48 kHz / 16 bit / 2 ch"| AudioIn
  AudioIn -->|フィードバック| Engine
  Harness -->|"CDC: START / STOP / INFO"| Link
  Link -->|"CDC: リンクフレーム"| Harness
```

| コンテキスト | 相手 | 経路 | 詳細 |
|---|---|---|---|
| `audio-in` | ホストのオーディオ経路 | USB Audio 1.0（アイソクロナス OUT + フィードバック IN） | [context/audio-in/](context/audio-in/domain-model.md) |
| `capture-link` | テストハーネス | USB CDC ACM（バルク） | [context/capture-link/](context/capture-link/domain-model.md) |

テスト対象アプリはキャプチャデバイスと直接つながらない。
ホストのオーディオ経路を通してのみ影響するので、コンテキストを持たない。

## 境界の翻訳

`audio-in` は USB パケットを **音声フレーム** に翻訳して公開する。
`capture-link` は音声フレームだけを受け取り、USB Audio の語彙（代替設定、パケット、フィードバック）を知らない。
