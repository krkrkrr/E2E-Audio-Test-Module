---
type: Question
title: "Q-0001: Windows のどの設定で、アプリのサンプル値が USB にそのまま届くか？"
description: 共有モードのミキサが音量やリサンプルで値を変えるため、E2E でビット一致を期待できる条件が分からない。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:17:22Z }
---

# Q-0001: Windows のどの設定で、アプリのサンプル値が USB にそのまま届くか？

**Status: `open`** — the directory this file sits in is the status.
**`git mv` it when that changes.** Do not write the status anywhere else.

## The question

ファームは USB で届いたサンプルをビット一致で返す。
しかし、アプリが出したサンプルと USB で届くサンプルが一致するかは、Windows のオーディオ経路で決まる。
E2E テストで「アプリが出した音」と「録音」をビット一致で比べられる条件はどれか。

## What it would take

- 既知の信号を排他モード（WASAPI exclusive）で鳴らし、録音とビット一致するかを測る
- 共有モードで、既定フォーマット 48 kHz / 16 bit、音量 100 %、オーディオ拡張を無効にした場合も同様に測る
- 結果を `doc/evidence/` に記録し、テストハーネスの許容範囲を決める

## What is known

- Windows の共有モードでは、ミキサが浮動小数点で合成し、音量を適用する
- デバイスが 48 kHz しか公開しなければ、アプリ側が 44.1 kHz で鳴らしたときはリサンプルが入る
- まだ何も測っていない
