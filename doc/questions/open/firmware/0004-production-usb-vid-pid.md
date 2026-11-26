---
type: Question
title: "Q-0004: 配布するファームには、どの USB VID/PID を使うか？"
description: 開発中は pid.codes の試験用 1209:0001 を使っているが、これは私的な試験にしか使えない。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:45:00Z }
---

# Q-0004: 配布するファームには、どの USB VID/PID を使うか？

**Status: `open`** — the directory this file sits in is the status.
**`git mv` it when that changes.** Do not write the status anywhere else.

## The question

`firmware/src/main.rs` は VID/PID に pid.codes の試験用 `1209:0001` を使っている。
これは私的な試験のための番号で、複数の人に配る機器には使えない。
Windows は VID/PID ごとにドライバとデバイス名を覚えるので、同じ番号の別機器と衝突すると混乱する。

## What it would take

- 社内で配るだけなら、所属組織の VID と空いている PID を使えるか確かめる
- オープンソースとして公開するなら、pid.codes か Raspberry Pi（2E8A）に PID を申請する

## What is known

- 試験用 `1209:0001` で開発を進めることは、この増分の Phase 5 で決めた
- シリアル番号は固定の `E2EAC-0001`。複数台を同時に挿すなら、RP2040 のフラッシュの一意な ID に替える必要がある
