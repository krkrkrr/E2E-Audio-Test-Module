@CAP-1
Feature: 録音セッションを開始する
  As テストハーネス
  I want to START を送って録音を始める
  So that 検証したい範囲の音声だけを受け取れる

  Background:
    Given CDC が開かれている

  Scenario: Happy path — Idle で START を送ると STARTED が返り、以後のフレームが位置 0 から録音される
    Given 録音セッションは Idle である
    And これまでに開始したセッションは無い
    When ハーネスが "START\n" を送る
    Then STARTED(session_id=1) が返る
    When audio-in が 48 個の音声フレームを公開する
    Then その先頭のフレームの位置は 0 である

  Scenario: Happy path — セッション ID はセッションごとに 1 ずつ増える
    Given セッション 1 を開始して停止した
    When ハーネスが "START\n" を送る
    Then STARTED(session_id=2) が返る

  Scenario: Business rule — START より前に届いたフレームは録音されない
    Given 録音セッションは Idle である
    And audio-in が 96 個の音声フレームを公開した
    When ハーネスが "START\n" を送る
    And audio-in が 48 個の音声フレームを公開する
    Then 録音される音声フレームは後の 48 個だけである

  Scenario Outline: Boundary — コマンド行の形
    Given 録音セッションは Idle である
    When ハーネスが <line> を送る
    Then <response> が返る
    Examples:
      | line                       | response                  |
      | "START\r\n"                | STARTED                   |
      | "start\n"                  | ERROR(UnknownCommand=3)   |
      | "START now\n"              | ERROR(UnknownCommand=3)   |
      | 32 バイトちょうどの不明な行 | ERROR(UnknownCommand=3)   |
      | 33 バイトの行              | ERROR(LineTooLong=4)      |

  Scenario: Boundary — 空行は無視され何も返らない
    When ハーネスが "\n" を送る
    Then リンクフレームは返らない

  Scenario: Boundary — 複数の USB パケットに分かれたコマンドも 1 行として解釈される
    Given 録音セッションは Idle である
    When ハーネスが "STA" を送り、続けて "RT\n" を送る
    Then STARTED が返る

  Scenario: Error — 録音中の START は拒否され、進行中のセッションは続く
    Given セッション 1 が Recording で、位置 480 まで録音している
    When ハーネスが "START\n" を送る
    Then ERROR(AlreadyRecording=1) が返る
    And セッション ID は 1 のままである
    And 次に録音される音声フレームの位置は 480 である

  Scenario: Error — 停止処理中の START は拒否される
    Given セッション 1 が Stopping である
    When ハーネスが "START\n" を送る
    Then ERROR(AlreadyRecording=1) が返る
    And セッション 1 の ENDED は通常どおり送られる

  Scenario: Concurrency — 1 回の書き込みに START が 2 行入っていれば、2 行目は拒否される
    Given 録音セッションは Idle である
    When ハーネスが "START\nSTART\n" を 1 回で送る
    Then STARTED(session_id=1) と ERROR(AlreadyRecording=1) がこの順で返る
