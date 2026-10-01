# v0.6.0 検証記録

2026-10-02。Windows x64、Launchkey MK4 61向けPreview。自動テスト、Windowsアプリ、本体の実機受け入れを分けて記録します。

## 自動チェック

- Rustライブラリ: 96成功、外部GitHub接続を明示実行する1件は除外。
- Vitest: 8ファイル、24テスト成功。
- Playwright: 19テスト成功。ホーム、本体割り当て、MIDI読込、左右の手・伴奏・音量・音声タイミング、独立操作パネルを確認。
- TypeScript / Vite production build、Rust fmt、Clippy全ターゲット（警告をエラー扱い）成功。既存のTauriダイアログの静的／動的import警告は残ります。
- サードパーティーライセンス342件の再生成に差分なし。npm / Cargo / Tauriのバージョンは0.6.0。
- 手の推定・手動指定・待機と採点の手別判定、同音の重なり、シーク・停止・ループ、負の音声補正の冒頭、正の補正で待機中に鳴らす伴奏をテスト。
- Feature Controlsの問い合わせ・初期値保存・解除確認・押下・リセット通知・復元・タイムアウト、OLEDの本体ポップアップ設定とACK欠落時の復元をテスト。
- 選択したピアノ音量フェーダーを両モードで予約する処理と、Windows音量の相対操作を累積する処理をテスト。

個々の論理単位で関連チェックを実行してコミットしました。最後の表示修正ではVitest・production build、Mockの解除確認修正ではruntimeの2テスト・fmtを再実行しました。公開時はタグのCIで全チェックと配布ビルドを再実行します。

## Windowsアプリでの確認

identifier・設定保存先を通常版から分けた検証ビルドを使用しました。次の操作はネイティブウィンドウへの実際のマウス入力で確認しました。

- 2560×1440・拡大率100%の2台を同時表示。左側モニターの原点はx=-2560です。
- 固定・クリック透過の演奏表示中でも、独立操作パネルの再生をクリックでき、MIDI音声のWindows出力ピークが非ゼロになりました。
- パネルの一時停止で再生が止まり、音声ピークが0になりました。
- パネルの位置合わせ／固定を往復でき、パネル操作は維持されました。
- 2台の演奏表示を開いたまま、背面の本画面のプリセット選択をクリックできました。

利用者のデスクトップを含む画像は公開物へ含めません。EscによるComputer Use停止を受け、追加の画面操作自動化は停止しました。Alt+F4によるパネル非表示、異なるDPIでの2台同時表示はこのWindows確認の対象外です。

音声はWindowsの実出力コールバック（48kHz・480フレーム）を使い、開発用IPCと合成した曲データで次の6項目を確認しました。スピーカーからの聴取や物理打鍵の受け入れとは区別します。

1. ライブ音量0でもMIDI音声を鳴らせ、ルーパーへ自動音が録音されない。
2. 右手待機中、100ms遅らせた左手伴奏が鳴り、音価どおりに解放される。
3. -500msの補正でも冒頭の短音を欠かさず、一時停止で消音する。
4. 区間リピートと、固定／Windows既定の出力再作成後に再発音する。
5. MIDI消音中も表示時計が進み、キューあふれを起こさない。
6. デスクトップ中のライブ演奏とMIDI音声オフが独立している。

MockDeviceの配布バイナリ自己テストは従来の7項目を維持します。接続・描画、停止中のコントローラー、再開、DAW譲渡、DAW終了後の復帰、抜き差し、プリセット永続化を検査し、`hardwareTested=false`を明記します。

## 本体の確認と制約

実機のLaunchkey MK4 61の通常MIDI・DAWポートへ接続し、Novationの機種識別応答とFeature Controlsの状態通知を受信しました。追加の物理ボタン操作・フェーダー操作・OLED視認の受け入れは未完了です。問い合わせ応答やMock入力を物理押下の成功として扱いません。

Scale／Arp／Chord Mapは[Novation Feature Controls](https://userguides.novationmusic.com/hc/en-gb/articles/23754916107922-Launchkey-feature-controls)の状態通知を利用します。問い合わせ応答と本体のリセット通知に押下の識別子がないため、解除確認後の状態変化だけを操作へ接続します。状態の確認に失敗した場合は割り当てを停止して警告します。DAW譲渡・終了前に元の本体状態を復元しますが、物理切断や送信エラー後の復元成功を保証しません。

OLEDはASCII表示です。設定した表示期間中に一時ページを更新し、フェーダー等の本体標準ポップアップを抑えます。本体の非揮発タイムアウト設定は変更しません。全ボタンの上書き、OLEDの全値での表示、USB再接続・スリープ・DAW共存・長時間演奏は実機受け入れ対象です。

左右の手はトラック名と音域・直前の手の位置を使う推定です。声部分離の研究も参照しましたが、[Madsen / Widmerの声部分離](https://www.ofai.at/~soren.madsen/pub/ismir06.pdf)そのものを再実装したものではありません。交差・密集した声部の正解を保証せず、トラック指定と分割音で修正できます。

## アトミックコミット

| SHA | 件名 | 目的 |
| --- | --- | --- |
| `3cc0a61` | fix(controller): keep manual hardware actions active when lighting is paused | 停止中の入力と手動ライティング選択を保持 |
| `8e3b751` | fix(oled): prevent native popups from masking operation feedback | 本体の標準表示との競合を抑制 |
| `4b2cc21` | fix(stage): pass desktop clicks through while keeping transport interactive | 演奏表示と操作パネルの入力を分離 |
| `9089059` | feat(practice): assign piano hands and grade only the selected hand | 手の推定と手別の待機・採点 |
| `742bf08` | feat(stage): color each hand and expose practice hand controls | 左右の色と対象手のUI |
| `332de18` | feat(practice): schedule independent accompaniment and listening playback | 伴奏の時刻管理と音声補正 |
| `0cf7400` | feat(stage): add listening, accompaniment volume and audio timing controls | 試聴・伴奏・音量・補正のUI |
| `1eb6c1c` | feat(audio): mix MIDI playback independently from live piano and looper | ライブ・録音から独立した自動音声 |
| `017f4fc` | feat(controller): control Windows volume while keeping piano available | 既定出力の音量と常時演奏 |
| `9db4730` | feat(home): show current control assignments on hardware hover | 本体図で割り当てを表示 |
| `6a8b88f` | feat(controller): claim native feature reports without firing query or reset echoes | 本体機能の通知を操作へ接続 |
| `15e5418` | fix(oled): retain operation feedback until its configured expiry | 表示期間の更新と期限処理 |
| `d422e52` | test(practice): cover hand and playback controls on both surfaces | 本画面・操作パネルの回帰テスト |
| `460b4e8` | fix(oled): restore native popups without waiting for a bitmap acknowledgement | ACK欠落時も本体設定を復元 |
| `949b438` | fix(controller): reserve the chosen piano volume fader across both modes | フェーダー割り当ての競合を解除 |
| `f939868` | fix(controller): preserve relative Windows volume moves and report the actual level | 相対操作の累積と実音量表示 |
| `fb9a5b6` | fix(controller): observe hardware mode changes while lighting is paused | 停止中も本体モードを取得 |
| `f626ff6` | fix(controller): restore native feature states before releasing their MIDI transport | DAW解放前に本体状態を復元 |
| `2f3b48b` | fix(practice): preserve offset audio at opening notes and wait targets | 補正した冒頭と待機中の伴奏を保持 |
| `36adbc7` | fix(build): keep controller volume tests after the implementation | Clippy規約へテストの配置を修正 |
| `f02bc10` | chore(release): prepare version 0.6.0 preview | バージョンを統一 |
| `a9b4a56` | fix(stage): retain both hand colors during listening playback | 試聴で両手を色分け |
| `15c6df4` | fix(controller): avoid hardware ownership warnings in mock mode | Mockでの誤った本体解除警告を防止 |

本記録・操作ガイドのドキュメントコミットとマージコミットを含む[変更・コミット列](https://github.com/lingmulongtai/Keylume/compare/v0.5.2...v0.6.0)をsquashせず保持します。

## 公開物

[v0.6.0 Release](https://github.com/lingmulongtai/Keylume/releases/tag/v0.6.0)へ、インストーラー、ポータブルZIP、SHA-256、ビルド元コミット、配布バイナリの自己テストを添付します。タグのWindows CIが成功した成果物だけを公開します。公開後に配布物をダウンロードし、ハッシュ・タグとのコミット一致・自己テスト7項目・同梱ドキュメント／ライセンス／音源を検証します。

未署名Previewです。インストール／アンインストール自体の実行検証は含みません。
