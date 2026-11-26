@AUD-3
Feature: 送出レートのフィードバックを返す
  As ホストのオーディオ経路
  I want to デバイスが受け入れたいレートを知る
  So that 1 パケットあたりのフレーム数を調整できる

  Scenario: Happy path — フィードバック値は 48.0 kHz を表す 10.14 形式
    Given 音声ストリームは Open である
    When ホストがフィードバックを要求する
    Then 返る 3 バイトは 00 00 0C である

  Scenario: Boundary — キャプチャバッファが満杯でもフィードバック値は変わらない
    Given 音声ストリームは Open である
    And 録音セッション中でキャプチャバッファが満杯である
    When ホストがフィードバックを要求する
    Then 返る 3 バイトは 00 00 0C である

  Scenario: Error — 閉じているストリームではフィードバックを返さない
    Given 音声ストリームは Closed である
    When ホストがフィードバックを要求する
    Then フィードバックは返らない

  Scenario: Business rule — フィードバック値はホスト制御値に影響されない
    Given 音声ストリームは Open である
    And ホストが両チャンネルをミュートしている
    When ホストがフィードバックを要求する
    Then 返る 3 バイトは 00 00 0C である

  Scenario: Idempotency — 何度要求しても同じ値が返る
    Given 音声ストリームは Open である
    When ホストがフィードバックを 1000 回要求する
    Then すべての応答は 00 00 0C である
