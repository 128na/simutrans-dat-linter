use clap::{ArgAction, Parser, Subcommand};
use dat_linter::diagnostics::Severity;
use dat_linter::parser::{read_dat_text, DatFile};
use dat_linter::registry::{RuleContext, RuleSet, SUPPORTED_OBJ_TYPES};
use dat_linter::{couplings, formatter, rules};
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
    path: PathBuf,
    /// -v: info まで表示 / -vv: debug まで表示
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    verbose: u8,
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

fn run_lint(args: &LintArgs) -> ExitCode {
    let level = Severity::from_verbosity(args.verbose);
    let path = args.path.as_path();

    let records = match DatFile::parse_all(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: 読み込みに失敗しました ({e})", path.display());
            return ExitCode::FAILURE;
        }
    };

    let supported = || {
        SUPPORTED_OBJ_TYPES
            .iter()
            .map(|t| format!("obj={t}"))
            .collect::<Vec<_>>()
            .join(" / ")
    };

    // 1ファイルに`-`区切りで複数obj定義が連結されている実例（建物の複数ステージを
    // 1つの.datにまとめたもの等）がある。obj定義が無い（レコード0件）場合も
    // 単一obj前提だった従来の「obj=は未対応です」メッセージ・終了コードを再現する。
    if records.is_empty() {
        eprintln!(
            "{}: obj= は未対応です。{} のみ検証できます",
            path.display(),
            supported()
        );
        return ExitCode::FAILURE;
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
                supported()
            );
            unsupported += 1;
            continue;
        };

        let ctx = RuleContext { dat, dat_dir };
        let mut record_diags = rules::check_duplicate_keys(dat);
        record_diags.extend(rule_set.run(&ctx));

        for d in record_diags.iter().filter(|d| d.severity <= level) {
            println!("{}{label}: {d}", path.display());
        }
        diags.extend(record_diags);
    }

    // 単一obj・未対応の場合は従来通り即失敗（サマリ行を出さない挙動を維持）。
    if total == 1 && unsupported == 1 {
        return ExitCode::FAILURE;
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

    if error_count > 0 || unsupported > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
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
