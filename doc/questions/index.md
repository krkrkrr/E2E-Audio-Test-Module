# open

* [Q-0001: Windows のどの設定で、アプリのサンプル値が USB にそのまま届くか？](open/host-audio/0001-bit-exact-host-path.md) - host-audio: 共有モードのミキサが音量やリサンプルで値を変えるため、E2E でビット一致を期待できる条件が分からない。
* [Q-0002: UF2 の書き込みと E2E テストを、どこから誰が実行するか？](open/environment/0002-flash-and-e2e-runner.md) - environment: ビルドは Linux のサンドボックス、Pico と音声出力は Windows ホスト側にあり、書き込みと E2E の実行経路が決まっていない。
* [Q-0003: キャプチャバッファ 16,384 フレーム（約 341 ms）で、ホストの読み出しの揺らぎを吸収できるか？](open/capture-link/0003-capture-buffer-size.md) - capture-link: バッファ容量は RAM の都合で選んだ値で、Windows 上のハーネスが CDC を読み遅れる最大時間を測っていない。

