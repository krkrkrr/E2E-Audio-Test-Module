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

各行は、それを守るテストへリンクする。

### AUD-2 音声パケットを受け、音声フレームとして公開する

| ID | プロパティ | 種別 | 表現 | テスト |
|---|---|---|---|---|
| P-AUD-1 | フレーム化はバイト列を保存する | ラウンドトリップ | `∀ p: concat(bytes(receive(p))) == p[..len(p) - len(p) % 4]` | [audio_in.rs](../../../test/integration/audio_in.rs) |
| P-AUD-2 | 数は単調に、パケット長どおりに増える | 単調 | `Δframes_received == len(p) / 4`、`Δpartial_packets == (len(p) % 4 != 0)` | audio_in.rs |
| P-AUD-3 | ホスト制御値はサンプルに作用しない | 同値 | `∀ c, p: receive(p) under c == receive(p) under default` | audio_in.rs |
| P-AUD-4 | 閉じている間は何も公開しない | 否定 | `state == Closed ⇒ receive(p) == []` | audio_in.rs |

### AUD-1 / AUD-3 ストリームとフィードバック

| ID | プロパティ | 種別 | 表現 | テスト |
|---|---|---|---|---|
| P-AUD-5 | 代替設定の適用は冪等 | 冪等 | `set_alt(a); set_alt(a) ≡ set_alt(a)` | audio_in.rs |
| P-AUD-6 | フィードバックは閉じていれば無く、開いていれば常に 48.0 | 不変量 | `∀ 操作列: feedback ∈ {None if Closed, 00 00 0C if Open}` | audio_in.rs |
