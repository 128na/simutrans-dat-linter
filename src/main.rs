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
    /// 1ディレクトリ内の obj=vehicle 群を連結制約について解析する
    Couplings(CouplingsArgs),
}

/// `lint`の長い説明（22obj種別の一覧を含む）。翻訳対象外、常に日本語のまま
/// （コーディネーター指示: 短いabout一行のみ翻訳し、この長文はJP固定でよい）。
const LINT_LONG_ABOUT_JA: &str = ".dat ファイル1件を静的検証する（obj=building / obj=vehicle / obj=way / obj=good / obj=bridge / obj=tunnel / obj=roadsign / obj=crossing / obj=way-object / obj=ground_obj / obj=tree / obj=citycar / obj=pedestrian / obj=factory / obj=sound / obj=ground / obj=menu / obj=cursor / obj=symbol / obj=smoke / obj=field / obj=misc）";

/// 各subcommandの短い`about`（JA/EN）。`apply_language_to_help`から参照する。
const LINT_ABOUT_JA: &str = ".dat ファイル1件を静的検証する";
const LINT_ABOUT_EN: &str = "Statically validate a single .dat file";
const FMT_ABOUT_JA: &str = ".dat ファイルを正規化・並び替えする";
const FMT_ABOUT_EN: &str = "Normalize and reorder a .dat file";
const COUPLINGS_ABOUT_JA: &str = "1ディレクトリ内の obj=vehicle 群を連結制約について解析する";
const COUPLINGS_ABOUT_EN: &str =
    "Analyze coupling constraints across obj=vehicle definitions in a directory";

/// `Cli::command()`が返す`clap::Command`の短い`about`を言語に応じて上書きする。
/// `long_about`（22obj種別一覧を含む長文）には触れない（翻訳対象外のため常に日本語）。
///
/// clapの`derive(Parser)`が生成する`Cli::parse()`は内部で`--help`/`-h`検出時に
/// 即座に`Command::print_help()`し`process::exit()`するため、`Cli::parse()`
/// そのものを呼ぶと事前に言語を反映する機会が無い。この関数を`Cli::command()`の
/// 戻り値に適用してから`.get_matches()`することで、`--help`表示にも
/// 翻訳後の文言を反映できる（`main()`参照）。
fn apply_language_to_help(cmd: clap::Command, lang: Language) -> clap::Command {
    let top_about = match lang {
        Language::Japanese => CLI_ABOUT_JA,
        Language::English => CLI_ABOUT_EN,
    };
    cmd.about(top_about)
        .mut_subcommand("lint", |c| {
            let about = match lang {
                Language::Japanese => LINT_ABOUT_JA,
                Language::English => LINT_ABOUT_EN,
            };
            c.about(about)
        })
        .mut_subcommand("fmt", |c| {
            let about = match lang {
                Language::Japanese => FMT_ABOUT_JA,
                Language::English => FMT_ABOUT_EN,
            };
            c.about(about)
        })
        .mut_subcommand("couplings", |c| {
            let about = match lang {
                Language::Japanese => COUPLINGS_ABOUT_JA,
                Language::English => COUPLINGS_ABOUT_EN,
            };
            c.about(about)
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
    path: PathBuf,
    /// 慣習的な順序に並び替える（デフォルトは元の行順を保持）
    #[arg(long)]
    reorder: bool,
    /// フォーマット結果をファイルへ書き込む
    #[arg(short = 'w', long)]
    write: bool,
    /// 出力言語等の設定ファイル（TOML）。省略時はカレントディレクトリの
    /// `dat_linter.toml`を自動探索する。
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(clap::Args)]
struct CouplingsArgs {
    dir: PathBuf,
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
        Command::Couplings(args) => run_couplings(&args, language),
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
            println!("{}{label}: {d}", path.display());
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

    if error_count == 0 && warning_count == 0 && unsupported == 0 {
        println!(
            "{}",
            t!(language,
                ja: "{p}: OK（既知ルールの範囲では問題なし）",
                en: "{p}: OK (no issues found within known rules)",
                p = path.display(),
            )
        );
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

    let failed = error_count > 0 || unsupported > 0;
    (error_count, warning_count, failed)
}

fn run_fmt(args: &FmtArgs, language: Language) -> ExitCode {
    let path = args.path.as_path();

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
            return ExitCode::FAILURE;
        }
    };

    let parsed = formatter::parse_entries(&text, language);
    for w in &parsed.warnings {
        eprintln!("{}: {w}", path.display());
    }

    let formatted = if args.reorder {
        let obj = formatter::obj_of(&parsed.entries).unwrap_or("").to_string();
        let (out, warnings) = formatter::format_reordered(&parsed.entries, &obj, language);
        for w in &warnings {
            eprintln!("{}: {w}", path.display());
        }
        out
    } else {
        formatter::format_preserve_order(&parsed.entries)
    };

    if args.write {
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
            return ExitCode::FAILURE;
        }
        eprintln!(
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

    ExitCode::SUCCESS
}

/// 静的解析(PHPStan的な層)のPoC: 1ディレクトリ内の vehicle dat 群を読み込み、
/// (1) makeobjが検証しないconstraint参照の実在性、(2) 連結制約の充足可能性
/// （有限な編成として絶対に成立しない車両が無いか）を検査する。
fn run_couplings(args: &CouplingsArgs, language: Language) -> ExitCode {
    let dir = args.dir.as_path();

    let (vehicles, mut diags) = couplings::load_vehicles(dir, language);
    diags.extend(couplings::check_dangling_refs(&vehicles, language));
    diags.extend(couplings::check_satisfiability(&vehicles, language));

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
        println!("{}: {d}", dir.display());
    }

    let error_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    if error_count == 0 {
        println!(
            "{}",
            t!(language,
                ja: "{d}: OK（既知ルールの範囲では問題なし）",
                en: "{d}: OK (no issues found within known rules)",
                d = dir.display(),
            )
        );
    } else {
        println!(
            "{}",
            t!(language,
                ja: "{d}: error {error_count} 件",
                en: "{d}: {error_count} error(s)",
                d = dir.display(),
                error_count = error_count,
            )
        );
    }

    if error_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
