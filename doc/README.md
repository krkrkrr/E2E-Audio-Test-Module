---
type: Documentation Index
title: E2E-Audio-Test-Module — ドキュメント
description: E2E-Audio-Test-Module の設計・決定・履歴の置き場所と、増分の年表。
---

# E2E-Audio-Test-Module — ドキュメント

RP2040 を USB Audio の出力先にし、ホストのアプリが出した音を録音して E2E テストに使う。

| 何を知りたいか | どこ |
|---|---|
| システムの境界とコンテキスト | [system-context.md](system-context.md) |
| ユースケース | [use-cases.md](use-cases.md) |
| コンテキストごとの設計 | [context/audio-in/](context/audio-in/domain-model.md)、[context/capture-link/](context/capture-link/domain-model.md) |
| 用語 | [glossary.md](glossary.md) |
| 決定とその理由 | [ADR/](ADR/) |
| 未解決の問い | [questions/index.md](questions/index.md) |
| テスト戦略 | [testing-strategy.md](testing-strategy.md) |
| 測ったこと | [evidence/](evidence/) |
| 増分ごとの要件 | [increments/](increments/) |

## 年表

| 日付 | 増分 | コンテキスト | Status |
|---|---|---|---|
| 2026-11-26 | [usb-audio-capture](increments/2026-11-26-usb-audio-capture/) | audio-in, capture-link | Phase 4 complete — features: audio-in [stream-open-close](context/audio-in/features/stream-open-close.feature), [receive-audio-packet](context/audio-in/features/receive-audio-packet.feature), [rate-feedback](context/audio-in/features/rate-feedback.feature); capture-link [start-session](context/capture-link/features/start-session.feature), [receive-capture-data](context/capture-link/features/receive-capture-data.feature), [stop-session](context/capture-link/features/stop-session.feature), [device-info](context/capture-link/features/device-info.feature) |
