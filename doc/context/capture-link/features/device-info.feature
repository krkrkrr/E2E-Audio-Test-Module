@CAP-4
Feature: デバイス情報を問い合わせる
  As テストハーネス
  I want to INFO でデバイスの状態を知る
  So that E2E の前提（フォーマット、ホスト制御値、録音状態）を確かめてから検証できる

  Background:
    Given CDC が開かれている

  Scenario: Happy path — Idle での INFO
    Given 録音セッションは Idle である
    And 音声ストリームは Closed である
    When ハーネスが "INFO\n" を送る
    Then INFO が返る
    And INFO には version、format=48000/16/2、state=idle、stream=closed が含まれる

  Scenario: Happy path — 録音中の INFO はセッションの進み具合を含む
    Given セッション 3 が Recording で、位置 16432 まで受け、欠落は 48 である
    When ハーネスが "INFO\n" を送る
    Then INFO には state=recording、session=3、position=16432、dropped=48 が含まれる

  Scenario: Business rule — INFO はホスト制御値を報告する
    Given ホストが両チャンネルの音量を -20 dB に設定し、R をミュートしている
    When ハーネスが "INFO\n" を送る
    Then INFO には L と R の音量 -20 dB と、R のミュートが含まれる

  Scenario: Error — 引数付きの INFO は不明なコマンドとして拒否される
    When ハーネスが "INFO all\n" を送る
    Then ERROR(UnknownCommand=3) が返る

  Scenario: Idempotency — INFO は状態を変えない
    Given セッション 1 が Recording である
    When ハーネスが "INFO\n" を 2 回送る
    Then 2 つの INFO の state と session は同じである
    And 録音セッションは Recording のままである
