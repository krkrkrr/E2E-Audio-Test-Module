@AUD-1
Feature: 音声ストリームを開く・閉じる
  As ホストのオーディオ経路
  I want to ストリーミングインタフェースの代替設定を切り替える
  So that 音声パケットを送ってよい期間をデバイスと合意できる

  Background:
    Given キャプチャデバイスが USB に接続され構成済みである

  Scenario: Happy path — 代替設定 1 でストリームが開き、0 で閉じる
    Given 音声ストリームは Closed である
    When ホストが代替設定 1 を選ぶ
    Then 音声ストリームは Open になる
    And StreamOpened が発行される
    When ホストが代替設定 0 を選ぶ
    Then 音声ストリームは Closed になる
    And StreamClosed が発行される

  Scenario: Boundary — 開いているストリームに再び代替設定 1 を選んでも Open のまま
    Given 音声ストリームは Open である
    When ホストが代替設定 1 を選ぶ
    Then 音声ストリームは Open のままである
    And それまでに受けた音声フレームの数は変わらない

  Scenario: Boundary — 閉じているストリームに代替設定 0 を選んでも Closed のまま
    Given 音声ストリームは Closed である
    When ホストが代替設定 0 を選ぶ
    Then 音声ストリームは Closed のままである

  Scenario: Error — 存在しない代替設定は拒否され状態は変わらない
    Given 音声ストリームは Open である
    When ホストが代替設定 2 を選ぶ
    Then その要求は拒否される
    And 音声ストリームは Open のままである

  Scenario: Business rule — USB リセットでストリームは閉じる
    Given 音声ストリームは Open である
    When USB がリセットされる
    Then 音声ストリームは Closed になる
    And StreamClosed が発行される

  Scenario: Concurrency — 閉じる前に届いたパケットは公開され、閉じた後のパケットは公開されない
    Given 音声ストリームは Open である
    And 48 フレーム分の音声パケットが届いている
    When ホストが代替設定 0 を選ぶ
    And その後に音声パケットが 1 つ届く
    Then 公開された音声フレームは 48 個である
