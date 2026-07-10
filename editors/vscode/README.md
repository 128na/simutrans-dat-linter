# Simutrans dat_linter (VSCode 拡張)

Simutrans アドオンの `.dat` を静的検証する Rust 製 CLI [`dat_linter`](https://github.com/128na/simutrans-dat-linter)
の診断結果を、VSCode の Problems パネルへ統合する拡張機能です。

`.dat` ファイルを開く・保存するたびに `dat_linter lint --format json` をバックグラウンドで実行し、
結果をエディタ上に波線・Problems パネルの一覧として表示します。

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
| `simutransDatLinter.configPath` | `""`（未指定） | `--config` に渡す `dat_linter.toml` の明示パス。**未指定の場合、`dat_linter` 自身がワークスペースフォルダのルート直下の `dat_linter.toml` を自動探索し、見つからなければそこへ自動生成します。** ルール設定を制御したい場合や、意図しない `dat_linter.toml` 生成を避けたい場合は明示的に指定してください。 |

## 既知の制限

- 一部の診断（obj全体に関わる問題など、特定の行に紐づけられないもの）には行番号が付与されません。
  この拡張はそのような診断をファイル先頭（0行目）に表示します。ファイルが長い場合、該当箇所を
  目視で探す必要があります。
- 単一の `.dat` ファイルを開いたときのみ検証します（ディレクトリ一括 lint や `analyze`（連結制約解析）
  など、CLI が持つ他のコマンドはこの拡張からは呼び出されません）。

## 開発者向けメモ

エンドユーザー向けの内容ではなく、この拡張自体を開発・改修する際に踏みやすい罠のメモです。

- **`dat_linter.toml` 自動生成の罠。** `configPath` を指定しない状態で拡張やテストを動かすと、
  `dat_linter` がカレントディレクトリ（この拡張では workspace folder root、フォールバックで
  linted file のあるディレクトリ）に `dat_linter.toml` を自動生成してしまう。動作確認・自動テストの
  際は必ず明示的な `--config`（`simutransDatLinter.configPath` 経由）を指定し、意図しない場所に
  設定ファイルが生成されないようにすること。本リポジトリ直下や `testdata/` 配下に誤って生成して
  しまった場合は `git status` で新規ファイルであることを確認してから削除する。
- **`fixtures/dat_linter.toml` という名前は使えない。** リポジトリ直下の `.gitignore` に
  `/dat_linter.toml`（ルート直下限定）というルールがあるためこのディレクトリの直下では問題ないが、
  念のため PoC を踏襲し `fixtures/test-lint-config.toml` という名前にしている。
- **stdout/stderr の分離。** `--format json` は診断・サマリをまとめて **stdout のみ** に1回出力し、
  stderr へは何も出さない（プロセス自体の起動に失敗した場合を除く）。旧バージョンの PoC
  （`try-out/vscode-dat-linter-poc/`、テキスト出力を正規表現でパースする方式）では逆に
  診断本体が stderr、末尾のサマリ行のみ stdout という構成だったため、両ストリームを連結して
  パースしていた。JSON 対応版ではその必要はなく、stdout のみをパースする。
- **バージョン互換性の検出はヒューリスティック。** `dat_linter` 実行が失敗した際、
  「実行ファイルが見つからない」のか「古いバージョンで `--format` 自体を認識しない」のかを
  stderr の文言（clap のエラーメッセージ）から推測して分かりやすいメッセージを出しているが、
  完全な判定ではない（`src/extension.ts` の `describeFailure` 参照）。
