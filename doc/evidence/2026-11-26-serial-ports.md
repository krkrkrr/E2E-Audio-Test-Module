---
type: Evidence
title: 2026-11-26 ホストのシリアルポート一覧
description: serial MCP の list_serial_ports で見えたホストの COM ポートと USB VID/PID。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:17:22Z }
---

# 2026-11-26 ホストのシリアルポート一覧

追記のみ。訂正は新しい行で書く。

| 日時 (UTC) | ポート | VID:PID | 製品名 | シリアル番号 | 解釈 |
|---|---|---|---|---|---|
| 2026-11-26 | COM3 | 514B:4D02 | USB シリアル デバイス (COM3) | 000000000000 | 不明な CDC デバイス |
| 2026-11-26 | COM4 | — | — | — | USB 情報なし |
| 2026-11-26 | COM5 | — | — | — | USB 情報なし |
| 2026-11-26 | COM6 | 2E8A:0005 | USB シリアル デバイス (COM6) | 5044466238B08F1C | Raspberry Pi RP2040 + MicroPython。この増分の対象 |

製造元はすべて「Microsoft」と表示された。これは Windows 標準の `usbser.sys` を指し、ハードウェアの製造元ではない。
