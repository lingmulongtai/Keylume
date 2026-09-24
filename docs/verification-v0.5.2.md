# v0.5.2 検証記録

対象: Windows x64、Launchkey MK4 61向けPreview。2026-09-25に確認しました。

## 実機の受信記録

利用者に各ボタンを順に押してもらい、通常MIDI・DAWの両ポートを同時に受信しました。既存アプリを開いたまま、診断側からモード変更などは送っていません。

| 操作                    | 通常MIDIポートの記録（16進数）      |
| ----------------------- | ----------------------------------- |
| 中央左／右              | `BF 67 7F` / `BF 66 7F`、解放は値00 |
| Undo                    | `BF 4D 7F` / 00                     |
| Record                  | `BF 75 7F` / 00                     |
| Metronome               | `BF 4C 7F` / 00                     |
| Play / Stop             | `FA` / `FC`                         |
| Capture MIDI / Quantise | `BF 4A 7F` / `BF 4B 7F`、解放は値00 |

従来のDAWポートに限定した処理から、これらの通常MIDI入力も操作へ接続しました。信号を回帰テストへ追加し、短い押下表示とリアルタイムメッセージの繰り返しも検査しています。

続けて操作したShift・Settings・Scale・Chord Map・Arp・Fixed Chord・Volume・Custom 1については、記録中に`B0 0B` / `B0 0C`を観測しましたが、各操作へ一意に対応付けられませんでした。全ボタンの実機確認済みとはしていません。MIDI Learnは通常MIDIのCustom CCにも対応します。

## 自動チェック

- Rustライブラリ: 82成功、1件は明示実行用の外部GitHub接続テストとして除外。
- Vitest: 7ファイル、23テスト成功。
- Playwright: 18テスト成功。位置合わせ・透明エフェクトの操作試験は低速ホスト用に60秒の制限を設定し、個別にも再実行。
- TypeScript / Vite production build、Rust fmt、Clippy全ターゲット（警告をエラー扱い）成功。
- サードパーティーライセンス一覧を再生成し、差分なし。
- モニターの負座標／異なるDPI、OLEDの応答待ちと表示期限、Capture後のNote Off、Quantiseの音価、Undo、独立クリックをテスト。
- 録音ボタンで重ね録りを往復しても、再生中の長音のサンプル出力が消えないことを回帰テスト。

## Windowsアプリでの確認

普段使いのアプリとidentifier・設定保存先を分けた検証ビルドを使用しました。MIDI入力の再生試験はMockDevice、音声はWindowsの実出力コールバックです。

- 通常MIDIとして上記の実機記録を再入力し、中央左右のライティング切替、録音・重ね録り・Undo、Play/Stop、独立メトロノームが動作。
- 無演奏のCaptureで操作ボタン自身が録音されず、鍵盤を弾くと取得でき、Quantise/Undoを操作できることを確認。
- 音量操作から`Piano Volume`のOLED SysExが送られることをMock出力で確認。フェーダーへのテンポ割り当ても確認。
- MIDI練習のPlayで選択画面と操作パネルを表示。パネルの一時停止・シーク・再生・停止・MIDI読込・クリック透過・位置合わせが動作。
- 表示の開閉を3回繰り返して再表示。操作パネルの「隠す」でネイティブウィンドウが非表示になり、本画面から再表示できることを確認。
- Windows合成後の画面を確認し、透明な演奏ウィンドウ越しの壁紙・他アプリがぼけたり暗くなったりしないことを確認。内部WebViewの透過だけでの判定にはしていません。利用者のデスクトップを含む検証画像は公開物に含めません。
- 最終のWindows実行時に列挙されたモニターは1台。複数画面の座標処理は自動テストで検証していますが、この更新の最終ビルドを2台同時表示で受け入れたとはしていません。

音声の連続試験では、GeneralUser GSを使うライブ音と録音ループを再生し、180.5秒間、8ノブへ合計6,360回のCC入力を送りました。48kHz・480フレームのコールバックで、18回の発音ピークとループ時計の進行、3イベントの録音保持を確認。固定出力／Windows既定の切替で出力を作り直した後、全音停止によるピーク0と新しい打鍵による非ゼロ出力、消去／Undoも確認しました。音声スレッドのpanicログはありません。

## アトミックコミット

| SHA       | 件名                                                                       | 目的                                             |
| --------- | -------------------------------------------------------------------------- | ------------------------------------------------ |
| `23ef9fb` | fix(controller): receive standalone hardware buttons and transport         | 実機の通常MIDIボタン・リアルタイム再生停止を受信 |
| `6f668ae` | fix(stage): clear native backdrops for true desktop transparency           | Windowsの暗い背景効果を解除                      |
| `ceb098c` | feat(groove): unify recording controls and add standalone metronome        | 本体と画面の録音動作統一、独立クリック・BPM      |
| `3618156` | feat(lighting): make knob and fader positions readable                     | 円弧・レベルバー・数値を表示                     |
| `c7a9a1a` | feat(oled): show timed control names and values on hardware                | 操作名・値・切替先の一時表示                     |
| `a908165` | feat(controller): edit bindings from the hardware diagram                  | 本体図から操作割り当てを編集                     |
| `f0af05b` | fix(controller): learn custom CC inputs from the regular MIDI port         | 通常MIDIのCustom CCを学習                        |
| `b0015c2` | feat(groove): capture recent playing and quantise recorded loops           | Capture MIDI・Quantise・Undo                     |
| `7ef849c` | feat(stage): open selected displays on play with shared transport controls | 再生時の自動表示と独立操作パネル                 |
| `f2caabe` | fix(stage): restore hidden controls and refresh changed monitors           | パネル再表示とモニター変更時の再生成             |
| `458ce6e` | test(stage): allow the full appearance flow to finish on busy hosts        | 画面試験の実行時間に合わせた制限                 |
| `06e3c49` | fix(groove): retain note releases and exclude control buttons from capture | Captureの音価と不要CC記録を修正                  |
| `83f04ec` | fix(audio): keep sustained loop voices when toggling overdub               | 重ね録り切替で再生音を保持                       |
| `2fd7544` | docs: explain hardware feedback and the performance controls               | 操作方法と制約を更新                             |
| `9443555` | fix(controller): keep short button presses visible between frames          | 短い本体押下も画面表示                           |
| `1ea3c52` | chore(release): prepare version 0.5.2 preview                              | npm / Cargo / Tauriの版を一致                    |

本記録のドキュメントコミットとマージコミットを含む履歴は、[変更・コミット列](https://github.com/lingmulongtai/Keylume/compare/v0.5.1...v0.5.2)から確認できます。squashせず保持します。

## 公開物と残る確認

[v0.5.2 Release](https://github.com/lingmulongtai/Keylume/releases/tag/v0.5.2)へSHA-256、ビルド元コミット、配布バイナリのMock自己テスト7項目を添付します。タグのWindows CIがビルドとテストを再実行し、成功した成果物を公開します。

更新後の物理ノブ・鍵盤・ペダルによる音の聴取、OLED表示の視認、実際の2画面、DAW共存、Windows既定出力変更、USB抜き差し・長時間稼働の実機受け入れは未完了です。Alt+F4によるパネルのhideはコードレビューで確認し、操作自動化によるキー入力は対象パネルをアクティブ化できず未実施です。「隠す」ボタンと再表示はWindows上で確認済みです。未署名Previewで、インストール／アンインストールの実行は検証していません。
