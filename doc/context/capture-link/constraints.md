---
type: Constraints
title: capture-link — 制約
description: capture-link コンテキストが守る規則と不変条件。各不変条件はそれを守るテストへリンクする。
---

# capture-link — 制約

正本: [system-context.md](../../system-context.md)。

## 守るべき規則

| # | 規則 | 出典 |
|---|---|---|
| CAP-R1 | 録音データはデバイスに保存せず、CDC で連続して送る | [ADR-0002](../../ADR/0002-stream-capture-over-usb-cdc.md) |
| CAP-R2 | 録音の範囲は START と STOP のコマンドだけで決まる | [requirements](../../increments/2026-11-26-usb-audio-capture/requirements.md) |
| CAP-R3 | 音声フレームのバイト列は、`audio-in` から受けたものと同一のまま送る | requirements |
| CAP-R4 | 欠落は黙って失わない。位置の飛びと ENDED の `dropped_frames` で必ず伝える | requirements |
| CAP-R5 | バッファが満杯のときは新しい側を捨てる（送り済みでない古いフレームを上書きしない） | この増分の Phase 2 |
| CAP-R6 | ENDED は、STOP 以前に受けたフレームをすべて送ったあとにだけ送る | この増分の Phase 2 |
| CAP-R7 | リンクフレームの形式は ADR-0004 に従う | [ADR-0004](../../ADR/0004-frame-cdc-link-with-sync-and-crc.md) |
| CAP-R8 | キャプチャバッファは 16,384 フレーム（64 KiB、約 341 ms）。**根拠は弱い（Q-0003）** | この増分の Phase 2 |

## 不変条件

Phase 4 で追記する。
