---
type: Question
title: "Q-0005: 共有モードで鳴らすアプリの音は、USB にビット一致で届くか？"
description: 排他モードではビット一致を確かめたが、テスト対象のアプリの多くは共有モードで鳴らすため、E2E の合格基準をどう置くかが決まらない。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T10:05:00Z }
---

# Q-0005: 共有モードで鳴らすアプリの音は、USB にビット一致で届くか？

**Status: `open`** — the directory this file sits in is the status.
**`git mv` it when that changes.** Do not write the status anywhere else.

## The question

[Q-0001](../../answered/host-audio/0001-bit-exact-host-path.md) で、WASAPI 排他モードならビット一致することが分かった。
しかし、テスト対象のアプリ（ブラウザ、一般のアプリ）はふつう共有モードで鳴らし、Windows のミキサを通る。
共有モードでもビット一致するか。しないなら、E2E の合格基準をどう置くか（相関、許容誤差、周波数領域の比較）。

## What it would take

- 共有モードで既知の信号を鳴らし、録音と比べる（`player` に共有モードの再生を足す）
- 条件を変えて測る: 既定フォーマット 48 kHz / 16 bit、音量 100 %、オーディオ拡張の有無、44.1 kHz の音源
- 結果を `doc/evidence/` に記録し、合格基準を ADR で決める

## What is known

- 共有モードのミキサは浮動小数点で合成し、音量を掛ける（一般的な知識。未測定）
- このデバイスは音量制御を持つと名乗るが適用しない（ADR-0003）。Windows がこれを見て音量をミキサで掛けないかどうかは未測定
- 2026-11-26 の INFO では `volume_8q8=0,0` だった。ホストが音量を一度も設定していないのか、0 dB を設定したのかは区別できていない
