---
type: Architecture Decision Record
title: "ADR-0005: フィーチャーファイルを cucumber-rs と pytest-bdd で実行する"
description: ファームの中核をハードウェア無しのシミュレーションとして cucumber-rs で、ハーネス側の復号を pytest-bdd で実行し、.feature を検査されない文書にしない。
status: draft
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:30:00Z }
---

# ADR-0005: フィーチャーファイルを cucumber-rs と pytest-bdd で実行する

## Status

Proposed

## Context

Phase 3 で 7 つの `.feature`（46 シナリオ）を書いた。
誰も実行しない `.feature` は、正しそうに見えるまま実装とずれていく。

シナリオの大半は、USB の代替設定、音声パケット、CDC のバイト列というファームへの入力と、CDC への出力で書ける。
これらはハードウェア無しで、ファームの中核（`capture-core`）を状態機械として駆動すれば再現できる。

一部のシナリオ（CRC 不一致の検出、途中からの同期）は、ハーネス側の復号器の振る舞いである。
復号器は Python で書くので、Rust からは実行できない。

実機と Windows の音声経路は、ビルド環境（Linux のサンドボックス）からは触れない（[Q-0002](../questions/open/environment/0002-flash-and-e2e-runner.md)）。

2026-11-26 の Phase 3 レビューで、ユーザーはこの方針で進めることに同意した。

## Decision

- `@harness` タグの付いていないシナリオは、cucumber-rs で `capture-core` のシミュレーション（`Device`）に対して実行する
- `@harness` タグのシナリオは、pytest-bdd で Python の復号器に対して実行する
- 実機と Windows の音声経路を通す検証は、フィーチャーではなく `test/e2e/` のプロパティベーステストで行う

## Consequences

- `.feature` が変われば、その日のうちにテストが落ちる
- ファームを「USB の配線」と「ハードウェアに依存しない状態機械」に分ける必要がある。配線側はフィーチャーで検査されない
- ステップ定義が Rust と Python の 2 つに分かれる。どちらで実行するかはタグだけで決まる
- 実機でしか分からない不具合（USB 記述子、embassy の挙動、タイミング）は E2E に任される
