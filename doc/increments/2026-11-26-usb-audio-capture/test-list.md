---
type: Test List
title: usb-audio-capture — テストリスト
description: Phase 5 の単体テストの一覧。上から順に Red → Green → Refactor で潰す。
---

# usb-audio-capture — テストリスト

Rust の単体テストは `test/unit/<module>.rs`、Python は `test/unit/test_<module>.py` に置く。

## capture-core: audio_in

- [x] 音声フレームは L, R の順に i16 LE の 4 バイトになり、そこから戻せる — unit
- [x] 新しいストリームは Closed で、代替設定 1 で Open、0 で Closed になる — unit
- [x] 代替設定 2 以上は拒否され、状態は変わらない — unit
- [x] 192 B のパケットは 48 フレームになり、受けたフレーム数が 48 増える — unit
- [x] 端数バイトは捨てられ、端数パケット数が 1 増える — unit
- [x] Closed のストリームはパケットを受けてもフレームを出さない — unit
- [x] フィードバックは Closed で None、Open で 00 00 0C — unit
- [x] ホスト制御値は記録され、読み出せる — unit
- [x] USB リセットでストリームは Closed になる — unit

## capture-core: link

- [x] CRC-16/CCITT-FALSE の check 値は 0x29B1 — unit
- [x] encode は同期語、種別、長さ、ペイロード、CRC の順に書く — unit
- [x] 行読み取り: START / STOP / INFO を認識する — unit
- [x] 行読み取り: `\r` は無視、空行は何も返さない — unit
- [x] 行読み取り: 不明な行は UnknownCommand — unit
- [x] 行読み取り: 32 バイトまでは解釈し、33 バイト以上は LineTooLong — unit
- [x] 行読み取り: 行が複数回に分かれて届いても 1 行として解釈する — unit

## capture-core: session

- [x] 新しいセッションは Idle で id 0 — unit
- [x] start で Recording になり、id が 1 ずつ増える — unit
- [x] Idle で受けたフレームは数えない — unit
- [x] Recording で受けたフレームは位置 0 から取り出せる — unit
- [x] take は 1 回に最大 256 フレーム — unit
- [x] バッファ満杯では新しい側を捨て、欠落数と位置が進む — unit
- [x] 欠落の後に受けたフレームは、飛んだ位置から取り出せる — unit
- [x] stop で Stopping になり、以後のフレームは受けない — unit
- [x] Stopping でバッファが空になると ENDED の値を返して Idle になる — unit
- [x] abort で何も返さず Idle に戻り、バッファを捨てる — unit

## capture-core: device

- [x] Idle で START を受けると STARTED(1) を送る — unit
- [x] Recording で START を受けると ERROR(1)、Idle で STOP を受けると ERROR(2) — unit
- [x] 応答は DATA より先に送る — unit
- [x] 録音中に受けた音声パケットは DATA として送る — unit
- [x] STOP の後、残りの DATA を送ってから ENDED を送る — unit
- [x] INFO は状態、フォーマット、位置、欠落、ホスト制御値を key=value で返す — unit
- [x] DTR が下がるとセッションを破棄し、送信待ちを捨てる — unit
- [x] USB リセットでストリームを閉じ、セッションを破棄する — unit

## host: e2e_audio.link

- [x] crc16 の check 値は 0x29B1 — unit
- [x] 各種別のフレームを encode / decode できる — unit
- [x] 前置のゴミを読み飛ばす — unit
- [x] CRC 不一致を数えて読み飛ばす — unit
- [x] 長さが大きすぎる偽の見出しで止まらず、後続の完全なフレームを優先する — unit
- [x] Recording: 位置の飛びを欠落として記録し、PCM を組み立てる — unit

## host: e2e_audio.client

- [x] START を送り STARTED を待つ — unit（偽のシリアル）
- [x] STOP を送り ENDED まで読んで Recording を返す — unit（偽のシリアル）
- [x] ERROR が返れば例外にする — unit（偽のシリアル）

## host: e2e_audio.player

- [x] 名前に "E2E Audio Capture" を含む WASAPI 出力を選ぶ。無ければ例外 — unit（偽の sounddevice）

## 文書

- [x] `doc/` の Markdown はすべて OKF の `type` を持つ — unit
- [x] `doc/` の相対リンクはすべて実在するファイルを指す — unit
- [x] Mermaid 図は 40 行以下、7 クラス以下 — unit
- [x] ユースケース ID はコンテキストの接頭辞（AUD- / CAP-）を持つ — unit
- [x] `domain-model.md` / `object-model.md` / `constraints.md` は決まった見出しを持つ — unit

## firmware（ビルドのみ）

- [x] `thumbv6m-none-eabi` でビルドでき、UF2 を生成できる — build
