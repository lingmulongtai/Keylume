# 検証記録 — 2026-09-16

Windows 上で v0.1.0 を検証しました。自動受け入れテストは MockDevice を基準にしています。接続済み MIDI ポートの列挙も確認していますが、実機の発光・演奏・DAW 共存の正しさは未確認です。

## 自動チェック

| チェック                | 結果              | 対象                                                                                                      |
| ----------------------- | ----------------- | --------------------------------------------------------------------------------------------------------- |
| Rust unit tests         | 23 passed         | プロトコル、量子化、ブレンド、ゾーン、全エフェクト、保存復旧、入力転送、プロファイル、出力キュー、解放    |
| Vitest                  | 5 passed          | プレビュー、ペイント、レイヤー、1bit 画像変換                                                             |
| Playwright / Edge       | 5 passed          | プリセットの編集・保存・再読込・書出し・削除、停止、ペイント、プロファイル、Mock 検証、全画面・最小サイズ |
| TypeScript + Vite       | passed            | 配布用フロントエンド                                                                                      |
| Clippy                  | passed            | 全 target、警告をエラーとして扱う                                                                         |
| rustfmt / Prettier      | passed            | Rust とフロントエンドの書式                                                                               |
| npm audit               | 0 vulnerabilities | 開発依存関係を含む lockfile                                                                               |
| Native debug 自己テスト | 7 passed          | 実際の Tauri プロセスをトレイ起動して、UI なしで常駐コアを検証                                            |

Native 自己テストの項目は接続・描画、停止・解放、再開・初期化、DAW 譲渡、DAW 終了後の復帰、切断・再接続、プリセットのディスク保存です。通常の設定とは別の一時フォルダーを使います。

配布用 Release バイナリでも同じ **7 項目が通過**しました。結果は `artifacts/native-smoke-release.json`（`passed: true`, `hardwareTested: false`）。`npm run package` で NSIS インストーラーの生成に成功しています。インストール / アンインストール操作とコード署名は実施していません。

## 配布物

- `artifacts/Keylume_0.1.0_x64-setup.exe` — 12,348,151 bytes
- `artifacts/Keylume.exe` — 21,915,648 bytes
- `artifacts/Keylume_0.1.0_windows-x64.zip` — 実行ファイル、ドキュメント、ライセンスを同梱
- SHA-256 は `artifacts/checksums.json` に保存

手動のブラウザー確認では、1280 × 800 の画面、デバイスキャンバス、各画面への遷移、ブラウザーエラーがないことを確認しました。

ネイティブ WebView でも画面描画、Rust IPC による状態取得（13 プリセット・42 LED）、一時停止後の `paused` 状態を確認しました。確認用の設定は `artifacts/native-ui-settings.json` に保持しています。

Release 版についても `http://tauri.localhost/` から同梱画面が表示され、Rust IPC と継続描画が動くことを確認しました。WebView のエラーログは空です。スクリーンショットは `artifacts/keylume-release.png` に保存しています。

## 検証で修正した問題

- OS / オーディオデバイス列挙に数秒かかる環境で描画と一時停止が止まる問題を、列挙スレッドの分離で修正。修正前に失敗した native pause テストは修正後に通過しました。
- 出力が遅い場合に LED の差分が失われる問題を、アドレスごとの最新値を保持する送信キューで防止しました。
- 同じ描画フレーム内に MIDI Clock が複数届く場合も、入力到着時刻でテンポを計算します。72 時間経過相当の時刻でもテストしています。
- 全輝度から消灯する際の整数オーバーフロー、レイアウト変更時の保持ノート、破損設定復旧時のバックアップ保持を確認しました。

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

ブランチは `feat/keylume-desktop`。リモートへの push / 公開は行っていません。

## 未検証・対応範囲

実機 LED アドレス・OLED・DAW の組み合わせ・スリープ・長時間安定性・性能目標は未検証です。ソフトウェアで未実装の仕様細部も [実装状況](implementation-status.md) に記載しています。[実機受け入れチェックリスト](hardware-acceptance.md) が完了するまでは、仕様全体の受け入れ完了とは扱いません。
