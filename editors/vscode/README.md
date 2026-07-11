# Simutrans dat_linter (VSCode 拡張)

Simutrans アドオンの `.dat` を静的検証・整形する Rust 製 CLI
[`dat_linter`](https://github.com/128na/simutrans-dat-linter) の結果を、VSCode の Problems パネルと
Document Formatting に統合する拡張機能です。

`.dat` ファイルを開く・保存するたびに `dat_linter lint --format json` をバックグラウンドで実行し、
結果をエディタ上に波線・Problems パネルの一覧として表示します。

また `dat_linter fmt` を使った Document Formatting（正規化・キー並び替え）にも対応しています。
VSCode の `editor.formatOnSave` を `true` にしておけば保存時に自動整形されますし、
コマンドパレットから `Format Document` を実行すれば手動整形もできます。改行コード（CRLF/LF）は
入力ファイルのものがそのまま保持されます。

## 前提条件

この拡張は `dat_linter` 本体を同梱しません。事前に別途インストールし、PATH に通しておく必要があります。

- 本体・インストール方法: https://github.com/128na/simutrans-dat-linter
  （リリースページから OS にあった実行ファイルをダウンロードしてください）
- `--format json` オプションは `dat_linter 0.1.2` 以降が対応しています。それより古いバージョンでは
  この拡張は動作しません（後述「既知の制限」参照）。

## 設定項目

`Ctrl+,` (Settings) から `Simutrans dat_linter` で検索するか、`settings.json` に直接記述してください。

| 設定キー | 既定値 | 説明 |
| --- | --- | --- |
| `simutransDatLinter.executablePath` | `"dat_linter"` | `dat_linter` 実行ファイルのパス。既定では PATH 上のものを使用します。 |
| `simutransDatLinter.configPath` | `""`（未指定） | `--config` に渡す `dat_linter.toml` の明示パス。**未指定の場合、`dat_linter` 自身がワークスペースフォルダのルート直下の `dat_linter.toml` を自動探索します。見つからない場合は自動生成せず、全ルール有効・`language=en` のデフォルト設定のまま動作します。** そのディレクトリに雛形を作りたい場合はターミナルから `dat_linter init` を実行してください。ルール設定を制御したい場合は `configPath` を明示的に指定するか、ワークスペースフォルダのルートで `dat_linter init` を実行してください。lint・フォーマッタの両方がこの設定を共有します。 |

`fmt` のキー並び替え（reorder）専用の設定項目はこの拡張には存在しません。無効化したい場合は
`dat_linter.toml` 側の `[rules] exclude` に `"fmt-reorder-applied"` を追加してください
（`dat_linter` 本体の README・`dat_linter list` 参照）。

## 既知の制限

- 一部の診断（obj全体に関わる問題など、特定の行に紐づけられないもの）には行番号が付与されません。
  この拡張はそのような診断をファイル先頭（0行目）に表示します。ファイルが長い場合、該当箇所を
  目視で探す必要があります。
- 単一の `.dat` ファイルを開いたときのみ検証・整形します（ディレクトリ一括 lint/fmt や `analyze`
  （連結制約解析）など、CLI が持つ他のコマンドはこの拡張からは呼び出されません）。
- フォーマッタは `dat_linter fmt <path> [--config ...]`（`-w`/`--write` なし）の標準出力で
  ドキュメント全体を置き換えます。ファイルへの直接書き込みは行わないため、実際にディスクへ
  反映するには VSCode 側で保存（`editor.formatOnSave` または手動保存）が必要です。

## 開発者向けメモ

エンドユーザー向けの内容ではなく、この拡張自体を開発・改修する際に踏みやすい罠のメモです。

- **`dat_linter.toml` の自動探索（自動生成はしない）。** `configPath` を指定しない状態で拡張や
  テストを動かすと、`dat_linter` はカレントディレクトリ（この拡張では workspace folder root、
  フォールバックで linted file のあるディレクトリ）の `dat_linter.toml` を自動探索する。見つからなくても
  そこへ自動生成することはなく、全ルール有効・`language=en` のデフォルト設定にフォールバックするだけ
  （`dat_linter`本体側でこの暗黙生成は廃止済み。生成は明示的な`dat_linter init`サブコマンドに一本化
  されている）。動作確認・自動テストで特定のルール設定を効かせたい場合は、明示的な `--config`
  （`simutransDatLinter.configPath` 経由）を指定すること。
- **`fixtures/dat_linter.toml` という名前は使えない。** リポジトリ直下の `.gitignore` に
  `/dat_linter.toml`（ルート直下限定）というルールがあるためこのディレクトリの直下では問題ないが、
  念のため PoC を踏襲し `fixtures/test-lint-config.toml` という名前にしている。
- **stdout/stderr の分離。** `--format json` は診断・サマリをまとめて **stdout のみ** に1回出力し、
  stderr へは何も出さない（プロセス自体の起動に失敗した場合を除く）。旧バージョンの PoC
  （`try-out/vscode-dat-linter-poc/`、テキスト出力を正規表現でパースする方式）では逆に
  診断本体が stderr、末尾のサマリ行のみ stdout という構成だったため、両ストリームを連結して
  パースしていた。JSON 対応版ではその必要はなく、stdout のみをパースする。
- **バージョン互換性の検出はヒューリスティック。** `dat_linter` 実行が失敗した際、
  「実行ファイルが見つからない」のか「古いバージョンで引数を認識しない」のかを
  stderr の文言（clap のエラーメッセージ）から推測して分かりやすいメッセージを出しているが、
  完全な判定ではない（`src/runner.ts` の `describeFailure` 参照。lint/fmt 双方の呼び出しが
  この共通ヘルパーを使い、コマンドごとの stderr パターン・メッセージだけを個別に渡す）。
- **cwd/config 解決ロジックの共有元は `src/runner.ts`。** `resolveExecutionContext` が
  workspace folder root（無ければ linted file のあるディレクトリ）と
  `simutransDatLinter.executablePath`/`configPath` 設定を解決する。`src/extension.ts`（lint）と
  `src/formatter.ts`（fmt）の両方がこれを使うため、cwd/config 周りの挙動を変える場合はここ1箇所を
  直せば両方に反映される。
- **フォーマッタの統合テストは "vscode.executeFormatDocumentProvider" の生の結果を直接は使えない。**
  このコマンドは provider が返した TextEdit を、VSCode 側で元の内容との最小差分へ変換してから返す
  （このプロバイダは常にドキュメント全体を置き換える単一の TextEdit を返すが、コマンド経由で受け取る
  頃には複数の小さな TextEdit に分割されている）。そのため `test/extension.test.ts` のフォーマッタ
  テストは、TextEdit の形状を検証するのではなく `editor.action.formatDocument` を実行してから
  `document.getText()` で最終的な中身を読み取り、CLI を直接実行して得た期待値と比較している。
