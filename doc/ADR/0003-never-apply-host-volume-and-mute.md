---
type: Architecture Decision Record
title: "ADR-0003: ホストの音量とミュートを受け付けるが適用せず、フィードバックは 48.0 kHz 固定にする"
description: embassy-usb の Speaker が公開する音量・ミュート制御をサンプルに作用させず、フィードバックは公称レートを返し続ける。
status: stable
verified: { by: human:krr, at: 2026-11-26T09:21:56Z }
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:18:52Z }
sources:
  - id: embassy-usb-speaker
    resource: https://crates.io/crates/embassy-usb/0.6.0
    title: embassy-usb 0.6.0 src/class/uac1/speaker.rs
    last_modified: 2026-03-20T14:17:03Z
---

# ADR-0003: ホストの音量とミュートを受け付けるが適用せず、フィードバックは 48.0 kHz 固定にする

## Status

Accepted

## Context

合格基準は、USB で届いたサンプルと CDC で返したサンプルのビット一致である（[requirements](../increments/2026-11-26-usb-audio-capture/requirements.md)）。

embassy-usb 0.6.0 の UAC1 Speaker は、チャンネルごとの音量とミュートを Feature Unit として常に公開する[^embassy-usb-speaker]。
ホストが音量を変えると、その値がデバイスに届く。
デバイスがそれを適用すれば、サンプルは変わる。

一方、Windows はデバイスが音量制御を持つとき、その制御に音量を任せることが多い。
デバイスに音量制御が無ければ、OS のミキサがサンプルに音量を掛ける。
したがって、制御を公開したうえで適用しないほうが、OS 経路でのサンプル変化も減らせる可能性がある。
これが本当に成り立つかは測っていない（[Q-0001](../questions/answered/host-audio/0001-bit-exact-host-path.md)）。

同じ Speaker は非同期のフィードバックエンドポイントも持つ。
通常のスピーカは DAC のクロックに合わせてフィードバック値を調整する。
このデバイスには DAC が無く、受けた量をそのまま CDC に流すので、合わせるべきクロックが無い。

## Decision

ホストが設定した音量とミュートは記録するだけにし、サンプルには一切作用させない。
記録した値は INFO で報告する。

フィードバック値は、公称レート 48.0 kHz（10.14 形式で `0x0C0000`）を常に返す。

## Consequences

- ホストの音量やミュートをどう操作しても、ファームが返すサンプルは変わらない
- 音量の操作がデバイス側で効かないため、人がこのデバイスで音を聴くことはできない（もともと出力は無い）
- ホストの USB クロックと公称レートのずれは、ホストがパケットあたりのフレーム数（47〜49）で吸収する。デバイスはどの数でも受け入れる
- Windows が実際に音量をデバイスに任せるかは Q-0001 の測定で確かめる

[^embassy-usb-speaker]: embassy-usb 0.6.0 の `src/class/uac1/speaker.rs` 冒頭のドキュメントと記述子の組み立て。2026-11-26 に crates.io の配布物で確認した。
