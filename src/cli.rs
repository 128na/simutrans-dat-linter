//! clapによるCLI定義（サブコマンド構造体・引数・ヘルプ文言のJA/EN定数・
//! 言語に応じたヘルプ翻訳の適用）。
//!
//! 第13弾: `src/main.rs`が肥大化（CLI引数定義＋全サブコマンドの実行ロジック＋
//! ファイル収集ユーティリティが同居）していたSRP的な分割候補だったため、
//! 責務ごとにモジュールを分けた。このモジュールは「CLIの形」（clap構造体・
//! ヘルプ文言・`--help`翻訳）のみを担い、各サブコマンドの実行ロジックは
//! `src/commands/*.rs`に、ファイル収集ユーティリティは`src/fs_walk.rs`に分離した。
//! 振る舞い（stdout/stderr出力・exit code・ヘルプ文言）は分割前後で完全に同一。

use clap::{ArgAction, Parser, Subcommand};
use dat_linter::i18n::Language;
use std::path::PathBuf;

/// トップレベル`Cli`のabout（短い1行説明）。`--help`翻訳対象
/// （`apply_language_to_help`が言語に応じて実際に使う方を選ぶ。derive由来の
/// `#[command(... about)]`は英語のデフォルト値として残しつつ、日本語選択時は
/// この定数で上書きする）。
const CLI_ABOUT_JA: &str = "Simutrans アドオンの .dat を静的検証・整形・連結解析するCLIツール";
const CLI_ABOUT_EN: &str =
    "Static validator, formatter, and coupling analyzer for Simutrans .dat files";

#[derive(Parser)]
#[command(name = "dat_linter", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    // Note: clapのderiveマクロはdocコメントをコンパイル時の静的文字列としてしか
    // 扱えないため、このヘルプ文言のobj種別一覧は`registry::SUPPORTED_OBJ_TYPES`から
    // 動的に構築できない。obj種別を追加・変更する際は、必ず
    // `registry::SUPPORTED_OBJ_TYPES`（正）と手動で同期させること
    // （実行時のエラーメッセージは`SUPPORTED_OBJ_TYPES`から動的に構築しており、
    // ズレは`tests/obj_type_coverage.rs`で検出できる）。
    //
    // 翻訳方針: ここに書く doc コメント（1行目 = `about`）は短い説明のみで、
    // 22obj種別の長い一覧は `long_about` 側（`LINT_LONG_ABOUT_JA`）に分離した。
    // `about`（この1行）は`apply_language_to_help`が言語に応じて動的に
    // 差し替える翻訳対象。`long_about`（22obj種別の一覧を含む長文）は
    // 翻訳対象外で、常に日本語のまま出力される（コーディネーター指示）。
    /// .dat ファイル1件を静的検証する
    #[command(long_about = LINT_LONG_ABOUT_JA)]
    Lint(LintArgs),
    /// .dat ファイルを正規化・並び替えする
    Fmt(FmtArgs),
    /// 1ディレクトリ内のdatファイル群を横断的に解析する（種別は--kindで選択）
    Analyze(AnalyzeArgs),
    /// dat_linter.toml の [rules] include/exclude に書けるcode一覧を表示する
    List(ListArgs),
    /// 指定したcodeの説明（なぜNGか・どう直すか）を表示する
    Describe(DescribeArgs),
}

/// `lint`の長い説明（22obj種別の一覧を含む）。翻訳対象外、常に日本語のまま
/// （コーディネーター指示: 短いabout一行のみ翻訳し、この長文はJP固定でよい）。
const LINT_LONG_ABOUT_JA: &str = ".dat ファイル1件を静的検証する（obj=building / obj=vehicle / obj=way / obj=good / obj=bridge / obj=tunnel / obj=roadsign / obj=crossing / obj=way-object / obj=ground_obj / obj=tree / obj=citycar / obj=pedestrian / obj=factory / obj=sound / obj=ground / obj=menu / obj=cursor / obj=symbol / obj=smoke / obj=field / obj=misc）";

/// 各subcommandの短い`about`（JA/EN）。`apply_language_to_help`から参照する。
const LINT_ABOUT_JA: &str = ".dat ファイル1件を静的検証する";
const LINT_ABOUT_EN: &str = "Statically validate a single .dat file";
const FMT_ABOUT_JA: &str = ".dat ファイルを正規化・並び替えする";
const FMT_ABOUT_EN: &str = "Normalize and reorder a .dat file";
const ANALYZE_ABOUT_JA: &str =
    "1ディレクトリ内のdatファイル群を横断的に解析する（種別は--kindで選択）";
const ANALYZE_ABOUT_EN: &str =
    "Analyze dat files across a directory (select the analysis kind with --kind)";
const LIST_ABOUT_JA: &str = "dat_linter.toml の [rules] include/exclude に書けるcode一覧を表示する";
const LIST_ABOUT_EN: &str =
    "List the codes that can be used in dat_linter.toml's [rules] include/exclude";
const DESCRIBE_ABOUT_JA: &str = "指定したcodeの説明（なぜNGか・どう直すか）を表示する";
const DESCRIBE_ABOUT_EN: &str =
    "Show the description (why it's flagged, how to fix it) for the given code";

/// 各引数の短い`help`（JA/EN）。第9弾（項目1）: 第2弾では「サブコマンドの
/// aboutの一行説明のみ翻訳対象」と決めていたが、これは`lint`/`fmt`/`analyze`
/// 自体の説明を指しており、個々の引数（`--kind`/`--config`/`-v`等）のhelp文字列は
/// 対象外のまま日本語ハードコードで残っていた（analyzeは第5弾の新設時点で
/// この整理に追随していなかった）。ここで同じ粒度（短い一行）で全引数のhelpも
/// 翻訳対象に揃える。詳細な設計理由（複数段落のdocコメント）はソース上の
/// コメントとして残し、`--help`に出す文言自体は短い一文に統一する。
const LINT_PATH_HELP_JA: &str =
    "単一ファイル・ディレクトリ・globパターン（例 `refs/*.dat`, `refs/**/*.dat`）";
const LINT_PATH_HELP_EN: &str =
    "A single file, directory, or glob pattern (e.g. `refs/*.dat`, `refs/**/*.dat`)";
const LINT_VERBOSE_HELP_JA: &str = "-v: info まで表示 / -vv: debug まで表示";
const LINT_VERBOSE_HELP_EN: &str = "-v: show up to info / -vv: show up to debug";
const CONFIG_HELP_JA: &str = "ルールのinclude/exclude等の設定ファイル（TOML）。省略時はカレントディレクトリの dat_linter.toml を自動探索する";
const CONFIG_HELP_EN: &str = "Configuration file (TOML) for rule include/exclude etc. If omitted, dat_linter.toml in the current directory is auto-discovered";
const FMT_PATH_HELP_JA: &str = "単一ファイル・ディレクトリ・globパターン（例 `refs/*.dat`, `refs/**/*.dat`）。複数ファイルに解決された場合は --write が必須";
const FMT_PATH_HELP_EN: &str = "A single file, directory, or glob pattern (e.g. `refs/*.dat`, `refs/**/*.dat`). --write is required when multiple files match";
const FMT_NO_REORDER_HELP_JA: &str =
    "並び替えを無効化し、元の行順を保持する（configの[fmt] reorder設定をこの実行に限り上書き）";
const FMT_NO_REORDER_HELP_EN: &str = "Disable reordering and keep the original line order (overrides [fmt] reorder for this run only)";
const FMT_WRITE_HELP_JA: &str = "フォーマット結果をファイルへ書き込む";
const FMT_WRITE_HELP_EN: &str = "Write the formatted output back to the file";
const ANALYZE_DIR_HELP_JA: &str = "解析対象のディレクトリ";
const ANALYZE_DIR_HELP_EN: &str = "Directory to analyze";
const ANALYZE_KIND_HELP_JA: &str =
    "解析種別。現状は coupling（obj=vehicle の連結制約解析）のみ対応";
const ANALYZE_KIND_HELP_EN: &str = "Analysis kind. Currently only `coupling` (obj=vehicle coupling constraint analysis) is supported";
const LIST_SOURCE_HELP_JA: &str = "表示するcodeの由来を絞り込む（省略時は全て表示）";
const LIST_SOURCE_HELP_EN: &str = "Filter which source's codes to show (omit to show all)";
const DESCRIBE_CODE_HELP_JA: &str =
    "説明を表示するcode（例 obsolete-type）。一覧は`dat_linter list`で確認できる";
const DESCRIBE_CODE_HELP_EN: &str =
    "The code to describe (e.g. obsolete-type). See `dat_linter list` for the full list";

/// `Cli::command()`が返す`clap::Command`の短い`about`を言語に応じて上書きする。
/// `long_about`（22obj種別一覧を含む長文）には触れない（翻訳対象外のため常に日本語）。
///
/// clapの`derive(Parser)`が生成する`Cli::parse()`は内部で`--help`/`-h`検出時に
/// 即座に`Command::print_help()`し`process::exit()`するため、`Cli::parse()`
/// そのものを呼ぶと事前に言語を反映する機会が無い。この関数を`Cli::command()`の
/// 戻り値に適用してから`.get_matches()`することで、`--help`表示にも
/// 翻訳後の文言を反映できる（`main()`参照）。
///
/// 第9弾（項目1）でサブコマンドのaboutと同じ仕組み（`mut_subcommand`）を使い、
/// 各引数のhelpも`mut_arg`で上書きするよう拡張した。引数IDは`clap::Args`の
/// フィールド名（スネークケース）がそのまま使われる（`#[arg(long = "...")]`で
/// 明示的にlong名を変えていても、`mut_arg`が参照するIDはフィールド名のまま）。
pub fn apply_language_to_help(cmd: clap::Command, lang: Language) -> clap::Command {
    let top_about = match lang {
        Language::Japanese => CLI_ABOUT_JA,
        Language::English => CLI_ABOUT_EN,
    };
    let config_help = match lang {
        Language::Japanese => CONFIG_HELP_JA,
        Language::English => CONFIG_HELP_EN,
    };
    cmd.about(top_about)
        .mut_subcommand("lint", |c| {
            let about = match lang {
                Language::Japanese => LINT_ABOUT_JA,
                Language::English => LINT_ABOUT_EN,
            };
            let path_help = match lang {
                Language::Japanese => LINT_PATH_HELP_JA,
                Language::English => LINT_PATH_HELP_EN,
            };
            let verbose_help = match lang {
                Language::Japanese => LINT_VERBOSE_HELP_JA,
                Language::English => LINT_VERBOSE_HELP_EN,
            };
            c.about(about)
                .mut_arg("path", |a| a.help(path_help))
                .mut_arg("verbose", |a| a.help(verbose_help))
                .mut_arg("config", |a| a.help(config_help))
        })
        .mut_subcommand("fmt", |c| {
            let about = match lang {
                Language::Japanese => FMT_ABOUT_JA,
                Language::English => FMT_ABOUT_EN,
            };
            let path_help = match lang {
                Language::Japanese => FMT_PATH_HELP_JA,
                Language::English => FMT_PATH_HELP_EN,
            };
            let no_reorder_help = match lang {
                Language::Japanese => FMT_NO_REORDER_HELP_JA,
                Language::English => FMT_NO_REORDER_HELP_EN,
            };
            let write_help = match lang {
                Language::Japanese => FMT_WRITE_HELP_JA,
                Language::English => FMT_WRITE_HELP_EN,
            };
            c.about(about)
                .mut_arg("path", |a| a.help(path_help))
                .mut_arg("no_reorder", |a| a.help(no_reorder_help))
                .mut_arg("write", |a| a.help(write_help))
                .mut_arg("config", |a| a.help(config_help))
        })
        .mut_subcommand("analyze", |c| {
            let about = match lang {
                Language::Japanese => ANALYZE_ABOUT_JA,
                Language::English => ANALYZE_ABOUT_EN,
            };
            let dir_help = match lang {
                Language::Japanese => ANALYZE_DIR_HELP_JA,
                Language::English => ANALYZE_DIR_HELP_EN,
            };
            let kind_help = match lang {
                Language::Japanese => ANALYZE_KIND_HELP_JA,
                Language::English => ANALYZE_KIND_HELP_EN,
            };
            c.about(about)
                .mut_arg("dir", |a| a.help(dir_help))
                .mut_arg("kind", |a| a.help(kind_help))
                .mut_arg("config", |a| a.help(config_help))
        })
        .mut_subcommand("list", |c| {
            let about = match lang {
                Language::Japanese => LIST_ABOUT_JA,
                Language::English => LIST_ABOUT_EN,
            };
            let source_help = match lang {
                Language::Japanese => LIST_SOURCE_HELP_JA,
                Language::English => LIST_SOURCE_HELP_EN,
            };
            c.about(about)
                .mut_arg("source", |a| a.help(source_help))
                .mut_arg("config", |a| a.help(config_help))
        })
        .mut_subcommand("describe", |c| {
            let about = match lang {
                Language::Japanese => DESCRIBE_ABOUT_JA,
                Language::English => DESCRIBE_ABOUT_EN,
            };
            let code_help = match lang {
                Language::Japanese => DESCRIBE_CODE_HELP_JA,
                Language::English => DESCRIBE_CODE_HELP_EN,
            };
            c.about(about).mut_arg("code", |a| a.help(code_help))
        })
}

#[derive(clap::Args)]
pub struct LintArgs {
    /// 単一ファイル・ディレクトリ・globパターン（例 `refs/*.dat`, `refs/**/*.dat`）を指定できる。
    /// PowerShellはシェル側で`*`を展開しないため、ツール自身がglobを解釈する
    /// （`glob`クレート使用。ディレクトリを指定した場合は再帰的に`.dat`を収集する）。
    pub path: String,
    /// -v: info まで表示 / -vv: debug まで表示
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    pub verbose: u8,
    /// ルールのinclude/exclude設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する（存在しなければ全ルール有効）。
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct FmtArgs {
    /// 単一ファイル・ディレクトリ・globパターン（例 `refs/*.dat`, `refs/**/*.dat`）を指定できる。
    /// `lint`と同じ`collect_dat_paths`で解決する（ディレクトリは再帰的に`.dat`を収集）。
    /// 複数ファイルに解決された場合、`--write`（`-w`）を指定しないとエラー終了する
    /// （整形結果を複数ファイル分stdoutへ混在させて出すのは実用性が低いため）。
    pub path: String,
    /// 並び替えを無効化し、元の行順を保持する（configの`[fmt] reorder`設定を
    /// このプロセスの実行に限り強制的に上書きする）。デフォルトでは
    /// `[fmt] reorder`（未設定時true）に従って並び替える。優先順位:
    /// `--no-reorder`指定 > config設定。
    #[arg(long)]
    pub no_reorder: bool,
    /// フォーマット結果をファイルへ書き込む
    #[arg(short = 'w', long)]
    pub write: bool,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// `analyze`が対応する解析種別。`registry::ObjType`と同じ「ワイルドカードarmを
/// 持たない網羅match」規約に従い、新しい解析種別を追加する際は`run_analyze`の
/// match（および将来追加されるであろう解析関数群）を必ず更新しないと
/// `cargo build`が非網羅match errorで失敗するようにする。
///
/// 現状は`Coupling`（旧`couplings`サブコマンド相当、`obj=vehicle`の連結制約解析）
/// の1種類のみ。将来的に別の横断解析（例: obj間の参照整合性チェック等）を
/// 追加する際にこの列挙へバリアントを増やす想定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum AnalyzeKind {
    /// `obj=vehicle`群の`constraint[prev]`/`constraint[next]`について、
    /// (1) makeobjが検証しないdangling参照の実在性、(2) 連結制約の充足可能性
    /// （有限な編成として絶対に成立しない車両が無いか）を検査する
    /// （旧`couplings`サブコマンド相当）。
    Coupling,
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    pub dir: PathBuf,
    /// 解析種別。現状は`coupling`（`obj=vehicle`の連結制約解析）のみ対応
    /// （将来の解析種別追加に備えたclap ValueEnum。デフォルトは`coupling`）。
    #[arg(long, value_enum, default_value_t = AnalyzeKind::Coupling)]
    pub kind: AnalyzeKind,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// 第9弾（項目2）: `dat_linter.toml`の`[rules] include/exclude`に書けるcode
/// （`Diagnostic.code`）の一覧が分かる手段が無かったため新設したサブコマンド。
/// `--source`で`lint`/`fmt`/`analyze`いずれかのcodeだけに絞り込める
/// （省略時は全て表示）。
#[derive(clap::Args)]
pub struct ListArgs {
    /// 表示するcodeの由来を絞り込む（省略時は全て表示）。
    #[arg(long, value_enum)]
    pub source: Option<ListSourceArg>,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// `--source`で選べる値。`codes::CodeSource`と1対1で対応するが、こちらは
/// clapの`ValueEnum`用にCLI引数値（`lint`/`fmt`/`analyze`という文字列）を
/// 持つ別の型として定義する（`codes::CodeSource`をライブラリ側に閉じ、
/// clap依存をcli.rs側だけに留めるため）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ListSourceArg {
    Lint,
    Fmt,
    Analyze,
}

impl ListSourceArg {
    /// **ワイルドカードarmを持たない網羅match**（このプロジェクトの規約）。
    pub fn to_code_source(self) -> dat_linter::codes::CodeSource {
        match self {
            ListSourceArg::Lint => dat_linter::codes::CodeSource::Lint,
            ListSourceArg::Fmt => dat_linter::codes::CodeSource::Fmt,
            ListSourceArg::Analyze => dat_linter::codes::CodeSource::Analyze,
        }
    }
}

/// 第10弾（項目6）: 指定したcodeの説明（なぜNGか・どう直すか）を表示する
/// `describe`サブコマンドの引数。
#[derive(clap::Args)]
pub struct DescribeArgs {
    /// 説明を表示するcode（例 obsolete-type）。一覧は`dat_linter list`で確認できる。
    pub code: String,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// `--config <path>`の値だけを事前に取り出す簡易スキャン。
///
/// 言語（`--help`の翻訳含む）は設定ファイルから決まるが、設定ファイルの
/// パスは`--config`オプションでユーザーが上書きできる。`Cli::parse()`は
/// `--help`検出時に内部でexitしてしまい、その前に設定ファイルを読む機会を
/// 与えてくれないため、本来のCLIパーサ（clap）を通す前に、`--config`の値
/// （在れば）だけを素朴な文字列走査で先読みする。ここでの走査結果は
/// 「どの設定ファイルを読むか」の判断にのみ使い、実際の引数解釈・
/// バリデーションは通常通り`clap`（`get_matches`/`from_arg_matches`）が行う。
pub fn peek_config_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--config" {
            return args.get(i + 1).map(PathBuf::from);
        }
        if let Some(v) = args[i].strip_prefix("--config=") {
            return Some(PathBuf::from(v));
        }
    }
    None
}
