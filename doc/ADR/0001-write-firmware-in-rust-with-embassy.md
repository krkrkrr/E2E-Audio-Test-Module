---
type: Architecture Decision Record
title: "ADR-0001: RP2040 のファームを Rust と embassy で書く"
description: USB Audio と CDC の複合デバイスを、embassy-usb の UAC1 Speaker と CDC ACM で実装する。
status: draft
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:17:22Z }
sources:
  - id: embassy-usb-crate
    resource: https://crates.io/crates/embassy-usb/0.6.0
    title: embassy-usb 0.6.0（src/class/uac1/speaker.rs を含む）
    last_modified: 2026-03-20T14:17:03Z
---

# ADR-0001: RP2040 のファームを Rust と embassy で書く

## Status

Proposed

## Context

RP2040 を USB Audio Class 1.0 の出力デバイスにし、同じ USB に CDC を並べる必要がある。

手元の Pico（COM6）は MicroPython で動いている。
MicroPython は USB Audio デバイスを作れないため、ファームは書き換える。

候補は C（Pico SDK + TinyUSB）、Arduino（Adafruit TinyUSB）、Rust（embassy）だった。
2026-11-26 のヒアリングで、ユーザーは Rust（embassy）を選んだ。

懸念は UAC クラスを自前で書く必要があるかどうかだった。
crates.io の embassy-usb 0.6.0 には、UAC1 Speaker クラスが含まれていることを確認した[^embassy-usb-crate]。
CDC ACM クラスも同じクレートにある。

## Decision

ファームは Rust で書き、embassy-rp と embassy-usb を使う。
USB Audio は embassy-usb の UAC1 Speaker、録音の転送は CDC ACM で実装する。

## Consequences

- UAC1 の記述子と制御要求の処理を自前で書かずに済む
- async タスクで、Audio の受信と CDC の送信を独立に書ける
- 録音経路のうち、ハードウェアに依存しない部分（フレーム化、連番、欠落の数え方）はホスト上の `cargo test` で検証できる
- embassy-usb の UAC1 Speaker は非同期フィードバックエンドポイントを持つ。フィードバック値をどう決めるかを Phase 2 で決める必要がある
- Rust ツールチェーン（`thumbv6m-none-eabi`）と UF2 変換の手段が要る。書き込みの経路は Q-0002 で扱う

[^embassy-usb-crate]: embassy-usb 0.6.0（2026-03-20 公開）の `src/class/uac1/speaker.rs`。2026-11-26 に crates.io から取得して確認した。
