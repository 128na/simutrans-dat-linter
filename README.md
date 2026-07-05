# simutrans-dat-linter

Simutrans アドオンの `.dat` を **pak 化する前に** 静的検証する Rust 製 CLI ツールです。

`makeobj` はパラメーター不足・矛盾をほぼ無視して pak を生成してしまいます。
このツールは makeobj のソースを根拠に、makeobj が黙って見逃す・FATAL ERROR にする項目を
pak 化せずに検出します。

## 特徴

- **`lint`** — `.dat` 1ファイルを静的検証（pak化に失敗する／ゲーム内で正しく表示されない項目を検知）
- **`fmt`** — `.dat` を見やすく整形（キー小文字化・並び替え）
- **`analyze`** — ディレクトリ横断の静的解析（現状: 車両の連結制約チェック）

対応 obj 種別・各コマンドの詳細は [`docs/`](docs/) を参照してください。

## インストール

リリースページから OS にあった実行ファイルをダウンロードしてください。
上級者向け: パスを通しておくとどこからでも呼び出せます。

## クイックスタート

```
dat_linter lint xxx.dat
```

指摘が無ければ何も出力せず終了します（サイレント成功）。指摘があれば内容が表示され、
終了コードが `1` になります。

## 主要コマンド

```
dat_linter lint  <file|dir|glob>       # 静的検証
dat_linter fmt   <file|dir|glob> -w    # 整形して書き戻す
dat_linter analyze <vehicle_dat_dir>   # 車両連結制約の解析
```

各コマンドの `--help` で詳細を確認できます。

## 設定ファイル

ルールの有効/無効・出力言語は `dat_linter.toml` で設定します（無ければ初回実行時に自動生成）。

```toml
[rules]
exclude = ["duplicate-key"]
[general]
language = "en"
```

詳細は [docs/config.md](docs/config.md) を参照してください。

## 詳細ドキュメント

- [docs/lint.md](docs/lint.md) — `lint` の使い方・対応 obj 種別
- [docs/fmt.md](docs/fmt.md) — `fmt` の使い方
- [docs/analyze.md](docs/analyze.md) — `analyze` の使い方
- [docs/config.md](docs/config.md) — 設定ファイル・ログレベル・終了コード

開発者向け情報（アーキテクチャ・テスト・リリース手順）は [CLAUDE.md](CLAUDE.md) を参照してください。

## ライセンス

MIT License（[LICENSE](LICENSE) 参照）
