---
type: Requirements
title: usb-audio-capture — 要件
description: RP2040 を USB Audio 出力先として見せ、受けた音声をビット一致のまま USB CDC でホストへ返すファームの要件（2026-11-26 のヒアリング）。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:17:22Z }
---

# usb-audio-capture — 要件

2026-11-26 のヒアリング結果。
この増分が始まった後は凍結する。
規則は Phase 2 で各コンテキストの `constraints.md` へ、受け入れ基準は Phase 3 の `.feature` へ移す。

## Phase 0 の判定

この時点のリポジトリには `doc/` がなく、bounded context は一つも存在しなかった。
そのため、既存コンテキストへの振り分けはできない。
コンテキストは Phase 2 で S 図（ファームが接続する相手）を描き、そこから数える。

## ドメインと文脈

### 解く問題

ホスト PC 上のアプリが、期待どおりの音を出しているかを E2E で検証したい。
そのために、アプリの出力先になり、出された音をそのまま記録する計測器が要る。

### 利用者（アクター）

- **テスト対象アプリ**（ホスト PC 上）: 音を出す。RP2040 を出力デバイスとして選ぶ
- **テストハーネス**（ホスト PC 上の Python）: 録音の開始と停止を指示し、録音データを受け取って検証する
- **ホスト OS のオーディオ経路**（Windows）: アプリの音を USB Audio のストリームとして RP2040 へ送る

### 中核ドメイン

USB Audio で受けたサンプル列を、欠落も加工もなくホストへ返す「録音」。
USB Audio デバイスとしての振る舞い、CDC での転送は、それを支える部分である。

### 業務規則と制約

1. RP2040 は USB Audio Class 1.0 の出力デバイス（スピーカ）としてホストに見える
2. フォーマットは 48 kHz / 16 bit / 2 ch に固定する（192,000 B/s）
3. USB Audio と CDC を 1 本の USB にまとめた複合デバイスにする
4. 録音はデバイス内に貯めず、CDC でリアルタイムにホストへ流す。録音時間に上限を設けない
5. 録音の範囲は、ホストが CDC で送る START と STOP で決める
6. ファームは受けたサンプル値を一切加工しない（音量、ミュート、リサンプルを適用しない）
7. 転送データには連番を付け、デバイス側で欠落が起きたらその事実と量をホストが知れるようにする

### 受け入れ基準

- USB Audio で届いたサンプル列と、CDC で取り出したサンプル列がビット一致する
- 欠落が起きた場合、それが黙って失われず、ホスト側で検出できる
- START から STOP までの範囲が、ホストのコマンドで決定的に定まる

OS のミキサがサンプル値を変えることは、ファームの責任外とする。
その差はテストハーネス側の許容範囲として扱う（Q-0001）。

## 技術的文脈

| 項目 | 内容 |
|---|---|
| ボード | Raspberry Pi Pico（RP2040）。現在 COM6 に MicroPython で接続されている個体 |
| ファーム | Rust + embassy（embassy-rp、embassy-usb の UAC1 Speaker と CDC ACM）[ADR-0001](../../ADR/0001-write-firmware-in-rust-with-embassy.md) |
| 録音の経路 | USB CDC へのリアルタイム転送 [ADR-0002](../../ADR/0002-stream-capture-over-usb-cdc.md) |
| ホスト | Windows PC |
| テストハーネス | Python（pyserial、sounddevice、pytest、hypothesis） |
| 既存システム | `rfc2217_server.py`（シリアルを TCP へ中継）、serial MCP |

### 非機能要件

- 48 kHz / 2 ch / 16 bit の連続ストリームで、デバイス側の欠落を起こさない
- USB Full Speed（12 Mbps）の上で、Audio（約 192 KB/s）と CDC（約 192 KB/s + ヘッダ）を同時に流す
- RP2040 の RAM（264 KB）に収まる。デバイス側のバッファはホストの読み出しの揺らぎを吸収できる量にする

ヒアリングでは性能、セキュリティ、可用性の追加要件は挙がらなかった。

## 未解決

- [Q-0001](../../questions/answered/host-audio/0001-bit-exact-host-path.md): Windows のどの設定でアプリのサンプル値が USB にそのまま届くか
- [Q-0002](../../questions/answered/environment/0002-flash-and-e2e-runner.md): UF2 の書き込みと E2E テストを、どこから誰が実行するか
