# 検証記録

## v0.3.0 — 2026-09-17

ライブ操作表示、固定色の再生／録音ランプ、CC0音源の常駐ピアノを追加しました。ローカルで Rust **39 passed / 1 ignored**（外部通信テスト）、Vitest **15 passed**、Playwright / Edge **11 passed**、TypeScript + Vite、Clippy（全target、警告なし）、書式検査が通過しました。依存ライセンス309件と音源のCC0原文を同梱しています。

分離したテスト用TauriプロセスをMock MIDIで動かし、Windowsの実際のWASAPI音声出力を確認しました。発音／減衰、CC64サステイン、全音停止、照明停止中の演奏、画面鍵盤の解放順序、出力先不在の表示と復帰、DAW検出時の消音、入力表示、照明停止・再開時のポート別状態維持を対象とします。ウィンドウを閉じて再表示する間も、演奏・押鍵状態を保持することを確認しました。

音声出力は48 kHzで動作しました。この環境では256フレーム指定からドライバーの既定値へフォールバックし、定常時のコールバックは480フレームでした。これはバッファ量の観測であり、鍵盤からスピーカーまでの実測遅延ではありません。実機Launchkey／物理ペダル、DAWごとの共有、スリープ復帰、長時間演奏、インストーラー実行、コード署名は未検証です。ASIO、半踏み、VSTホストには対応しません。

| SHA       | 件名                                                                | 目的                                               |
| --------- | ------------------------------------------------------------------- | -------------------------------------------------- |
| `5724d5e` | feat(device): mirror live controls and fixed transport lamp colors  | つまみ・フェーダー等のライブ表示と単色ランプの再現 |
| `beaf99e` | feat(piano): bundle a CC0 acoustic piano with sustain and transpose | 音源、サステイン、移調、音声合成の基礎             |
| `6788205` | feat(piano): add resident audio playback and performance controls   | Windows音声出力、独立MIDI入力、演奏UI、常駐        |
| `4cba453` | feat(device): show touch and Arp/Scale feedback                     | Touch・圧力・機能状態の表示                        |
| `a6df623` | fix(piano): retain paused input state and clean up the audio worker | 停止中の画面入力と音声スレッド終了の修正           |
| `a0fddc9` | fix(device): preserve keyboard state when releasing the DAW port    | 照明停止・再接続時の鍵盤／ペダル状態の保持         |

公開パイプラインは上記の自動チェックを再実行し、配布用バイナリで既存のMock自己テスト7項目を検証します。[v0.3.0 Release](https://github.com/lingmulongtai/Keylume/releases/tag/v0.3.0) の `verification.json`、`build-provenance.json`、`SHA256SUMS` で結果・ビルド元・ファイルハッシュを確認できます。[ピアノの使い方](piano.md)

## v0.2.0 — 2026-09-17

61鍵の本体図、編集UI、更新確認を変更しました。ローカルで Rust 33テスト、Vitest 11テスト、Playwright / Edge 8テスト、TypeScript + Vite、Clippy（全target、警告なし）、書式検査が通過しました。明示実行した WinHTTP → GitHub Releases の実通信テストも1件通過しています。通常のテストでは外部通信を実行しません。

追加テストは旧配置からの移行とバックアップ、校正値・カスタム配置の保持、61鍵の相対配置、クリック領域と本体図の一致、200%拡大時の操作、SemVer比較・Preview版選択・不正リリース除外、通知の抑制、手動要求の合流、更新UIのエラーと再試行を対象とします。1280×800の本体図を目視し、最小1024×680でも全画面を操作確認しました。

| SHA | 件名 | 目的 |
| --- | --- | --- |
| `f949777` | feat(ui): replace decorative chrome with a compact editing workspace | 上部ナビゲーションと簡潔な編集UI |
| `cb1bafb` | fix(dev): exclude generated artifacts from file watching | ビルド中の開発サーバー応答を維持 |
| `c49bc28` | fix(device): match the Launchkey MK4 61 physical layout | 公式61鍵配置・共通描画・設定移行 |
| `c3df5a5` | feat(updates): prompt for new Windows releases automatically | 自動/手動/トレイ更新確認と通知 |

公開パイプラインでも検証を行います。配布バイナリの自己テスト結果とビルド元は [v0.2.0 Release](https://github.com/lingmulongtai/Keylume/releases/tag/v0.2.0) の `verification.json` と `build-provenance.json` を参照してください。コード署名、実機発光・OLED・DAW共存、スリープ、72時間稼働は未検証です。

## v0.1.0 — 2026-09-16

Windows 上で v0.1.0 を検証しました。自動受け入れテストは MockDevice を基準にしています。接続済み MIDI ポートの列挙も確認していますが、実機の発光・演奏・DAW 共存の正しさは未確認です。

## 自動チェック

| チェック                | 結果              | 対象                                                                                                      |
| ----------------------- | ----------------- | --------------------------------------------------------------------------------------------------------- |
| Rust unit tests         | 26 passed         | プロトコル、量子化、ブレンド、ゾーン、全エフェクト、保存復旧、入力転送、プロファイル、出力キュー、解放    |
| Vitest                  | 9 passed          | プレビュー、ペイント、レイヤー、1bit 画像変換                                                             |
| Playwright / Edge       | 6 passed          | プリセットの編集・保存・再読込・書出し・削除、停止、ペイント、プロファイル、Mock 検証、全画面・最小サイズ |
| TypeScript + Vite       | passed            | 配布用フロントエンド                                                                                      |
| Clippy                  | passed            | 全 target、警告をエラーとして扱う                                                                         |
| rustfmt / Prettier      | passed            | Rust とフロントエンドの書式                                                                               |
| npm audit               | 0 vulnerabilities | 開発依存関係を含む lockfile                                                                               |
| Native debug 自己テスト | 7 passed          | 実際の Tauri プロセスをトレイ起動して、UI なしで常駐コアを検証                                            |

Native 自己テストの項目は接続・描画、停止・解放、再開・初期化、DAW 譲渡、DAW 終了後の復帰、切断・再接続、プリセットのディスク保存です。通常の設定とは別の一時フォルダーを使います。

配布用 Release バイナリでも同じ **7 項目が通過**しました。結果は `artifacts/native-smoke-release.json`（`passed: true`, `hardwareTested: false`）。`npm run package` で NSIS インストーラーの生成に成功しています。インストール / アンインストール操作とコード署名は実施していません。

## 初期ローカルビルドの配布物（公開版の出力先とは異なります）

- `artifacts/Keylume_0.1.0_x64-setup.exe` — 12,348,151 bytes
- `artifacts/Keylume.exe` — 21,915,648 bytes
- `artifacts/Keylume_0.1.0_windows-x64.zip` — 実行ファイル、ドキュメント、ライセンスを同梱
- SHA-256 は `artifacts/checksums.json` に保存

公開パイプラインの配布物は `artifacts/release/` に生成します。Release に添付する `SHA256SUMS` と `build-provenance.json` が公開版のハッシュとビルド元を示します。上記のサイズ・パスは最初の手動ビルドの記録です。

手動のブラウザー確認では、1280 × 800 の画面、デバイスキャンバス、各画面への遷移、ブラウザーエラーがないことを確認しました。

ネイティブ WebView でも画面描画、Rust IPC による状態取得（13 プリセット・42 LED）、一時停止後の `paused` 状態を確認しました。確認用の設定は `artifacts/native-ui-settings.json` に保持しています。

Release 版についても `http://tauri.localhost/` から同梱画面が表示され、Rust IPC と継続描画が動くことを確認しました。WebView のエラーログは空です。スクリーンショットは `artifacts/keylume-release.png` に保存しています。

## 検証で修正した問題

- OS / オーディオデバイス列挙に数秒かかる環境で描画と一時停止が止まる問題を、列挙スレッドの分離で修正。修正前に失敗した native pause テストは修正後に通過しました。
- 出力が遅い場合に LED の差分が失われる問題を、アドレスごとの最新値を保持する送信キューで防止しました。
- 同じ描画フレーム内に MIDI Clock が複数届く場合も、入力到着時刻でテンポを計算します。72 時間経過相当の時刻でもテストしています。
- 全輝度から消灯する際の整数オーバーフロー、レイアウト変更時の保持ノート、破損設定復旧時のバックアップ保持を確認しました。

- 公開前レビューに対応し、MIDI ドライバー停止時のキュー投入・終了待ちを制限して期限切れ命令を破棄し、低サンプルレートで FFT の負周波数側を参照しないよう修正しました。
- 不正なレイアウト保存を拒否し、LED 数の減少時にもデバイス画面を維持します。ブラウザープレビューの保存状態、呼吸エフェクトの色、検索表示を修正しました。

## コミット

| SHA       | 件名                                                                | 目的                                               |
| --------- | ------------------------------------------------------------------- | -------------------------------------------------- |
| `bc23612` | feat(core): render layered lighting and validate device protocols   | データモデル、エンジン、Mock、保存、転送の基礎     |
| `7092717` | feat(ui): add the lighting editor and guided device setup           | 編集画面とセットアップ                             |
| `2604d99` | fix(ui): apply presets directly from the editor selector            | エディターから直接プリセットを選択                 |
| `4d57fda` | feat(midi): keep the latest LED updates under output backpressure   | 遅い MIDI 出力でも色更新を保持                     |
| `0c07ef5` | fix(engine): derive MIDI tempo from input arrival times             | フレーム周期から独立したテンポ計算                 |
| `3587c0b` | feat(desktop): run lighting and DAW coexistence in the Windows tray | Windows 常駐、実機 I/O、音声、共存、パッケージ設定 |
| `a4e3dd0` | fix(ui): keep the preset selector from crowding the editor title    | 選択欄の幅を制限し見出しを保持                     |

追加のポート識別修正: `565e089` — `fix(midi): identify Windows DAW ports regardless of enumeration order`。Windows の MIDIIN2 / MIDIOUT2 を名前で識別し、列挙順に依存しないことをテストしました。

上記は初期ローカル実装時点の記録です。公開版の履歴は [GitHub](https://github.com/lingmulongtai/Keylume)、各ビルドの結果は [Actions](https://github.com/lingmulongtai/Keylume/actions)、配布ファイルは [v0.1.0 Preview](https://github.com/lingmulongtai/Keylume/releases/tag/v0.1.0) を参照してください。Release の `build-provenance.json` が配布バイナリのビルド元コミットを示します。

## 未検証・対応範囲

実機 LED アドレス・OLED・DAW の組み合わせ・スリープ・長時間安定性・性能目標は未検証です。ソフトウェアで未実装の仕様細部も [実装状況](implementation-status.md) に記載しています。[実機受け入れチェックリスト](hardware-acceptance.md) が完了するまでは、仕様全体の受け入れ完了とは扱いません。
