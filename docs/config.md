# 設定ファイル（`dat_linter.toml`）

`--config <path>` で TOML 形式の設定ファイルを指定できます。省略時はカレントディレクトリ直下の
`dat_linter.toml` を自動探索します（存在しなければ全ルール有効のまま動作）。無ければ
`lint`/`fmt`/`analyze` 初回実行時にコメント付きのデフォルト設定ファイルが自動生成されます。

## `[rules]` — ルールの include/exclude

`Diagnostic.code`（例 `obsolete-type` / `missing-tile-image`）単位で有効/無効を制御します。
`lint`/`fmt`/`analyze` すべてに同じ意味論で適用されます。

```toml
[rules]
include = ["obsolete-type", "missing-waytype"]  # 空なら全ルールが候補（デフォルト）
exclude = ["duplicate-key"]                      # includeの結果からさらに除外
```

1. `include` が空なら全ての `code` が候補。非空なら列挙された `code` のみが候補。
2. 候補集合から `exclude` に列挙された `code` をさらに除外する（`exclude` が常に優先）。

```
dat_linter lint    --config dat_linter.toml path/to/file.dat
dat_linter fmt     --config dat_linter.toml path/to/file.dat
dat_linter analyze --config dat_linter.toml path/to/vehicle_dir
```

`code` の一覧・現在の有効/無効・説明は次のコマンドで確認できます。

```
dat_linter list                              # 全code一覧（--source lint|fmt|analyze で絞込）
dat_linter list --config dat_linter.toml     # 現在の設定での有効/無効も表示
dat_linter describe obsolete-type            # なぜNGか・どう直すかを表示
```

## `[general]` — 出力言語

```toml
[general]
language = "en"  # "en"（デフォルト） または "ja"
```

設定ファイルが無い・`language` 未指定・不正な値の場合はすべて英語にフォールバックします。
`lint`/`fmt`/`analyze`/`list` の `--help` 短文もこの設定に従いますが、`lint --help` が表示する
22 obj種別の長い一覧文だけは翻訳対象外で常に日本語です。

## ログレベルと終了コード

| level | 表示条件 | 用途 |
|---|---|---|
| error | 既定 | pak 化に失敗する／ゲーム内で正常に表示されない |
| warn | 既定 | 非推奨・設定が推奨される項目 |
| info | `-v` | 各チェックの合格確認 |
| debug | `-vv` | 生の値・解決後パス・索いたキー名 |

`error`/`warn` の診断が1件でもあれば終了コードは `1`、無ければ `0` です。

## 出力チャンネル（stdout/stderr）

- 診断本文（error/warn/info/debug）は常に **stderr**
- 情報メッセージ（合格通知・サマリ行）は **stdout**（指摘0件時はこれも省略。サイレント成功）
- `fmt` を `--write` 無しで実行した場合の整形結果本体は常に stdout
