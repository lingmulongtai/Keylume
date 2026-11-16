# Windows のローカルビルド出力

OneDrive 内のチェックアウトでは、生成済みの `target` 内ファイルがオンラインのみになる場合があります。PDB 書込みで `LNK1201` が出た場合は、空き容量・パス・権限に加え、`attrib` の `O`（Offline）属性を確認してください。`LNK1201` だけで原因は断定できません。

ソースを移動したり、OneDriveの設定・権限を変更したりせずに、Cargo標準の出力先指定で通常ビルドできます。

```powershell
$target = Join-Path $env:LOCALAPPDATA 'KeylumeBuild\local'
cargo build --manifest-path src-tauri/Cargo.toml --target-dir $target --offline
```

`--offline` は必要な依存関係がキャッシュにある場合に使えます。キャッシュ不足のエラーは別途解決してください。出力先を変えると最初は依存関係も再ビルドされ、数GBの空き容量が必要です。上の設定はこのコマンドだけに適用されます。既存の `target`、インストール版、ユーザー設定は変更しません。

2026-11-17の検証では、古い `keylume_lib.pdb` がOffline状態で通常の出力先でのリンクに失敗しました。同じソースをOneDrive外の新規出力先でビルドすると、標準dev profile（debuginfoあり）のEXE・DLL・PDB生成まで成功しました。元の同期失敗理由までは特定していません。生成したデスクトップアプリの起動・音出し・インストーラー作成は、この検証には含めていません。

参考: [Cargo build の出力先指定](https://doc.rust-lang.org/cargo/commands/cargo-build.html)、[Microsoft LNK1201](https://learn.microsoft.com/en-us/cpp/error-messages/tool-errors/linker-tools-error-lnk1201?view=msvc-170)。
