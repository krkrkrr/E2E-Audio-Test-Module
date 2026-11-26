---
type: Evidence
title: 2026-11-26 初回書き込み後の実機確認
description: e2e-audio-capture.uf2（a4e3fd2）を書き込んだ Pico を、serial MCP から CDC で操作した結果。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:55:00Z }
---

# 2026-11-26 初回書き込み後の実機確認

追記のみ。訂正は新しい行で書く。
ファーム: `a4e3fd2` のビルド。操作: serial MCP（ホスト Windows）から COM7 を 115200 8N1、DTR オンで開いた。

| # | 操作 | 観測 | 解釈 |
|---|---|---|---|
| 1 | ポート一覧 | COM7、VID:PID `1209:0001`、シリアル番号 `E2EAC` と表示。COM6（MicroPython）は消えた | CDC は列挙された。シリアル番号は `E2EAC-0001` を設定したが `E2EAC` と表示された。どこで切れたかは未確認 |
| 2 | `INFO\r\n` | INFO（110 B、CRC 一致）: `state=idle`、`stream=open`、`session=0`、`volume_8q8=0,0`、`mute=0,0` | リンクフレームの形式は ADR-0004 どおり。ホストは代替設定 1 を選んでいる |
| 3 | `START\r\n`、続けて `STOP\r\n` | STARTED(1)、ENDED(1, 0, 0) | セッションの一巡は動く |
| 4 | `START\r\n`、約 3 秒後に `INFO\r\n` | STARTED(2)、INFO: `state=recording`、`stream=open`、`position=0` | **代替設定 1 のまま、音声パケットが 1 つも届いていない** |
| 5 | `STOP\r\n` | ENDED(2, 0, 0) | |

この時点で、ホストで音を鳴らしていたかは確認していない。

## 追記: ホストで音を鳴らしながら（2026-11-26 09:50 UTC ごろ）

ユーザーが「E2E Audio Capture」を出力先にして音を鳴らした状態で、同じ手順を繰り返した。

| # | 操作 | 観測 | 解釈 |
|---|---|---|---|
| 6 | `START\r\n` の後、読み出しを数回 | 2 回の読み出しで 117,173 B と 197,604 B を受信。中身は L と R がほぼ同じ値の 16 bit LE 列 | 音声パケットが届き、DATA として流れている。上の #4 の `position=0` は、音を鳴らしていなかったためと考えられる |
| 7 | `STOP\r\n` | ENDED(3, total_frames=457,776, dropped_frames=0) | 約 9.5 秒分。**デバイス側の欠落は 0** |
| 8 | `START`、約 5 秒後に `INFO`、`STOP` | ENDED(4, total_frames=93,888, dropped_frames=0) | 約 2 秒分。欠落 0 |
| 9 | serial MCP の `serial_log_start` で受信を保存 | 保存されたファイルはテキスト化されていた（0x80 以上のバイトが `EF BF BD` に置換され、行頭にタイムスタンプ） | **serial MCP のログからは、バイナリのリンクフレームを復元できない**。ビット一致の検証には、ハーネス（`CaptureClient`）で COM7 を直接読む必要がある |

serial MCP の読み出しは出力を切り詰めるため、DATA の位置の連続性と CRC は検証していない。
#7 と #8 の欠落数は、デバイス自身が数えた値である。

## 追記: Windows 上のハーネスで直接読む（2026-11-26）

ユーザーが Windows の PowerShell で、`CaptureClient.open('COM7')` → `start()` → 5 秒待つ → `stop()` を実行した。音は鳴らしたまま。

| # | 操作 | 観測 | 解釈 |
|---|---|---|---|
| 10 | ハーネスで 5 秒録音 | `Ended(session_id=6, total_frames=241008, dropped_frames=0)`、gaps `[]`、CRC エラー 0 | 約 5.02 秒分。**デバイス側の欠落 0、受信側で検出した位置の飛び 0、CRC 不一致 0**。ハーネスの読み出しは 192 kB/s に追いついている |

## 追記: 実機 E2E テスト（2026-11-26）

ユーザーが Windows の PowerShell で `$env:E2E_AUDIO_PORT = "COM7"` を設定し、`pytest -m device test/e2e -v` を実行した。
他の音は止めた状態。Python 3.15.2、pytest 9.2.0、hypothesis 6.170.17。

| # | 操作 | 観測 | 解釈 |
|---|---|---|---|
| 11 | `test_p_e2e_played_signal_is_captured_bit_exact`（hypothesis、5 例） | PASSED（3.56 s）。sounddevice 由来の DeprecationWarning が 206 件 | **WASAPI 排他モードで鳴らした 0.1〜1 秒の乱数ステレオ信号が、5 例とも録音の中にビット一致で連続して現れた。** 同時に、保存則と欠落 0（P-E2E-2）も成り立った |

共有モードでは測っていない。
