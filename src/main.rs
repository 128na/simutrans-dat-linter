use clap::{ArgAction, Parser, Subcommand};
use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::registry::{RuleContext, RuleSet};
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
    /// .dat ファイル1件を静的検証する（obj=building / obj=vehicle / obj=way / obj=good / obj=bridge / obj=tunnel / obj=roadsign / obj=crossing / obj=way-object / obj=ground_obj / obj=tree / obj=citycar / obj=pedestrian / obj=factory / obj=sound / obj=ground）
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

    let dat = match DatFile::parse(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: 読み込みに失敗しました ({e})", path.display());
            return ExitCode::FAILURE;
        }
    };

    let obj_type = dat.get("obj").unwrap_or("").to_string();
    let dat_dir = path.parent().unwrap_or_else(|| Path::new("."));

    let Some(rule_set) = RuleSet::for_obj_type(&obj_type, &dat) else {
        eprintln!(
            "{}: obj={obj_type} は未対応です。obj=building / obj=vehicle / obj=way / obj=good / obj=bridge / obj=tunnel / obj=roadsign / obj=crossing / obj=way-object / obj=ground_obj / obj=tree / obj=citycar / obj=pedestrian / obj=factory / obj=sound / obj=ground のみ検証できます",
            path.display()
        );
        return ExitCode::FAILURE;
    };

    let ctx = RuleContext { dat: &dat, dat_dir };
    let mut diags = rules::check_duplicate_keys(&dat);
    diags.extend(rule_set.run(&ctx));

    for d in diags.iter().filter(|d| d.severity <= level) {
        println!("{}: {d}", path.display());
    }

    let error_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();

    if error_count == 0 && warning_count == 0 {
        println!("{}: OK（既知ルールの範囲では問題なし）", path.display());
    } else {
        println!(
            "{}: error {error_count} 件 / warning {warning_count} 件",
            path.display()
        );
    }

    if error_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_fmt(args: &FmtArgs) -> ExitCode {
    let path = args.path.as_path();

    let text = match std::fs::read_to_string(path) {
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
