---
type: Constraints
title: audio-in — 制約
description: audio-in コンテキストが守る規則と不変条件。各不変条件はそれを守るテストへリンクする。
---

# audio-in — 制約

正本: [system-context.md](../../system-context.md)。

## 守るべき規則

| # | 規則 | 出典 |
|---|---|---|
| AUD-R1 | USB Audio Class 1.0 の出力デバイス（スピーカ）としてホストに見える | [requirements](../../increments/2026-11-26-usb-audio-capture/requirements.md) |
| AUD-R2 | 公開するフォーマットは 48 kHz / 16 bit 符号付き LE / 2 ch の 1 つだけ | requirements |
| AUD-R3 | サンプル値を加工しない。音量、ミュート、リサンプル、ディザを適用しない | requirements、[ADR-0003](../../ADR/0003-never-apply-host-volume-and-mute.md) |
| AUD-R4 | フィードバック値は 48.0 kHz の固定値を返す | ADR-0003 |
| AUD-R5 | 端数バイトは音声フレームにせず、数を記録する | この増分の Phase 2 |

## 不変条件

Phase 4 で追記する。
