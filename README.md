# simutrans-dat-linter

Simutrans アドオンの `.dat` を **pak 化する前に** 静的検証する Rust 製 CLI ツールです。
makeobj が認識する全 22 obj 種別を検証します。

`makeobj` は一部の問題を見逃したまま pak を生成します。
このツールは makeobj のソースを根拠に、それらを pak 化前に検出します。

## 特徴

- **`fmt`** — `.dat` を慣習的な形式へ自動整形
- **`lint`** — makeobj では見逃される問題を pak 化前に検出
- **`analyze`** — 複数ファイルを横断して車両連結制約を検証

対応 obj 種別・各コマンドの詳細は [`docs/`](docs/) を参照してください。

## インストール

リリースページから OS にあった実行ファイルをダウンロードしてください。
上級者向け: パスを通しておくとどこからでも呼び出せます。

## クイックスタート

```
dat_linter lint addons/vehicle.dat
```

指摘が無ければ何も出力せず終了します（サイレント成功）。指摘があれば内容が表示され、
終了コードが `1` になります。

## 主要コマンド

```
dat_linter init                        # dat_linter.toml をカレントディレクトリに生成
dat_linter fmt   <file|dir|glob> -w    # 整形して書き戻す
dat_linter lint  <file|dir|glob>       # 静的検証
dat_linter analyze <vehicle_dat_dir>   # 車両連結制約の解析
```

各コマンドの `--help` で詳細を確認できます。

## 設定ファイル

ルールの有効/無効・出力言語は `dat_linter.toml` で設定します。無ければ全ルール有効・`language=en`の
デフォルト設定で動作しますが、暗黙の自動生成はしません。カレントディレクトリに雛形を作りたい場合は
明示的に `dat_linter init` を実行してください（既に `dat_linter.toml` が存在する場合は上書きせず
失敗終了します）。

```
dat_linter init
```

```toml
[rules]
exclude = ["duplicate-key"]
[general]
language = "en"
```

詳細は [docs/config.md](docs/config.md) を参照してください。

## 詳細ドキュメント

| ドキュメント                       | 内容                           |
| ---------------------------------- | ------------------------------ |
| [docs/fmt.md](docs/fmt.md)         | フォーマッタ                   |
| [docs/lint.md](docs/lint.md)       | `lint` の使い方・対応 obj 種別 |
| [docs/analyze.md](docs/analyze.md) | 静的解析                       |
| [docs/config.md](docs/config.md)   | 設定・ログ・終了コード         |

開発者向け情報（アーキテクチャ・テスト・リリース手順）は [CLAUDE.md](CLAUDE.md) を参照してください。

## ライセンス

MIT License（[LICENSE](LICENSE) 参照）
