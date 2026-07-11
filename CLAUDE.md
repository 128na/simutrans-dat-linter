# CLAUDE.md

このファイルは開発者（Claude Code含む）向けの情報です。利用者向けの使い方は
[README.md](README.md) を参照してください。

## 検証根拠の一次情報はソースコード側にある

各 obj 種別の検出項目・makeobj ソース上の根拠・調査の経緯は、README には書かず
`src/rules/<obj種別>.rs` のモジュール/アイテムドキュメントコメントに直接記載しています。
「なぜこのルールを追加したか」だけでなく「なぜ追加しなかったか」も、各ファイル内に
`REJECTED` として理由付きで記録しています。obj種別の検証ロジックを読む・変更・追加する際は、
README ではなくまずそのファイルのドキュメントコメントを読んでください（README・CLAUDE.md 側に
同じ内容を重複して書かないこと）。

全体の検証方針（vanilla simutrans の pinned commit
`1d2799f9a73adf94751e2d8357fea9dabcc4f740` の `descriptor/writer/*.cc` を直接ミラーする、等）は
`src/rules/mod.rs` 冒頭コメント参照。

新しい obj 種別を追加する場合は `rules/<obj種別>.rs` を新設し、
`registry::RuleSet::for_obj_type` にディスパッチを追加してください。

## アーキテクチャ

```
src/
  main.rs                 バイナリ入口。`--config`の先読み→言語決定→clapパース→
                          各サブコマンドへのディスパッチのみを行う薄いエントリポイント
  cli.rs                  clapの`Cli`/`Command`/各`*Args`構造体、ヘルプ文言のJA/EN定数、
                          `apply_language_to_help`、`peek_config_arg`
  fs_walk.rs              `lint`/`fmt`共通のファイル収集ユーティリティ（単一ファイル・
                          ディレクトリ再帰・globパターンの3通りをdatファイル一覧へ解決）
  commands/
    lint.rs                 `run_lint`
    fmt.rs                  `run_fmt`
    analyze.rs              `run_analyze`（--kindで解析種別を選択。現状coupling一種のみ）
    list.rs                 `run_list`
    describe.rs              `run_describe`
  config.rs               lint/fmt/analyze共通設定（TOML）。[rules]診断ルールinclude/exclude
                          （Diagnostic.code単位）・[general] languageの2セクション。
                          fmtのreorder挙動も専用フィールドではなく[rules] exclude内の
                          "fmt-reorder-applied"というcodeで制御する。初回起動時の
                          dat_linter.toml自動生成もここが担う
  codes.rs                 全Diagnostic.codeの一覧（ALL_CODES）。dat_linter list/describeが
                          参照する。各codeにwhy（なぜNGか）・how_to_fix（どう直すか）を
                          JA/EN両方で保持し、`tests/codes_completeness.rs`が実ソースとの
                          整合性・説明文の非空を保証する
  i18n.rs                  Language enum（en/ja、デフォルトen）とt!マクロ
  registry.rs              Rule trait・RuleContext（language含む）・obj種別ディスパッチ
  parser.rs                .dat パーサ（先勝ち・行番号追跡・重複キー検出）
  param_expansion.rs        tabfile_t::read()のパラメータ展開構文
                          （数値カンマリスト・ダッシュ範囲・方向名(ribi)文字列リスト・
                          <$N>算術参照）の再現
  diagnostics.rs           Diagnostic・Severity・Location
  rules/<obj種別>.rs       各obj種別のRule実装。検証根拠・調査経緯はここに記載（上記参照）
  rules/common.rs          共有定数・ヘルパー（KNOWN_WAYTYPES等）・duplicate-key検出
  couplings.rs             vehicle連結制約のグラフ解析（lintとは別スコープ）
  formatter/
    mod.rs                   パース・正規化ロジック
    order.rs                  obj種別ごとの並び順定義
```

## テストを書く際の注意点

`tests/cli_integration.rs`で`dat_linter`バイナリを起動するテストは、**必ず`current_dir`を
明示的に指定すること**（`bin()`をそのまま`.output()`しない）。指定しないと、テストプロセスの
実cwd（クレートルート）を見に行ってしまい、開発者がローカルで手動テスト用に置いている
`dat_linter.toml`（`.gitignore`対象で`git status`には出ないが実在しうる、`language = "ja"`等）を
拾って挙動が変わる。実際にこのセッション中、意図しない`dat_linter.toml`の生成・混入が複数回
発生し、5件（後に洗い出しでもう1件追加）のテストがローカル環境依存で壊れる不具合を引き起こした。
既存のヘルパー: config込みの挙動を検証したいときは`run_with_ja_config`、config無し（デフォルト）
の挙動を検証したいときは`run_in_clean_dir`を使うこと。

## 開発コマンド

```
cargo test                                   # 統合テスト（tests/*.rs）
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

`testdata/` に正常系・意図的に壊した系・フォーマッタ用・連結制約用の `.dat`／`.png` を用意しています。
`testdata/real_data/` は `simutrans_addon` ワークスペースの `refs/linter_test/`
（gitignore対象の参照アドオン置き場）にある実データをコピーした回帰テスト専用フィクスチャで、
`tests/real_data_regression.rs` から参照します。**`testdata/real_data/` はサードパーティ製の
実アドオンデータのため`.gitignore`対象で、git管理には含めていません**（ローカル検証専用）。
このディレクトリが無い環境（CI・新規クローン時）では、`tests/real_data_regression.rs`の各テストは
フィクスチャ欠如を検知して理由を出力した上で自動的にスキップされ、`cargo test`全体はfailしません。

CI は Linux / Windows の両方でビルド・テストします。

## リリース手順

バージョン管理・タグ打ちには [cargo-release](https://github.com/crate-ci/cargo-release) を使います
（`cargo install cargo-release`で導入）。`release.toml`で`publish = false`固定にしており、
crates.io へは公開しません（GitHub Release でのバイナリ配布のみ）。

```
# まずdry-runで変更内容を確認（デフォルトでdry-run、何も実行されない）
cargo release patch   # あるいは minor / major

# 問題なければ--executeで実行
# Cargo.toml の version 更新 → コミット → v{version} 形式のタグ作成 → push まで一括で行う
cargo release patch --execute
```

`v*`形式のタグが push されると `.github/workflows/release.yml` が Linux/Windows 向けの
release ビルドを行い、バイナリを添付した GitHub Release を自動作成します。
