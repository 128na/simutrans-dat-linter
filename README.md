# simutrans-dat-linter

Simutrans アドオンの `.dat`（オブジェクト定義ファイル）を **pak 化する前に** 静的検証する Rust 製 CLI ツールです。

`makeobj` はパラメーター不足・矛盾をほぼ無視して pak を生成してしまい、ゲーム内で初めて不具合に気付く
→ 原因調査に時間がかかる、という問題があります。このツールは makeobj の C++ ソースを精読し、
**makeobj が黙って見逃す／FATAL ERROR にする項目**を makeobj には依存せず、`.dat` 構文を独自に解析します。

`lint` は makeobj が認識する全 22 obj 種別をカバーしています。各 obj 種別の検出項目・makeobj ソース上の根拠は
`src/rules/<obj種別>.rs` のドキュメントコメントを参照してください（開発者向けの詳細は
[CLAUDE.md](CLAUDE.md)）。

## 3層の役割

| 層        | サブコマンド | 役割                                                                  |
| --------- | ------------ | --------------------------------------------------------------------- |
| formatter | `fmt`        | 見やすく整形（キー小文字化・`=`前後の空白除去・デフォルトで並び替え） |
| linter    | `lint`       | pak 化に失敗する／ゲーム内で正しく表示されない項目を検知              |
| 静的解析  | `analyze`    | 実行しなくても分かるゲーム内利用時の問題を検知                        |

`lint` は1ファイル単位の検証、`analyze` はより高度な解析を行うコマンドです。（今は連結制約の充足可能性のチェック機能のみ）

## インストール

リリースページからOSにあった実行ファイルをダウンロードしてください。
上級者向け：パスを通しておくとどこからでも呼び出せます。

## 使い方

すべてのサブコマンド（`lint` / `fmt` / `analyze` / `list` / `describe`）は明示的に
指定する必要があります。

`lint`/`fmt`/`analyze` は指摘が1件も無ければ（error/warning/unsupportedが全て0件）
stdout に一切出力しません（サイレント成功。CI・スクリプトでの利用を想定した
Unix系リンタの一般的な流儀）。指摘が1件でもあれば従来通り出力し、診断本文は
すべて stderr に出ます（stdout には情報メッセージ・サマリ行のみが出ます）。

```
dat_linter --help
dat_linter lint --help
```

### `lint` — 静的検証

```
dat_linter lint <path/to/file.dat>
dat_linter lint -v  <path/to/file.dat>   # info まで表示
dat_linter lint -vv <path/to/file.dat>   # debug（生の値・解決後パス）まで表示
```

#### 複数ファイル一括処理

`lint` の引数には単一ファイルパスの他に、ディレクトリパスや glob パターンも指定できます。

```
dat_linter lint path/to/dir             # ディレクトリ内の.datを再帰的に収集して検証
dat_linter lint "path/to/*.dat"         # globパターン（"**/*.dat"のような再帰パターンも可）
```

PowerShell は Unix シェルと異なり `*` をシェル側で自動展開しないため、`dat_linter`
自身が glob パターンを解釈します。収集した `.dat` ファイル一覧はパスの辞書順で処理します。

複数ファイルを指定した場合、各ファイルの診断・サマリ行は従来通りファイル単位で出力され、
末尾に全体の集計行が追加されます（`合計: 対象ファイル 4 件 / error 12 件 / warning 3 件`）。
1ファイルでも error または warning が1件でもあれば、全体の終了コードは失敗（`1`）になります。

#### 設定ファイルによるルールの include/exclude

`--config <path>` で TOML 形式の設定ファイルを指定すると、`Diagnostic.code`
（各診断ルールの一意なID。例 `obsolete-type` / `missing-tile-image`）単位で
診断の有効/無効を制御できます。`--config` を省略した場合、カレントディレクトリ直下の
`dat_linter.toml` を自動探索します（存在しなければ全ルール有効のまま動作します）。

```toml
# dat_linter.toml
[rules]
include = ["obsolete-type", "missing-waytype"]  # 空なら全ルールが候補（デフォルト）
exclude = ["duplicate-key"]                      # includeの結果からさらに除外
```

意味論:

1. `include` が空なら全ての `code` が候補。非空なら `include` に列挙された `code` のみが候補。
2. 候補集合から `exclude` に列挙された `code` をさらに除外する（`exclude` は常に `include`
   より優先）。

```
dat_linter lint --config dat_linter.toml path/to/file.dat
```

この include/exclude は `lint` だけでなく `fmt`・`analyze` にも全く同じ意味論で適用されます。
デフォルト（`--config`未指定・include/exclude空）は3サブコマンドとも「全code許可」です。

```
dat_linter fmt --config dat_linter.toml path/to/file.dat
dat_linter analyze --config dat_linter.toml path/to/vehicle_dir
```

#### `dat_linter list` — include/exclude に書ける code 一覧を表示する

`dat_linter.toml` の `[rules] include/exclude` に指定できる `code` 一覧は
`dat_linter list` で確認できます（`--source lint|fmt|analyze` で絞り込み可能。
省略時は全件表示）。`--config` を指定すると、各 `code` が現在の設定で
有効(`enabled`)か無効(`disabled`)かも併記されます。

```
dat_linter list                              # 全code一覧
dat_linter list --source fmt                 # fmtのcodeのみ
dat_linter list --config dat_linter.toml     # 現在の設定での有効/無効も表示
```

#### `dat_linter describe <code>` — codeの説明（なぜNGか・どう直すか）を表示する

```
dat_linter describe obsolete-type            # 指定codeの説明を表示
dat_linter describe --config dat_linter.toml missing-waytype   # configのlanguageに従う
dat_linter describe not-a-real-code          # 不明なcode: エラー終了 + list案内
```

#### 設定ファイルの自動生成

`--config` 未指定かつカレントディレクトリに `dat_linter.toml` が無い状態で
`lint`/`fmt`/`analyze` のいずれかを実行すると、コメント付きのデフォルト設定ファイルを
カレントディレクトリに自動生成します。生成に失敗しても致命的エラーにはせず、
設定ファイル無しの状態のまま動作を継続します。

#### 出力言語

設定ファイルの `[general]` セクションで、診断メッセージ・サマリ行・`--help` の
出力言語を切り替えられます。

```toml
# dat_linter.toml
[general]
language = "en"  # "en"（デフォルト） または "ja"
```

設定ファイルが無い場合・`language` キー未指定の場合のデフォルトは英語（`en`）です。
不正な値（`"en"`/`"ja"` 以外）が指定された場合も英語にフォールバックします。

`lint`/`fmt`/`analyze`/`list` の全サブコマンドが `--config`（またはカレントディレクトリの
`dat_linter.toml` 自動探索）でこの設定を共有します。ただし `lint --help` が表示する
22obj種別の長い一覧文は翻訳対象外で、常に日本語のまま表示されます。

### `fmt` — フォーマッタ

```
dat_linter fmt <file.dat>                    # 慣習的な順序に並び替え（標準出力へ。デフォルト挙動）
dat_linter fmt --no-reorder <file.dat>       # 並び替えず元の順序のまま正規化
dat_linter fmt --write <file.dat>            # ファイルに上書き（-w も可）
dat_linter fmt <dir> --write                 # ディレクトリ内の.datを再帰的に整形して書き戻す
dat_linter fmt "path/to/*.dat" --write       # globパターンも指定できる（lintと同じcollect_dat_paths）
```

安全な正規化（キー小文字化・`=`前後の空白除去・値の前後トリム）のみ行い、値の内容変更のような
壊しうる操作は行いません。慣習的な順序への並び替えが**デフォルト挙動**です（並び替え自体は
スタイル上の慣習であり makeobj の動作には影響しません）。並び替えを無効化したい場合は
`--no-reorder`（そのプロセスの実行に限り無効化）を指定するか、`dat_linter.toml`の
`[rules] exclude = ["fmt-reorder-applied"]`で恒久的に無効化してください
（優先順位: `--no-reorder`指定 > config設定）。
並び順は `obj=` の値ごとに定義されており、未対応の obj 種別では並び替えを行わず
元の順序のまま出力します。

複数ファイルに解決された場合、`--write`（`-w`）を指定しないとエラー終了します
（整形結果を複数ファイル分stdoutへ混在させて出すのは実用性が低いため）。

`#` 始まりのコメント行は、直後に現れる最初の `key=value` 行に紐づき、そのPairと一緒に
並び替え後も移動します。紐づけ先が無いと判断される場合（不正行の直前・複数obj連結の
区切り行をまたぐ場合・セグメント末尾の場合）は、他の行と同様に出力から削除され、
削除した件数が警告として表示されます。

### `analyze` — ディレクトリ横断解析

```
dat_linter analyze <path/to/vehicle_dat_dir>   # --kindは現状coupling固定のデフォルトなので指定不要
```

1ディレクトリ内の複数 `.dat` を横断的に解析するサブコマンドです。`--kind`は将来の解析種別
追加に備えたオプションで、現状は`coupling`のみ・デフォルトのため通常は指定不要です。

#### 車両連結制約の静的解析（`--kind coupling`）

1ディレクトリ内の全 `obj=vehicle` を読み込み、`constraint[prev]` / `constraint[next]` について:

1. **dangling 参照チェック**: 参照先の車両名がディレクトリ内に実在するか（makeobj は検証しない）
2. **充足可能性チェック**: 到達可能性解析により「有限な編成として絶対に成立しない車両」が無いか

を検査します。

## ログレベルと終了コード

| level | 表示条件 | 用途                                           |
| ----- | -------- | ---------------------------------------------- |
| error | 既定     | pak 化に失敗する／ゲーム内で正常に表示されない |
| warn  | 既定     | 非推奨・設定が推奨される項目                   |
| info  | `-v`     | 各チェックの合格確認                           |
| debug | `-vv`    | 生の値・解決後パス・索いたキー名               |

終了コードは `lint` / `fmt` / `analyze` のいずれも、`error` または `warn` の診断が
1件でもあれば `1`、指摘が1件も無ければ `0` になります。

### 出力チャンネル（stdout/stderr）

- **診断本文**（error/warn/info/debug）は `lint`/`fmt`/`analyze` いずれも **stderr** に出力されます。
- **情報メッセージ**（"OK" 相当の合格通知、件数サマリ行など）は **stdout** に出力されます。
  ただし指摘が1件も無い場合はこれらの情報メッセージ自体も省略されます（サイレント成功）。
- `fmt` を `--write`（`-w`）無しで単一ファイルに対して実行した場合の整形結果本体は、
  上記の分類とは独立に常に stdout に出力されます。

## 対応範囲

- `lint`: `obj=building`（`type=extension`/`stop`/`depot` 系）、`obj=vehicle`、`obj=way`、
  `obj=good`、`obj=bridge`、`obj=tunnel`、`obj=roadsign`、`obj=crossing`、`obj=way-object`、
  `obj=ground_obj`、`obj=tree`、`obj=citycar`、`obj=pedestrian`、`obj=factory`、`obj=sound`、
  `obj=ground`、`obj=menu`、`obj=cursor`、`obj=symbol`、`obj=smoke`、`obj=field`、`obj=misc`
  （makeobj が認識する全22 obj 種別）
- `analyze --kind coupling`: `obj=vehicle` の `constraint[prev]` / `constraint[next]`

各ルールは makeobj の C++ ソースで根拠を確認しています。大半のルールは makeobj 自体
（コンパイル時）の `dbg->fatal`/`dbg->warning` を根拠としますが、`vehicle` の
`power-gear-mismatch` と `factory` の `productivity-zero` の2ルールのみ、ゲームエンジンの
ランタイムコードを根拠とする「静的解析」層の例外です。

各 obj 種別の検出項目の詳細・意図的に対応していない項目とその理由は
`src/rules/<obj種別>.rs` のドキュメントコメント（`REJECTED` として理由付きで記録）を
参照してください。

## ライセンス

MIT License（[LICENSE](LICENSE) 参照）
