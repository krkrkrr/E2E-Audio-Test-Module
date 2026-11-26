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

各行は、それを守るテストへリンクする。
`Device` はファームの中核の状態機械（ハードウェア無しのシミュレーション）、復号器はハーネス側の Python 実装である。

### リンクフレーム（CAP-1〜4 共通）

| ID | プロパティ | 種別 | 表現 | テスト |
|---|---|---|---|---|
| P-CAP-1 | 符号化と復号は往復する | ラウンドトリップ | `decode(encode(k, payload)) == (k, payload)` | [test_link_decoder.py](../../../test/integration/test_link_decoder.py) |
| P-CAP-2 | 1 ビットの破損は必ず検出される | 否定 | `∀ 1 ビット反転: decode は元のフレームを返さない` | test_link_decoder.py |
| P-CAP-3 | 前置のゴミがあっても、後続のフレームは復号できる | 同値 | `decode(g ++ encode(fs))` の末尾が `fs` に一致する | test_link_decoder.py |
| P-CAP-4 | Rust の符号化と Python の復号は同じ形式を話す | 同値 | 共通の試験ベクタ `link_vectors.json` を両方が満たす | [capture_link.rs](../../../test/integration/capture_link.rs)、test_link_decoder.py |

### CAP-1〜3 録音セッション（状態を持つモデルとの比較）

| ID | プロパティ | 種別 | 表現 | テスト |
|---|---|---|---|---|
| P-CAP-5 | 保存則 | 不変量 | `Σ DATA のフレーム数 + dropped_frames == total_frames` | [capture_link.rs](../../../test/integration/capture_link.rs) |
| P-CAP-6 | 位置は欠けずに進み、飛びは欠落数に等しい | 単調 | `pos(next) == pos(prev) + n(prev) + 欠落(間)` | capture_link.rs |
| P-CAP-7 | 届いたフレームは、その位置で受けたフレームとビット一致する | 同値 | `∀ k ∈ 届いた位置: frame(k) == accepted[k]` | capture_link.rs |
| P-CAP-8 | 満杯のときは新しい側を捨てる | 不変量 | 読まれない間に n 個受けたら、届くのは先頭の `min(n, 16384)` 個 | capture_link.rs |
| P-CAP-9 | DATA は 1〜256 フレーム | 不変量 | `1 ≤ n(DATA) ≤ 256` | capture_link.rs |
| P-CAP-10 | セッションごとに STARTED は 1 回、ENDED は高々 1 回で、そのセッションの DATA より後 | 不変量 | 出力列をセッションで区切ったときの形 | capture_link.rs |
| P-CAP-11 | Idle 中のフレームはどのセッションにも入らない | 否定 | `total_frames` は Recording 中に受けたフレーム数に等しい | capture_link.rs |
| P-CAP-12 | コマンドの分割位置は応答を変えない | 同値 | `∀ 分割: responses(chunks(cmds)) == responses(cmds)` | capture_link.rs |
| P-CAP-13 | INFO は録音結果を変えない | 同値 | INFO を任意の位置に挟んでも、INFO 以外の出力列は同じ | capture_link.rs |

### 実機（E2E）

| ID | プロパティ | 種別 | 表現 | テスト |
|---|---|---|---|---|
| P-E2E-1 | 実機で WASAPI 排他モードで鳴らした信号と録音がビット一致する | 同値 | 先頭の無音を除いて `captured == played`（[Q-0001](../../questions/answered/host-audio/0001-bit-exact-host-path.md) の条件下） | [test_capture_e2e.py](../../../test/e2e/test_capture_e2e.py) |
| P-E2E-2 | 実機の録音で保存則が成り立ち、欠落が 0 | 不変量 | P-CAP-5 と `dropped_frames == 0` | test_capture_e2e.py |
