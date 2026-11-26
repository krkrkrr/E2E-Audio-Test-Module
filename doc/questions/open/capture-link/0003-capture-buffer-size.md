---
type: Question
title: "Q-0003: キャプチャバッファ 16,384 フレーム（約 341 ms）で、ホストの読み出しの揺らぎを吸収できるか？"
description: バッファ容量は RAM の都合で選んだ値で、Windows 上のハーネスが CDC を読み遅れる最大時間を測っていない。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:18:52Z }
---

# Q-0003: キャプチャバッファ 16,384 フレーム（約 341 ms）で、ホストの読み出しの揺らぎを吸収できるか？

**Status: `open`** — the directory this file sits in is the status.
**`git mv` it when that changes.** Do not write the status anywhere else.

## The question

CAP-R8 はキャプチャバッファを 64 KiB（16,384 フレーム）と定めた。
これは RP2040 の RAM 264 KB のうち、スタックや USB のバッファを残して確保しやすい値として選んだにすぎない。
Windows 上の Python ハーネスが CDC を読み遅れる時間が 341 ms を超えれば欠落が起きる。

## What it would take

- ファームを書き込み、長時間（例: 10 分）の録音で欠落数を測る
- ハーネスに負荷をかけた状態（GC、WAV 書き出し）でも測る
- 結果を `doc/evidence/` に記録し、容量を確定するか変更する

## What is known

- 48 kHz / 2 ch / 16 bit で 192,000 B/s。64 KiB は約 341 ms 分
- 欠落は位置の飛びとして必ず検出できる（CAP-R4）ので、容量不足は黙って失われない
- 2026-11-26: ファーム全体の RAM は `.bss` 約 70 KB（264 KB 中）。バッファを広げる余地はある
- 2026-11-26: Windows 上のハーネスで 5 秒、serial MCP 経由で約 9.5 秒録音し、どちらも欠落 0（[証拠](../../../evidence/2026-11-26-first-flash.md)）。長時間と、ハーネスに負荷がある状態はまだ測っていない
