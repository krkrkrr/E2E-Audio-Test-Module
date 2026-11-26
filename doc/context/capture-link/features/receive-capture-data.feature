@CAP-2
Feature: 録音データを受け取る
  As テストハーネス
  I want to 録音中の音声フレームを DATA で受け取る
  So that USB Audio で届いたサンプル列をビット一致で再構成し、欠落を検出できる

  Background:
    Given CDC が開かれている
    And 録音セッションが Recording である

  Scenario: Happy path — 受け取ったフレームを連結すると audio-in が公開した列と一致する
    When audio-in が任意のサンプルの音声フレームを 480 個公開する
    And ハーネスがすべての DATA を読む
    Then DATA の位置は 0 から欠けずに続いている
    And DATA の音声フレームを連結した列は、公開された 480 個とバイト単位で一致する

  Scenario Outline: Boundary — 1 つの DATA は最大 256 フレーム
    Given CDC がしばらく読まれず、<n> 個のフレームがバッファに溜まっている
    When ハーネスが読み出しを再開する
    Then DATA は <chunks> 個に分かれて届く
    And どの DATA も 1〜256 個のフレームを持つ
    Examples:
      | n   | chunks |
      | 1   | 1      |
      | 256 | 1      |
      | 257 | 2      |
      | 512 | 2      |

  Scenario: Boundary — バッファ容量ちょうどまでは欠落しない
    Given CDC が読まれていない
    When audio-in が 16384 個の音声フレームを公開する
    And ハーネスが読み出しを再開する
    Then 位置 0〜16383 がすべて届く
    And 欠落は 0 である

  Scenario: Business rule — バッファが満杯なら新しい側を捨て、位置の飛びで欠落を伝える
    Given CDC が読まれていない
    When audio-in が 16384 個の音声フレームを公開する
    And audio-in がさらに 48 個の音声フレームを公開する
    And ハーネスが読み出しを再開する
    And audio-in がさらに 48 個の音声フレームを公開する
    Then 位置 0〜16383 のフレームは元の値のまま届く
    And 次の DATA の位置は 16432 である
    And ハーネスは欠落 48 フレームを検出する

  Scenario: Business rule — ストリームが閉じている間は DATA が来ないが、セッションは続く
    Given 位置 96 まで録音している
    When audio-in のストリームが閉じ、しばらくして再び開く
    And audio-in が 48 個の音声フレームを公開する
    Then 次の DATA の位置は 96 である
    And 欠落は 0 である

  @harness
  Scenario: Error — CRC が合わないリンクフレームはハーネスで捨てられ、位置の飛びとして検出される
    Given ハーネスが受けた DATA のうち 1 つのペイロードが 1 ビット壊れている
    When ハーネスがリンクフレームを復号する
    Then 壊れたフレームは CRC 不一致として捨てられる
    And 後続の DATA は正しく復号される
    And その区間は欠落として報告される

  @harness
  Scenario: Error — 途中から読み始めても、同期語と CRC で同期を取り直せる
    Given CDC の受信列の先頭に、無関係なバイト列と DATA の後半が残っている
    When ハーネスがリンクフレームを復号する
    Then 最初の完全なリンクフレームから正しく復号される

  Scenario: Concurrency — 録音中の INFO は DATA の間に挟まるが、位置の連続を壊さない
    When audio-in が音声フレームを公開し続けている間にハーネスが "INFO\n" を送る
    Then INFO が DATA と DATA の間に 1 つ届く
    And DATA の位置は欠けずに続いている
