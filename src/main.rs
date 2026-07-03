use clap::{ArgAction, Parser, Subcommand};
use dat_linter::config::LintConfig;
use dat_linter::diagnostics::Severity;
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

#[derive(Subcommand)]
enum Command {
    // Note: clapのderiveマクロはdocコメントをコンパイル時の静的文字列としてしか
    // 扱えないため、このヘルプ文言のobj種別一覧は`registry::SUPPORTED_OBJ_TYPES`から
    // 動的に構築できない。obj種別を追加・変更する際は、必ず
    // `registry::SUPPORTED_OBJ_TYPES`（正）と手動で同期させること
    // （実行時のエラーメッセージは`SUPPORTED_OBJ_TYPES`から動的に構築しており、
    // ズレは`tests/obj_type_coverage.rs`で検出できる）。
    /// .dat ファイル1件を静的検証する（obj=building / obj=vehicle / obj=way / obj=good / obj=bridge / obj=tunnel / obj=roadsign / obj=crossing / obj=way-object / obj=ground_obj / obj=tree / obj=citycar / obj=pedestrian / obj=factory / obj=sound / obj=ground / obj=menu / obj=cursor / obj=symbol / obj=smoke / obj=field / obj=misc）
    Lint(LintArgs),
    /// .dat ファイルを正規化・並び替えする
    Fmt(FmtArgs),
    /// 1ディレクトリ内の obj=vehicle 群を連結制約について解析する
    Couplings(CouplingsArgs),
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
}

#[derive(clap::Args)]
struct CouplingsArgs {
    dir: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Lint(args) => run_lint(&args),
        Command::Fmt(args) => run_fmt(&args),
        Command::Couplings(args) => run_couplings(&args),
    }
}

/// 未対応obj種別のエラーメッセージ末尾に付ける対応obj一覧
/// （`obj=building / obj=vehicle / ...`）。
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
fn collect_dat_paths(input: &str) -> Result<Vec<PathBuf>, String> {
    let has_glob_meta = input.contains(['*', '?', '[']);

    if has_glob_meta {
        let mut paths = BTreeSet::new();
        let entries = glob::glob(input).map_err(|e| format!("不正なglobパターンです ({e})"))?;
        for entry in entries {
            match entry {
                Ok(p) => {
                    if p.is_dir() {
                        collect_dat_files_recursive(&p, &mut paths);
                    } else if is_dat_file(&p) {
                        paths.insert(p);
                    }
                }
                Err(e) => return Err(format!("globの展開に失敗しました ({e})")),
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

fn run_lint(args: &LintArgs) -> ExitCode {
    let level = Severity::from_verbosity(args.verbose);

    let config = match LintConfig::load_or_default(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("設定ファイルの読み込みに失敗しました ({e})");
            return ExitCode::FAILURE;
        }
    };

    let paths = match collect_dat_paths(&args.path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {e}", args.path);
            return ExitCode::FAILURE;
        }
    };

    if paths.is_empty() {
        eprintln!("{}: 該当する .dat ファイルが見つかりません", args.path);
        return ExitCode::FAILURE;
    }

    // 単一ファイル指定時は従来通りの出力・終了コードのみ（サマリ行を追加しない）。
    if paths.len() == 1 {
        return lint_one_file(&paths[0], level, &config);
    }

    let mut total_error = 0usize;
    let mut total_warning = 0usize;
    let mut any_failure = false;

    for path in &paths {
        let (error_count, warning_count, failed) = lint_one_file_counts(path, level, &config);
        total_error += error_count;
        total_warning += warning_count;
        any_failure |= failed;
    }

    println!(
        "合計: 対象ファイル {} 件 / error {total_error} 件 / warning {total_warning} 件",
        paths.len()
    );

    if any_failure {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 1ファイルを検証し、`ExitCode`を返す（単一ファイル指定時の従来どおりの
/// 出力・終了コードそのもの）。
fn lint_one_file(path: &Path, level: Severity, config: &LintConfig) -> ExitCode {
    let (_, _, failed) = lint_one_file_counts(path, level, config);
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 1ファイルの検証本体。`(error_count, warning_count, is_failure)`を返す。
/// 個々の診断行・サマリ行の出力は従来の単一ファイル出力フォーマットと同じ。
fn lint_one_file_counts(path: &Path, level: Severity, config: &LintConfig) -> (usize, usize, bool) {
    let records = match DatFile::parse_all(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: 読み込みに失敗しました ({e})", path.display());
            return (0, 0, true);
        }
    };

    // 1ファイルに`-`区切りで複数obj定義が連結されている実例（建物の複数ステージを
    // 1つの.datにまとめたもの等）がある。obj定義が無い（レコード0件）場合も
    // 単一obj前提だった従来の「obj=は未対応です」メッセージ・終了コードを再現する。
    if records.is_empty() {
        eprintln!(
            "{}: obj= は未対応です。{} のみ検証できます",
            path.display(),
            supported_obj_list()
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
                "{}{label}: obj={obj_type} は未対応です。{} のみ検証できます",
                path.display(),
                supported_obj_list()
            );
            unsupported += 1;
            continue;
        };

        let ctx = RuleContext { dat, dat_dir };
        let mut record_diags = rules::check_duplicate_keys(dat);
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
        println!("{}: OK（既知ルールの範囲では問題なし）", path.display());
    } else if unsupported > 0 {
        println!(
            "{}: error {error_count} 件 / warning {warning_count} 件 / 未対応 {unsupported} 件",
            path.display()
        );
    } else {
        println!(
            "{}: error {error_count} 件 / warning {warning_count} 件",
            path.display()
        );
    }

    let failed = error_count > 0 || unsupported > 0;
    (error_count, warning_count, failed)
}

fn run_fmt(args: &FmtArgs) -> ExitCode {
    let path = args.path.as_path();

    let text = match read_dat_text(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: 読み込みに失敗しました ({e})", path.display());
            return ExitCode::FAILURE;
        }
    };

    let parsed = formatter::parse_entries(&text);
    for w in &parsed.warnings {
        eprintln!("{}: {w}", path.display());
    }

    let formatted = if args.reorder {
        let obj = formatter::obj_of(&parsed.entries).unwrap_or("").to_string();
        let (out, warnings) = formatter::format_reordered(&parsed.entries, &obj);
        for w in &warnings {
            eprintln!("{}: {w}", path.display());
        }
        out
    } else {
        formatter::format_preserve_order(&parsed.entries)
    };

    if args.write {
        if let Err(e) = std::fs::write(path, &formatted) {
            eprintln!("{}: 書き込みに失敗しました ({e})", path.display());
            return ExitCode::FAILURE;
        }
        eprintln!("{}: フォーマット結果を書き込みました", path.display());
    } else {
        print!("{formatted}");
    }

    ExitCode::SUCCESS
}

/// 静的解析(PHPStan的な層)のPoC: 1ディレクトリ内の vehicle dat 群を読み込み、
/// (1) makeobjが検証しないconstraint参照の実在性、(2) 連結制約の充足可能性
/// （有限な編成として絶対に成立しない車両が無いか）を検査する。
fn run_couplings(args: &CouplingsArgs) -> ExitCode {
    let dir = args.dir.as_path();

    let (vehicles, mut diags) = couplings::load_vehicles(dir);
    diags.extend(couplings::check_dangling_refs(&vehicles));
    diags.extend(couplings::check_satisfiability(&vehicles));

    println!(
        "{}: {} 台の vehicle dat を読み込みました",
        dir.display(),
        vehicles.len()
    );
    for d in &diags {
        println!("{}: {d}", dir.display());
    }

    let error_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    if error_count == 0 {
        println!("{}: OK（既知ルールの範囲では問題なし）", dir.display());
    } else {
        println!("{}: error {error_count} 件", dir.display());
    }

    if error_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
