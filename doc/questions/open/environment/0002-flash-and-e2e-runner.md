---
type: Question
title: "Q-0002: UF2 の書き込みと E2E テストを、どこから誰が実行するか？"
description: ビルドは Linux のサンドボックス、Pico と音声出力は Windows ホスト側にあり、書き込みと E2E の実行経路が決まっていない。
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-26T09:17:22Z }
---

# Q-0002: UF2 の書き込みと E2E テストを、どこから誰が実行するか？

**Status: `open`** — the directory this file sits in is the status.
**`git mv` it when that changes.** Do not write the status anywhere else.

## The question

ファームのビルドは Linux のサンドボックスで行える。
しかし Pico は Windows ホストに USB で挿さっており、BOOTSEL モードのドライブもホスト側に現れる。
音を鳴らすのもホストである。

UF2 の書き込みと、音を鳴らして録音を確かめる E2E テストを、どこから実行するのか。

## What it would take

- BOOTSEL への切り替え手段を決める（BOOTSEL ボタン、MicroPython の `machine.bootloader()`、ファームに CDC コマンドを持たせる）
- UF2 をホストのドライブへコピーする手段を決める（ユーザーの手作業、ホスト側スクリプト、MCP）
- E2E テストを Windows 上の Python で実行するか、サンドボックスから遠隔で駆動できるかを確かめる

## What is known

- 作業ディレクトリ `/c/Users/mizak/workspace/E2E-Audio-Test-Module` はホストの `C:\Users\mizak\workspace\...` と同じ木に見える。ここに UF2 を置けばホストから見える可能性が高い（未確認）
- serial MCP はホストの COM ポート（COM3〜COM6）を扱える（[証拠](../../../evidence/2026-11-26-serial-ports.md)）
- サンドボックスから Windows の音声デバイスを直接鳴らす手段は確認していない
- 2026-11-26: サンドボックスでファームをビルドし、`firmware/dist/e2e-audio-capture.uf2` を作れた。このファイルは作業ディレクトリにあり、ホストの `C:\Users\mizak\workspace\E2E-Audio-Test-Module\firmware\dist\` からも見える前提。書き込みは、Pico を BOOTSEL で挿して現れるドライブ（RPI-RP2）へのコピーで行う
- 書き込み後の Pico は MicroPython ではなくなるので、`machine.bootloader()` で BOOTSEL に入る手段は失われる。以後は BOOTSEL ボタンを押しながら挿す
