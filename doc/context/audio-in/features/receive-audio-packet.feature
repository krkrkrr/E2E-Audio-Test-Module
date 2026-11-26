@AUD-2
Feature: 音声パケットを受け、音声フレームとして公開する
  As ホストのオーディオ経路
  I want to 1 ms ごとに音声パケットを送る
  So that 送ったサンプルがそのまま capture-link に渡る

  Background:
    Given 音声ストリームは Open である

  Scenario: Happy path — 192 B のパケットは 48 個の音声フレームとして値を変えずに公開される
    When 48 フレーム分の任意のサンプルを含む 192 B のパケットが届く
    Then 48 個の音声フレームが公開される
    And 各フレームの L と R は、パケットのバイト列をリトルエンディアンで読んだ値と一致する

  Scenario Outline: Boundary — パケット長ごとの音声フレーム数
    When <bytes> B のパケットが届く
    Then <frames> 個の音声フレームが公開される
    And 端数バイトのあったパケット数は <partial> 増える
    Examples:
      | bytes | frames | partial |
      | 0     | 0      | 0       |
      | 4     | 1      | 0       |
      | 188   | 47     | 0       |
      | 192   | 48     | 0       |
      | 196   | 49     | 0       |
      | 190   | 47     | 1       |
      | 3     | 0      | 1       |

  Scenario Outline: Boundary — サンプルの極値はそのまま保たれる
    When L が <left>、R が <right> のフレームを含むパケットが届く
    Then 公開されたフレームの L は <left>、R は <right> である
    Examples:
      | left   | right  |
      | -32768 | 32767  |
      | 32767  | -32768 |
      | 0      | -1     |

  Scenario: Error — 閉じているストリームに届いたパケットは公開されない
    Given 音声ストリームは Closed である
    When 192 B のパケットが届く
    Then 音声フレームは公開されない
    And 受けた音声フレームの数は変わらない

  Scenario: Business rule — ホストが音量とミュートを変えてもサンプルは変わらない
    Given ホストが両チャンネルの音量を -20 dB に設定している
    And ホストが L チャンネルをミュートしている
    When 48 フレーム分の任意のサンプルを含むパケットが届く
    Then 公開された音声フレームは、パケットのバイト列と完全に一致する
    And ホスト制御値には音量 -20 dB と L のミュートが記録されている

  Scenario: Idempotency — 同じ内容のパケットが 2 回届けば 2 回公開される
    When 同じ 192 B のパケットが 2 回続けて届く
    Then 96 個の音声フレームが公開される
