---
type: Testing Strategy
title: E2E-Audio-Test-Module — テスト戦略
description: どの層を何で守るか。ファームの中核は Rust のシミュレーション、ハーネスは Python、実機は E2E で確かめる。
---

# テスト戦略

**全体に効く。コンテキスト別ではない。**

| 層 | 対象 | 道具 | 置き場所 | 実行場所 |
|---|---|---|---|---|
| 単体 | `capture-core` の各モジュール | `cargo test`（`#[test]`） | `test/unit/*.rs` | どこでも |
| 単体 | ハーネス（`host/e2e_audio`） | pytest | `test/unit/test_*.py` | どこでも |
| 文書 | `doc/` の型、リンク、図の大きさ、ID、見出し | pytest | `test/unit/test_documentation.py` | どこでも |
| ビルド | ファーム | `cargo build --release`（`firmware/`） | — | どこでも |
| 結合（プロパティ） | `capture-core` の `AudioStream` と `Device` | proptest | `test/integration/*.rs` | どこでも |
| 結合（プロパティ） | ハーネスの復号器と録音の再構成 | hypothesis | `test/integration/test_*.py` | どこでも |
| フィーチャー | `doc/context/*/features/*.feature`（`@harness` 以外） | cucumber-rs | `test/integration/features.rs` | どこでも |
| フィーチャー | `@harness` のシナリオ | pytest-bdd | `test/integration/test_harness_features.py` | どこでも |
| E2E（プロパティ） | 実機 + Windows の音声経路 | pytest + hypothesis | `test/e2e/test_*.py`（`-m device`） | Windows（[Q-0002](questions/answered/environment/0002-flash-and-e2e-runner.md)） |

## プロパティベーステストの道具

- Rust: **proptest**。縮小（shrinking）は既定で有効
- Python: **hypothesis**

## 二つの言語が同じ形式を話すことの確かめ方

リンクフレームの形式（[ADR-0004](ADR/0004-frame-cdc-link-with-sync-and-crc.md)）は、`test/fixtures/link_vectors.json` の試験ベクタで固定する。
Rust の符号化と Python の符号化・復号の両方が、同じベクタを満たすことを検査する（P-CAP-4）。

## 実行

```sh
cargo test                                          # capture-core の単体・結合・フィーチャー
uv run --project host --extra test pytest           # ハーネス・文書・@harness（実機テストは skip）
```

実機 E2E は Windows で `E2E_AUDIO_PORT` を設定して `pytest -m device test/e2e` を実行する。

## 守られないもの

USB 記述子、embassy-usb の挙動、タイミングは、シミュレーションでは検査されない（[ADR-0005](ADR/0005-execute-features-with-cucumber-rs-and-pytest-bdd.md)）。
これらは E2E に任される。
