---
type: Architecture Decision Record
title: "ADR-0002: 録音はデバイスに貯めず USB CDC でリアルタイムに流す"
description: 受けた音声を RP2040 内に保存せず、START から STOP まで同じ USB の CDC でホストへ連続転送する。
status: draft
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:17:22Z }
---

# ADR-0002: 録音はデバイスに貯めず USB CDC でリアルタイムに流す

## Status

Proposed

## Context

48 kHz / 16 bit / 2 ch の音声は 192,000 B/s になる。
RP2040 の RAM は 264 KB なので、RAM に貯めると約 1 秒で一杯になる。
Pico の内蔵 Flash（2 MB）でも約 10 秒で、書き込みは遅く、書き換え回数にも上限がある。
SD カードは配線と部品を増やす。

一方、USB Full Speed は 12 Mbps あり、Audio の受信と同じ量を CDC で送り返す帯域は残る。
ホストには serial MCP と `rfc2217_server.py` があり、CDC なら既存の道具で読める。

2026-11-26 のヒアリングで、ユーザーは「USB でリアルタイム転送」「USB CDC シリアル」「CDC コマンドで明示的に開始と停止」を選んだ。

## Decision

録音データは RP2040 内に保存しない。
USB Audio と CDC ACM の複合デバイスにし、ホストが CDC で START を送ってから STOP を送るまで、受けたサンプルを CDC で連続して送る。
送るデータには連番を付け、デバイス側で落としたサンプルの量をホストが知れるようにする。

## Consequences

- 録音時間に上限がない
- デバイス側のバッファは、ホストの読み出しの揺らぎを吸収する分だけで済む
- ホストが CDC を読み続けなければ、デバイス側で欠落が起きる。欠落を黙って捨てず、検出できる形で伝える必要がある
- CDC は Audio と制御コマンドの両方を運ぶため、バイト列の上にフレームの形式を決める必要がある。形式は Phase 2 で決める
- ネットワーク越し（rfc2217 や serial MCP）に読む場合、192 KB/s を維持できるかは別途確かめる必要がある
