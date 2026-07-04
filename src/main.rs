use clap::{ArgAction, CommandFactory, FromArgMatches, Parser, Subcommand};
use dat_linter::config::LintConfig;
use dat_linter::diagnostics::Severity;
use dat_linter::i18n::{Language, t};
use dat_linter::parser::{DatFile, read_dat_text};
use dat_linter::registry::{RuleContext, RuleSet, SUPPORTED_OBJ_TYPES};
use dat_linter::{couplings, formatter, rules};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "dat_linter", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// トップレベル`Cli`のabout（短い1行説明）。`--help`翻訳対象
/// （`apply_language_to_help`が言語に応じて実際に使う方を選ぶ。derive由来の
/// `#[command(... about)]`は英語のデフォルト値として残しつつ、日本語選択時は
/// この定数で上書きする）。
const CLI_ABOUT_JA: &str = "Simutrans アドオンの .dat を静的検証・整形・連結解析するCLIツール";
const CLI_ABOUT_EN: &str =
    "Static validator, formatter, and coupling analyzer for Simutrans .dat files";

#[derive(Subcommand)]
enum Command {
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
fn apply_language_to_help(cmd: clap::Command, lang: Language) -> clap::Command {
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
struct LintArgs {
    /// 単一ファイル・ディレクトリ・globパターン（例 `refs/*.dat`, `refs/**/*.dat`）を指定できる。
    /// PowerShellはシェル側で`*`を展開しないため、ツール自身がglobを解釈する
    /// （`glob`クレート使用。ディレクトリを指定した場合は再帰的に`.dat`を収集する）。
    path: String,
    /// -v: info まで表示 / -vv: debug まで表示
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    verbose: u8,
    /// ルールのinclude/exclude設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する（存在しなければ全ルール有効）。
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(clap::Args)]
struct FmtArgs {
    /// 単一ファイル・ディレクトリ・globパターン（例 `refs/*.dat`, `refs/**/*.dat`）を指定できる。
    /// `lint`と同じ`collect_dat_paths`で解決する（ディレクトリは再帰的に`.dat`を収集）。
    /// 複数ファイルに解決された場合、`--write`（`-w`）を指定しないとエラー終了する
    /// （整形結果を複数ファイル分stdoutへ混在させて出すのは実用性が低いため）。
    path: String,
    /// 並び替えを無効化し、元の行順を保持する（configの`[fmt] reorder`設定を
    /// このプロセスの実行に限り強制的に上書きする）。デフォルトでは
    /// `[fmt] reorder`（未設定時true）に従って並び替える。優先順位:
    /// `--no-reorder`指定 > config設定。
    #[arg(long)]
    no_reorder: bool,
    /// フォーマット結果をファイルへ書き込む
    #[arg(short = 'w', long)]
    write: bool,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    config: Option<PathBuf>,
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
enum AnalyzeKind {
    /// `obj=vehicle`群の`constraint[prev]`/`constraint[next]`について、
    /// (1) makeobjが検証しないdangling参照の実在性、(2) 連結制約の充足可能性
    /// （有限な編成として絶対に成立しない車両が無いか）を検査する
    /// （旧`couplings`サブコマンド相当）。
    Coupling,
}

#[derive(clap::Args)]
struct AnalyzeArgs {
    dir: PathBuf,
    /// 解析種別。現状は`coupling`（`obj=vehicle`の連結制約解析）のみ対応
    /// （将来の解析種別追加に備えたclap ValueEnum。デフォルトは`coupling`）。
    #[arg(long, value_enum, default_value_t = AnalyzeKind::Coupling)]
    kind: AnalyzeKind,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    config: Option<PathBuf>,
}

/// 第9弾（項目2）: `dat_linter.toml`の`[rules] include/exclude`に書けるcode
/// （`Diagnostic.code`）の一覧が分かる手段が無かったため新設したサブコマンド。
/// `--source`で`lint`/`fmt`/`analyze`いずれかのcodeだけに絞り込める
/// （省略時は全て表示）。
#[derive(clap::Args)]
struct ListArgs {
    /// 表示するcodeの由来を絞り込む（省略時は全て表示）。
    #[arg(long, value_enum)]
    source: Option<ListSourceArg>,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    config: Option<PathBuf>,
}

/// `--source`で選べる値。`codes::CodeSource`と1対1で対応するが、こちらは
/// clapの`ValueEnum`用にCLI引数値（`lint`/`fmt`/`analyze`という文字列）を
/// 持つ別の型として定義する（`codes::CodeSource`をライブラリ側に閉じ、
/// clap依存をmain.rs側だけに留めるため）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ListSourceArg {
    Lint,
    Fmt,
    Analyze,
}

impl ListSourceArg {
    /// **ワイルドカードarmを持たない網羅match**（このプロジェクトの規約）。
    fn to_code_source(self) -> dat_linter::codes::CodeSource {
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
struct DescribeArgs {
    /// 説明を表示するcode（例 obsolete-type）。一覧は`dat_linter list`で確認できる。
    code: String,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    config: Option<PathBuf>,
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
fn peek_config_arg() -> Option<PathBuf> {
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

fn main() -> ExitCode {
    // 1. 設定ファイル（言語含む）を、clapによる本来の引数解釈より先に読み込む。
    //    `--help`/`-h`はclapが`get_matches()`内で検出し即座に終了するため、
    //    ヘルプ表示にも翻訳後の言語を反映するにはこの順序が必須。
    //    設定ファイル自動生成（初回起動時）もこのタイミングで発生する。
    let explicit_config_path = peek_config_arg();
    let config = match LintConfig::load_or_default(explicit_config_path.as_deref()) {
        Ok(c) => c,
        Err(_) => {
            // 設定読み込み失敗時もヘルプ表示自体は継続できるよう、ここでは
            // デフォルト設定にフォールバックする。実際のエラー報告は
            // 各run_*関数内で`LintConfig::load_or_default`を再度呼んだ際に行う
            // （`--config`に不正なパスを渡した場合の従来のエラー経路を維持するため）。
            LintConfig::all_enabled()
        }
    };
    let language = config.language();

    // 2. `Cli::command()`（`CommandFactory`経由）で得たclap::Commandの短いabout
    //    文言を言語に応じて上書きしてから`get_matches()`する。これにより
    //    `--help`/`-h`表示にも翻訳が反映される。
    let cmd = apply_language_to_help(Cli::command(), language);
    let matches = cmd.get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    match cli.command {
        Command::Lint(args) => run_lint(&args, language),
        Command::Fmt(args) => run_fmt(&args, language),
        Command::Analyze(args) => run_analyze(&args, language),
        Command::List(args) => run_list(&args, language),
        Command::Describe(args) => run_describe(&args, language),
    }
}

/// 未対応obj種別のエラーメッセージ末尾に付ける対応obj一覧
/// （`obj=building / obj=vehicle / ...`）。obj種別名自体（`building`等）は
/// 翻訳対象外（`.dat`に実際に書く値そのものであり、変えるとユーザーが混乱する）。
fn supported_obj_list() -> String {
    SUPPORTED_OBJ_TYPES
        .iter()
        .map(|t| format!("obj={t}"))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// `LintArgs::path`（1ファイル・ディレクトリ・globパターンいずれか）を実在する
/// `.dat`ファイルパスの一覧へ解決する。
///
/// - globのメタ文字（`*`/`?`/`[`）を含む場合は`glob`クレートで展開する。
///   PowerShellはUnixシェルと異なり`*`をシェル側で自動展開しないため、
///   ツール自身がこの展開を担う必要がある。
/// - 実在するディレクトリの場合は再帰的に`.dat`ファイルを収集する。
/// - それ以外は単一ファイルパスとしてそのまま返す（存在しない場合も含め、
///   そのまま返してファイル読み込み時のエラーに委ねる。従来の単一ファイル
///   挙動と互換）。
///
/// 戻り値は入力順に依らず安定した順序（パス文字列の辞書順）に揃える。
fn collect_dat_paths(input: &str, lang: Language) -> Result<Vec<PathBuf>, String> {
    let has_glob_meta = input.contains(['*', '?', '[']);

    if has_glob_meta {
        let mut paths = BTreeSet::new();
        let entries = glob::glob(input).map_err(|e| {
            t!(lang,
                ja: "不正なglobパターンです ({e})",
                en: "Invalid glob pattern ({e})",
                e = e,
            )
        })?;
        for entry in entries {
            match entry {
                Ok(p) => {
                    if p.is_dir() {
                        collect_dat_files_recursive(&p, &mut paths);
                    } else if is_dat_file(&p) {
                        paths.insert(p);
                    }
                }
                Err(e) => {
                    return Err(t!(lang,
                        ja: "globの展開に失敗しました ({e})",
                        en: "Failed to expand glob pattern ({e})",
                        e = e,
                    ));
                }
            }
        }
        return Ok(paths.into_iter().collect());
    }

    let path = Path::new(input);
    if path.is_dir() {
        let mut paths = BTreeSet::new();
        collect_dat_files_recursive(path, &mut paths);
        return Ok(paths.into_iter().collect());
    }

    Ok(vec![path.to_path_buf()])
}

fn is_dat_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("dat"))
        .unwrap_or(false)
}

fn collect_dat_files_recursive(dir: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dat_files_recursive(&path, out);
        } else if is_dat_file(&path) {
            out.insert(path);
        }
    }
}

fn run_lint(args: &LintArgs, language: Language) -> ExitCode {
    let level = Severity::from_verbosity(args.verbose);

    let config = match LintConfig::load_or_default(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "設定ファイルの読み込みに失敗しました ({e})",
                    en: "Failed to load the configuration file ({e})",
                    e = e,
                )
            );
            return ExitCode::FAILURE;
        }
    };

    let paths = match collect_dat_paths(&args.path, language) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {e}", args.path);
            return ExitCode::FAILURE;
        }
    };

    if paths.is_empty() {
        eprintln!(
            "{}",
            t!(language,
                ja: "{path}: 該当する .dat ファイルが見つかりません",
                en: "{path}: No matching .dat files were found",
                path = args.path,
            )
        );
        return ExitCode::FAILURE;
    }

    // 単一ファイル指定時は従来通りの出力・終了コードのみ（サマリ行を追加しない）。
    if paths.len() == 1 {
        return lint_one_file(&paths[0], level, &config, language);
    }

    let mut total_error = 0usize;
    let mut total_warning = 0usize;
    let mut any_failure = false;

    for path in &paths {
        let (error_count, warning_count, failed) =
            lint_one_file_counts(path, level, &config, language);
        total_error += error_count;
        total_warning += warning_count;
        any_failure |= failed;
    }

    // 第10弾（項目1）: 指摘が一切無い（合計error/warningが共に0、かつ個々のファイルで
    // unsupported等の失敗も無い）場合は合計行も出力しない（サイレント成功）。
    if total_error > 0 || total_warning > 0 || any_failure {
        println!(
            "{}",
            t!(language,
                ja: "合計: 対象ファイル {n} 件 / error {total_error} 件 / warning {total_warning} 件",
                en: "Total: {n} file(s) / {total_error} error(s) / {total_warning} warning(s)",
                n = paths.len(),
                total_error = total_error,
                total_warning = total_warning,
            )
        );
    }

    if any_failure {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 1ファイルを検証し、`ExitCode`を返す（単一ファイル指定時の従来どおりの
/// 出力・終了コードそのもの）。
fn lint_one_file(
    path: &Path,
    level: Severity,
    config: &LintConfig,
    language: Language,
) -> ExitCode {
    let (_, _, failed) = lint_one_file_counts(path, level, config, language);
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 1ファイルの検証本体。`(error_count, warning_count, is_failure)`を返す。
/// 個々の診断行・サマリ行の出力は従来の単一ファイル出力フォーマットと同じ。
fn lint_one_file_counts(
    path: &Path,
    level: Severity,
    config: &LintConfig,
    language: Language,
) -> (usize, usize, bool) {
    let records = match DatFile::parse_all(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "{p}: 読み込みに失敗しました ({e})",
                    en: "{p}: Failed to read the file ({e})",
                    p = path.display(),
                    e = e,
                )
            );
            return (0, 0, true);
        }
    };

    // 1ファイルに`-`区切りで複数obj定義が連結されている実例（建物の複数ステージを
    // 1つの.datにまとめたもの等）がある。obj定義が無い（レコード0件）場合も
    // 単一obj前提だった従来の「obj=は未対応です」メッセージ・終了コードを再現する。
    if records.is_empty() {
        eprintln!(
            "{}",
            t!(language,
                ja: "{p}: obj= は未対応です。{list} のみ検証できます",
                en: "{p}: obj= is not supported. Only {list} can be validated",
                p = path.display(),
                list = supported_obj_list(),
            )
        );
        return (0, 0, true);
    }

    let dat_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let total = records.len();
    let mut diags = Vec::new();
    let mut unsupported = 0usize;

    for (idx, dat) in records.iter().enumerate() {
        let obj_type = dat.get("obj").unwrap_or("").to_string();
        let label = if total > 1 {
            let name = dat.get("name").unwrap_or("");
            if name.is_empty() {
                format!(" [{}/{total}]", idx + 1)
            } else {
                format!(" [{}/{total} name={name}]", idx + 1)
            }
        } else {
            String::new()
        };

        let Some(rule_set) = RuleSet::for_obj_type(&obj_type, dat) else {
            eprintln!(
                "{}",
                t!(language,
                    ja: "{p}{label}: obj={obj_type} は未対応です。{list} のみ検証できます",
                    en: "{p}{label}: obj={obj_type} is not supported. Only {list} can be validated",
                    p = path.display(),
                    label = label,
                    obj_type = obj_type,
                    list = supported_obj_list(),
                )
            );
            unsupported += 1;
            continue;
        };

        let ctx = RuleContext {
            dat,
            dat_dir,
            language,
        };
        let mut record_diags = rules::check_duplicate_keys(dat, language);
        record_diags.extend(rule_set.run(&ctx));
        record_diags.retain(|d| config.is_enabled(d.code));

        for d in record_diags.iter().filter(|d| d.severity <= level) {
            eprintln!("{}{label}: {d}", path.display());
        }
        diags.extend(record_diags);
    }

    // 単一obj・未対応の場合は従来通り即失敗（サマリ行を出さない挙動を維持）。
    if total == 1 && unsupported == 1 {
        return (0, 0, true);
    }

    let error_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();

    // 第10弾（項目1）: 指摘が1件も無い（error=0 && warning=0 && unsupported=0）場合、
    // stdoutへは一切出力しない（サイレント成功。CI/スクリプトでの利用を想定した
    // Unix系リンタの一般的な流儀）。従来はここで"OK"行を出力していたが、
    // 「指摘無し = 無音」を採用したため、この分岐からは何も出力しない。
    if error_count == 0 && warning_count == 0 && unsupported == 0 {
        // 何も出力しない（silent success）。
    } else if unsupported > 0 {
        println!(
            "{}",
            t!(language,
                ja: "{p}: error {error_count} 件 / warning {warning_count} 件 / 未対応 {unsupported} 件",
                en: "{p}: {error_count} error(s) / {warning_count} warning(s) / {unsupported} unsupported",
                p = path.display(),
                error_count = error_count,
                warning_count = warning_count,
                unsupported = unsupported,
            )
        );
    } else {
        println!(
            "{}",
            t!(language,
                ja: "{p}: error {error_count} 件 / warning {warning_count} 件",
                en: "{p}: {error_count} error(s) / {warning_count} warning(s)",
                p = path.display(),
                error_count = error_count,
                warning_count = warning_count,
            )
        );
    }

    // 第10弾（項目3）: warning以上（error or warning）が1件でもあれば異常終了扱いにする
    // （従来はerror_count>0またはunsupported>0のみが失敗条件で、warningのみの場合は
    // exit 0のままだった）。
    let failed = error_count > 0 || warning_count > 0 || unsupported > 0;
    (error_count, warning_count, failed)
}

fn run_fmt(args: &FmtArgs, language: Language) -> ExitCode {
    let config = match LintConfig::load_or_default(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "設定ファイルの読み込みに失敗しました ({e})",
                    en: "Failed to load the configuration file ({e})",
                    e = e,
                )
            );
            return ExitCode::FAILURE;
        }
    };
    // 優先順位: --no-reorder指定 > config設定（[fmt] reorder、未指定時デフォルトtrue）。
    let should_reorder = !args.no_reorder && config.fmt_reorder();

    let paths = match collect_dat_paths(&args.path, language) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {e}", args.path);
            return ExitCode::FAILURE;
        }
    };

    if paths.is_empty() {
        eprintln!(
            "{}",
            t!(language,
                ja: "{path}: 該当する .dat ファイルが見つかりません",
                en: "{path}: No matching .dat files were found",
                path = args.path,
            )
        );
        return ExitCode::FAILURE;
    }

    // 単一ファイル指定時は従来通りの出力・終了コードのみ（サマリ行を追加しない）。
    if paths.len() == 1 {
        return fmt_one_file(&paths[0], should_reorder, args.write, &config, language).0;
    }

    // 複数ファイルに解決された場合、`--write`が無いと整形結果をどのstdoutへ
    // 出すべきか一意に決まらない（複数ファイル分の内容が混在してしまう）ため、
    // ユーザー確認済みの仕様としてエラー終了する。
    if !args.write {
        eprintln!(
            "{}",
            t!(language,
                ja: "{path}: 複数ファイル（{n}件）に一致しましたが --write が指定されていません。\
                     複数ファイルを整形する場合は -w/--write を指定してください",
                en: "{path}: Matched {n} files, but --write was not specified. \
                     Pass -w/--write to format multiple files",
                path = args.path,
                n = paths.len(),
            )
        );
        return ExitCode::FAILURE;
    }

    let mut total_warnings = 0usize;
    let mut any_failure = false;
    for path in &paths {
        let (result, warning_count) =
            fmt_one_file(path, should_reorder, args.write, &config, language);
        total_warnings += warning_count;
        any_failure |= result == ExitCode::FAILURE;
    }

    // 第10弾（項目1）: warningが無ければ合計行も出力しない（サイレント成功）。
    // ただしwrite失敗（any_failureがtrueだがtotal_warnings==0）のケースは
    // 個々のファイルのエラーメッセージが既にstderrに出ているため、ここでの
    // 合計行は「warningの集計」目的のみと割り切り、warning自体が無ければ省略する。
    if total_warnings > 0 {
        println!(
            "{}",
            t!(language,
                ja: "合計: 対象ファイル {n} 件 / warning {total_warnings} 件",
                en: "Total: {n} file(s) / {total_warnings} warning(s)",
                n = paths.len(),
                total_warnings = total_warnings,
            )
        );
    }

    if any_failure {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 1ファイルを整形する。`(ExitCode, warning_count)`を返す
/// （`warning_count`は複数ファイル時の集計サマリに使う）。
///
/// 第9弾（項目3）: `fmt`が出すwarning（`parse_entries`/`format_reordered`由来）にも
/// `[rules] include/exclude`（`config.is_enabled`）を`lint`と全く同じ意味論で適用する。
/// フィルタ後の件数を`warning_count`として返すため、複数ファイル時の集計サマリも
/// 除外されたwarningを含まない。
fn fmt_one_file(
    path: &Path,
    should_reorder: bool,
    write: bool,
    config: &LintConfig,
    language: Language,
) -> (ExitCode, usize) {
    let text = match read_dat_text(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "{p}: 読み込みに失敗しました ({e})",
                    en: "{p}: Failed to read the file ({e})",
                    p = path.display(),
                    e = e,
                )
            );
            return (ExitCode::FAILURE, 0);
        }
    };

    let parsed = formatter::parse_entries(&text, language);
    let filtered_parse_warnings: Vec<_> = parsed
        .warnings
        .iter()
        .filter(|w| config.is_enabled(w.code))
        .collect();
    let mut warning_count = filtered_parse_warnings.len();
    for w in &filtered_parse_warnings {
        eprintln!("{}: {w}", path.display());
    }

    let formatted = if should_reorder {
        let obj = formatter::obj_of(&parsed.entries).unwrap_or("").to_string();
        let (out, warnings) = formatter::format_reordered(&parsed.entries, &obj, language);
        let filtered_warnings: Vec<_> = warnings
            .iter()
            .filter(|w| config.is_enabled(w.code))
            .collect();
        warning_count += filtered_warnings.len();
        for w in &filtered_warnings {
            eprintln!("{}: {w}", path.display());
        }
        out
    } else {
        formatter::format_preserve_order(&parsed.entries)
    };

    if write {
        if let Err(e) = std::fs::write(path, &formatted) {
            eprintln!(
                "{}",
                t!(language,
                    ja: "{p}: 書き込みに失敗しました ({e})",
                    en: "{p}: Failed to write the file ({e})",
                    p = path.display(),
                    e = e,
                )
            );
            return (ExitCode::FAILURE, warning_count);
        }
        // 第10弾（項目4）: 書き込み成功メッセージは診断ではなく純粋な情報メッセージ
        // なので、他の情報メッセージ（OK行・件数サマリ行等）と同じくstdoutに統一する
        // （従来はeprintln!でstderrに出ていた）。
        println!(
            "{}",
            t!(language,
                ja: "{p}: フォーマット結果を書き込みました",
                en: "{p}: Formatted output written",
                p = path.display(),
            )
        );
    } else {
        print!("{formatted}");
    }

    // 第10弾（項目3）: warningが1件でもあれば異常終了扱いにする
    // （従来はwrite失敗以外は常にSUCCESSを返していた）。
    let result = if warning_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    };
    (result, warning_count)
}

/// `analyze`サブコマンドの入口。`args.kind`に応じた解析関数へディスパッチする。
/// `AnalyzeKind`に対する**ワイルドカードarmを持たない網羅match**であることが
/// このリファクタの要点で、将来`AnalyzeKind`に新しいバリアントを追加してこの
/// matchへのarm追加を忘れると`cargo build`が失敗する（`registry::RuleSet::for_obj_type`
/// と同じ設計思想）。
///
/// 第9弾（項目3）: `lint`/`fmt`と同じく`args.config`から`LintConfig`を読み込み、
/// `run_analyze_coupling`へ渡して`couplings.rs`が出す`Diagnostic`にも
/// include/exclude（`config.is_enabled`）を適用する。
fn run_analyze(args: &AnalyzeArgs, language: Language) -> ExitCode {
    let config = match LintConfig::load_or_default(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "設定ファイルの読み込みに失敗しました ({e})",
                    en: "Failed to load the configuration file ({e})",
                    e = e,
                )
            );
            return ExitCode::FAILURE;
        }
    };
    match args.kind {
        AnalyzeKind::Coupling => run_analyze_coupling(&args.dir, &config, language),
    }
}

/// 静的解析(PHPStan的な層)のPoC: 1ディレクトリ内の vehicle dat 群を読み込み、
/// (1) makeobjが検証しないconstraint参照の実在性、(2) 連結制約の充足可能性
/// （有限な編成として絶対に成立しない車両が無いか）を検査する。
/// （旧`couplings`サブコマンド相当。`couplings.rs`モジュール自体の関数は変更していない）
fn run_analyze_coupling(dir: &Path, config: &LintConfig, language: Language) -> ExitCode {
    let (vehicles, mut diags) = couplings::load_vehicles(dir, language);
    diags.extend(couplings::check_dangling_refs(&vehicles, language));
    diags.extend(couplings::check_satisfiability(&vehicles, language));
    diags.retain(|d| config.is_enabled(d.code));

    // 第10弾（項目4）: 情報メッセージ（読み込み件数）はstdout、診断本文はstderrに分離する。
    println!(
        "{}",
        t!(language,
            ja: "{d}: {n} 台の vehicle dat を読み込みました",
            en: "{d}: Loaded {n} vehicle dat file(s)",
            d = dir.display(),
            n = vehicles.len(),
        )
    );
    for d in &diags {
        eprintln!("{}: {d}", dir.display());
    }

    let error_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();

    // 第10弾（項目1）: 指摘が無ければ"OK"行も出力しない（サイレント成功）。
    // couplings.rsは現状Diagnostic::errorのみ発行しwarningは無いが、将来warningが
    // 追加された場合にも備えてerror_count/warning_countの両方を判定に含める
    // （一貫性のため、`lint`/`fmt`と同じ扱い）。
    if error_count == 0 && warning_count == 0 {
        // 何も出力しない（silent success）。
    } else {
        println!(
            "{}",
            t!(language,
                ja: "{d}: error {error_count} 件 / warning {warning_count} 件",
                en: "{d}: {error_count} error(s) / {warning_count} warning(s)",
                d = dir.display(),
                error_count = error_count,
                warning_count = warning_count,
            )
        );
    }

    // 第10弾（項目3）: warning以上（error or warning）が1件でもあれば異常終了扱いにする
    // （従来はerror_count>0のみが失敗条件だった。couplings.rsは現状warningを発行しないため
    // 実害は無いが、lint/fmtとの一貫性のため一般化しておく）。
    if error_count > 0 || warning_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 第9弾（項目2）: `dat_linter.toml`の`[rules] include/exclude`に書けるcode
/// （`Diagnostic.code`）の一覧を表示する。一覧自体は`codes::ALL_CODES`
/// （実ソースとの整合性は`tests/codes_completeness.rs`が保証）から取得する。
/// `--config`が指定された場合、各codeが現在の設定で有効か無効かも併記する
/// （設定ファイルを編集する前に効果を確認できるようにするため）。
fn run_list(args: &ListArgs, language: Language) -> ExitCode {
    let config = match LintConfig::load_or_default(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "設定ファイルの読み込みに失敗しました ({e})",
                    en: "Failed to load the configuration file ({e})",
                    e = e,
                )
            );
            return ExitCode::FAILURE;
        }
    };

    let wanted_source = args.source.map(ListSourceArg::to_code_source);
    let mut shown = 0usize;
    for info in dat_linter::codes::ALL_CODES {
        if let Some(w) = wanted_source
            && w != info.source
        {
            continue;
        }
        shown += 1;
        let enabled = config.is_enabled(info.code);
        let status = match (enabled, language) {
            (true, Language::Japanese) => "有効",
            (true, Language::English) => "enabled",
            (false, Language::Japanese) => "無効",
            (false, Language::English) => "disabled",
        };
        println!("{:<12} {:<45} {status}", info.source.as_str(), info.code);
    }

    println!(
        "{}",
        t!(language,
            ja: "合計 {shown} 件のcode",
            en: "Total: {shown} code(s)",
            shown = shown,
        )
    );

    ExitCode::SUCCESS
}

/// 第10弾（項目6）: 指定したcodeの説明（なぜNGか・どう直すか）を表示する。
/// `codes::ALL_CODES`（`list`と同じ一覧、`tests/codes_completeness.rs`が実ソースとの
/// 整合性を保証）からcodeを検索し、見つかれば`why`/`how_to_fix`をJA/ENに応じて表示する。
/// 見つからない場合は`list`コマンドの案内を添えてexit failureにする。
fn run_describe(args: &DescribeArgs, language: Language) -> ExitCode {
    let config = match LintConfig::load_or_default(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "設定ファイルの読み込みに失敗しました ({e})",
                    en: "Failed to load the configuration file ({e})",
                    e = e,
                )
            );
            return ExitCode::FAILURE;
        }
    };

    let Some(info) = dat_linter::codes::ALL_CODES
        .iter()
        .find(|info| info.code == args.code)
    else {
        eprintln!(
            "{}",
            t!(language,
                ja: "{code}: 不明なcodeです。`dat_linter list` で有効なcode一覧を確認してください",
                en: "{code}: Unknown code. Run `dat_linter list` to see the list of valid codes",
                code = args.code,
            )
        );
        return ExitCode::FAILURE;
    };

    let enabled = config.is_enabled(info.code);
    let status = match (enabled, language) {
        (true, Language::Japanese) => "有効",
        (true, Language::English) => "enabled",
        (false, Language::Japanese) => "無効",
        (false, Language::English) => "disabled",
    };

    println!("{:<12} {} ({status})", info.source.as_str(), info.code);
    println!();
    println!(
        "{}",
        t!(language,
            ja: "なぜNGか:",
            en: "Why:",
        )
    );
    println!("{}", info.why(language));
    println!();
    println!(
        "{}",
        t!(language,
            ja: "どう直すか:",
            en: "How to fix:",
        )
    );
    println!("{}", info.how_to_fix(language));

    ExitCode::SUCCESS
}
