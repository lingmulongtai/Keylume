# v0.5.0 検証記録

2026-09-21。物理Launchkeyの受け入れと、ソフトウェアの自動検証を区別します。今回もWindows Previewとして配布します。

## 自動チェック

| チェック          | 結果                  | 主な対象                                                                               |
| ----------------- | --------------------- | -------------------------------------------------------------------------------------- |
| Rust              | 71 passed / 1 ignored | 音源ID・SF2検証・インポート、8エフェクト、6キット、Undo、入力解釈、設定patch、既存コア |
| Vitest            | 22 passed             | MIDI読込、位置合わせ、黒鍵幅と色、音名、設定の差分統合                                 |
| Playwright / Edge | 17 passed             | 音源カード、割り当て保存、ドラム編集、全画面遷移、演出・レイヤー・設定の永続化         |
| TypeScript / Vite | passed                | 配布用フロントエンド                                                                   |
| Clippy / rustfmt  | passed                | 全target・警告をエラー扱い、Rust書式                                                   |
| ライセンス再生成  | 342 packages、一致    | ロックされた依存と配布通知                                                             |
| Tauri debug build | passed                | 検証用identifierの独立したWindowsアプリ                                                |

ignoredは既存の外部通信テストです。GeneralUserのダウンロードは別のネイティブ検証で実通信しました。ブラウザーCLIが起動できなかったため、同じMicrosoft EdgeをPlaywrightで操作して画面とエラーを確認しています。

## Windowsネイティブの確認

普段使いのアプリと設定領域・identifierを分けた検証用プロセスへ、Mock MIDIとIPCで操作しました。実機MIDIの代わりに合成入力を使い、音声はWindowsの実出力コールバックを通しています。

- GeneralUser GS 2.0.3を固定HTTPS URLから取得。32,319,396 bytes、SHA-256 `9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe`。4同梱＋274追加の音色を確認。
- グランドピアノ、シンセリード、シンセパッド、ストリングスを48kHzのWindows音声出力で発音。コールバック480frames、非ゼロの出力ピークを観測。これは鍵盤からスピーカーまでの遅延測定ではありません。
- SF2をアプリ領域へインポートし、元ファイルを削除しても発音できることを確認。
- ノブCCでエフェクトを変更。TouchやCustomモードの同番号CCを既定ノブとして誤実行しないことを確認。
- パッド上下・ノブ上下・中央左右を押下時に1回だけ実行し、キット・音色・照明を切替。Customフェーダーによる音量の誤変更を防止。
- モード切替で保持音と演奏表示を解放し、再接続せず演奏へ戻る。MIDI Learnは割り当ての実行を抑止。
- ドラムと鍵盤を録音し、消去後に本体Undo相当入力でイベント数・テンポを復元。
- 出力先の明示指定と「Windowsの既定」設定の切替後も録音を保持。
- 2560×1440の2台（左側はx=-2560）のモニターで透過画面を開く。HTML/body/rootの透明背景と、Win32の最前面・クリック透過スタイルを確認。本画面から編集の解除／再ロック／全画面の終了を確認。

UI画像は現在の1440×1000ブラウザープレビューから撮影しました。黒鍵の幅・暗い色、重なり順、ドレミ表示、演出のみの透明プレビュー、音源カードとライティングカードを目視しました。

## レビューで修正した競合

ノブ変更とUI保存を全設定の上書きで競合させず、変更した項目だけを最新設定へ統合します。常駐ループのrevisionとライブ設定は同じスナップショットから取得し、通常時と終了時のディスク保存はUI保存と同じロック順に統一しました。Learnの開始を入力処理と同じキューで扱い、転送済みノートの解放を取りこぼさないようにしました。OS操作の待ち行列は設定変更・モード変更・停止で失効させます。

## アトミックコミット

各行は独立した関心事です。機能・修正ごとに関連するRustテスト、UIテストまたは型チェック／ビルドを実施し、最後に上記の全体チェックを行いました。履歴はsquashせず保持します。[完全な変更・コミット列](https://github.com/lingmulongtai/Keylume/compare/v0.4.0...v0.5.0)

| SHA       | 件名                                                                             | 目的                               |
| --------- | -------------------------------------------------------------------------------- | ---------------------------------- |
| `5554e9a` | fix(stage): select calibration handles precisely and coalesce drag updates       | 位置合わせの操作安定化             |
| `e902a3f` | feat(stage): distinguish black notes and offer solfege labels                    | 黒鍵幅・色・音名                   |
| `4d12643` | feat(audio): add eight smooth stereo instrument effects                          | 音作りのDSP                        |
| `b345160` | feat(stage): add reactive visual effects and independent display layers          | 演出と表示レイヤー                 |
| `0d20f2a` | feat(stage): overlay transparent effects with recoverable click-through controls | 透過画面と復帰操作                 |
| `1e6439b` | feat(instruments): expose live sound shaping without restarting audio            | エフェクトUIと音声連携             |
| `7a17edd` | build(audio): add verified HTTPS sound-pack downloads                            | 取得用依存とライセンス             |
| `b929ad3` | feat(library): catalog instrument patches and validate imported SoundFonts       | 音源カタログと検証                 |
| `63fff80` | feat(library): install verified sound banks and switch their instrument patches  | ダウンロード・インポート・音色切替 |
| `12313f4` | feat(instruments): browse searchable sound cards and import personal banks       | 音源カードUI                       |
| `599e3b6` | feat(drums): add six stereo kits with editable pad voices                        | ドラム合成                         |
| `baad761` | feat(drums): edit and persist each pad within its selected kit                   | キット別編集UI                     |
| `9823ed6` | feat(looper): undo recording overdubs and clearing with bounded history          | 8段階Undo                          |
| `f3d5b4f` | feat(controls): define per-mode mappings and decode Launchkey control input      | 操作定義と入力解釈                 |
| `d2e8019` | feat(controls): connect hardware modes effects and desktop actions safely        | 本体・演奏・OS操作の統合           |
| `e80dd0b` | feat(controls): edit hardware assignments and switch performance modes           | 割り当てUI                         |
| `aaf499a` | fix(settings): preserve live hardware edits when saving UI preferences           | 設定patchの統合                    |
| `1b11850` | feat(navigation): choose lighting visually and consolidate advanced settings     | 視覚的選択とタブ整理               |
| `db959b2` | fix(controls): stop desktop actions on panic and respect custom faders           | 全音停止とCustom保護               |
| `b27b285` | fix(stage): keep black notes legible and strengthen particle trails              | 重なり順と粒子の視認性             |
| `ec327d8` | fix(lighting): keep preset thumbnails above their labels                         | サムネイル配置                     |
| `da844c9` | fix(runtime): serialize controller transitions and settings persistence          | 同時操作・Learn・保存の競合修正    |
| `30d2d0d` | docs: explain sound libraries hardware mappings and desktop overlays             | 操作ガイド                         |
| `4e8208b` | chore(release): advance all package versions to 0.5.0                            | バージョン整合                     |
| `ed3ca19` | docs: present v0.5 features with current interface screenshots                   | READMEと最新画像                   |
| `12c584b` | docs(release): describe v0.5 downloads features and support boundaries           | リリースノート                     |

本検証記録のコミットとPRのマージコミットも、上記の完全なコミット列から確認できます。

`de1edb9` — `fix(stage): create overlays with their final topmost window style`では、透過ウィンドウの最前面指定を作成時へ移し、2画面起動時に指定が遅れる状況を防止しました。関連するモニター座標テストとネイティブのウィンドウスタイル検査を実施しました。

## 公開物と未検証範囲

[v0.5.0 Release](https://github.com/lingmulongtai/Keylume/releases/tag/v0.5.0)の`SHA256SUMS`、`build-provenance.json`、`verification.json`が公開物の正本です。タグ用CIは全チェックとWindowsインストーラー生成、配布バイナリのMock自己テスト7項目を再実行します。追加32MB音源は配布物に埋め込まず、ライセンス原文とカタログを同梱します。

物理Launchkey、OSショートカットと対象アプリの組み合わせ、実際のWindows既定出力デバイス変更・USB抜き差し、物理ペダル、LED/OLED、DAW共存、スリープ・ロック復帰、長時間稼働は未受け入れです。インストール／アンインストール操作とコード署名は実施していません。ASIO、VST、仮想ゲームパッド、ループのファイル保存・書出しは対象外です。
