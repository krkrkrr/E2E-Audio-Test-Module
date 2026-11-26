---
type: Object Model
title: capture-link — オブジェクトモデル
description: capture-link が担うユースケース CAP-1〜4 と、そのやりとり。
---

# capture-link — オブジェクトモデル

正本: [system-context.md](../../system-context.md)、[use-cases.md](../../use-cases.md)。

## この文脈が担うユースケース

| ID | ユースケース |
|---|---|
| CAP-1 | 録音セッションを開始する |
| CAP-2 | 録音データを受け取る |
| CAP-3 | 録音セッションを停止する |
| CAP-4 | デバイス情報を問い合わせる |

## やりとり

### CAP-1 録音セッションを開始する

```mermaid
sequenceDiagram
  participant H as テストハーネス
  participant CDC as cdc_link（firmware）
  participant S as CaptureSession
  H->>CDC: "START\n"
  CDC->>S: start()
  alt Idle
    S-->>CDC: SessionStarted(id)
    CDC-->>H: STARTED(id)
  else Recording / Stopping
    S-->>CDC: AlreadyRecording
    CDC-->>H: ERROR(1)
  end
```

### CAP-2 録音データを受け取る

```mermaid
sequenceDiagram
  participant A as audio-in
  participant S as CaptureSession
  participant CDC as cdc_link（firmware）
  participant H as テストハーネス
  A->>S: accept(48 frames)
  alt バッファに空きがある
    S->>S: push、nextPosition += 48
  else 満杯
    S->>S: 入らない分を捨てて droppedFrames に足す
  end
  CDC->>S: take(256)
  S-->>CDC: DataChunk(position, frames)
  CDC-->>H: DATA(position, frames)
  H->>H: 前の DATA との位置の差で欠落を検出
```

### CAP-3 録音セッションを停止する

```mermaid
sequenceDiagram
  participant H as テストハーネス
  participant CDC as cdc_link（firmware）
  participant S as CaptureSession
  H->>CDC: "STOP\n"
  CDC->>S: stop()
  alt Recording
    S->>S: Stopping へ。以後の音声フレームは受けない
    loop バッファが空になるまで
      CDC->>S: take(256)
      CDC-->>H: DATA(position, frames)
    end
    S-->>CDC: SessionEnded(id, total, dropped)
    CDC-->>H: ENDED(id, total, dropped)
  else Idle
    CDC-->>H: ERROR(2)
  end
```

### CAP-4 デバイス情報を問い合わせる

```mermaid
sequenceDiagram
  participant H as テストハーネス
  participant CDC as cdc_link（firmware）
  participant S as CaptureSession
  participant A as audio-in
  H->>CDC: "INFO\n"
  CDC->>S: 状態、id、位置、欠落数
  CDC->>A: フォーマット、ストリーム状態、HostControls
  CDC-->>H: INFO("version=…\nformat=48000/16/2\n…")
```

INFO は録音中でも受け付ける。DATA の間に挟まって届く。

## Review Notes

### 2026-11-26 第 1 回

ユーザーがモデルを承認した。提示した判断点は既定案で確定した。

- ホスト側の途切れ（パケットが来ない区間）の検出は今回の範囲に含めない。USB フレーム番号（SOF）は DATA に載せない
- 録音中の START は ERROR(AlreadyRecording) を返し、進行中のセッションは続ける
- INFO（CAP-4）は残す
- ADR-0003、ADR-0004 は Accepted
