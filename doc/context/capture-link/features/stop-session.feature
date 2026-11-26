@CAP-3
Feature: 録音セッションを停止する
  As テストハーネス
  I want to STOP を送って録音を終える
  So that セッションの範囲と、受け取るべきフレーム数と欠落数を確定できる

  Background:
    Given CDC が開かれている

  Scenario: Happy path — STOP 以前のフレームを送り終えてから ENDED が届く
    Given セッション 1 が Recording で、512 個のフレームを受けている
    And そのうち 256 個はまだ送られていない
    When ハーネスが "STOP\n" を送る
    Then 残りの 256 個が DATA で届く
    And その後に ENDED(session_id=1, total_frames=512, dropped_frames=0) が届く
    And 録音セッションは Idle に戻る

  Scenario: Business rule — STOP の後に届いたフレームは録音されない
    Given セッション 1 が Recording で、96 個のフレームを受けている
    When ハーネスが "STOP\n" を送る
    And audio-in が 48 個の音声フレームを公開する
    Then ENDED の total_frames は 96 である

  Scenario: Business rule — DATA のフレーム数と欠落数の和は total_frames に等しい
    Given セッション 1 が Recording で、バッファ溢れにより 48 フレームが欠落している
    When ハーネスが "STOP\n" を送り、ENDED まで読む
    Then DATA のフレーム数の合計 + dropped_frames = total_frames である
    And dropped_frames は 48 である

  Scenario: Boundary — START 直後の STOP は 0 フレームの ENDED になる
    Given セッション 1 を開始し、音声フレームはまだ届いていない
    When ハーネスが "STOP\n" を送る
    Then DATA は届かない
    And ENDED(session_id=1, total_frames=0, dropped_frames=0) が届く

  Scenario: Error — Idle で STOP を送ると拒否される
    Given 録音セッションは Idle である
    When ハーネスが "STOP\n" を送る
    Then ERROR(NotRecording=2) が返る

  Scenario: Business rule — 録音中に CDC が閉じられるとセッションは ENDED 無しで破棄される
    Given セッション 1 が Recording である
    When ハーネスが CDC を閉じる（DTR が下がる）
    And ハーネスが CDC を開き直して "START\n" を送る
    Then STARTED(session_id=2) が返る
    And 次に録音される音声フレームの位置は 0 である

  Scenario: Idempotency — STOP を 2 回続けて送っても ENDED は 1 回だけ
    Given セッション 1 が Recording である
    When ハーネスが "STOP\nSTOP\n" を 1 回で送る
    Then ENDED はちょうど 1 回届く
    And 2 回目の STOP には ERROR(NotRecording=2) が返る
