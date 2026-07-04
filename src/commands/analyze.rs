//! `analyze`サブコマンドの実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で切り出した。振る舞いは分割前と完全に同一。

use crate::cli::{AnalyzeArgs, AnalyzeKind};
use dat_linter::config::LintConfig;
use dat_linter::couplings;
use dat_linter::diagnostics::Severity;
use dat_linter::i18n::{Language, t};
use std::path::Path;
use std::process::ExitCode;

/// `analyze`サブコマンドの入口。`args.kind`に応じた解析関数へディスパッチする。
/// `AnalyzeKind`に対する**ワイルドカードarmを持たない網羅match**であることが
/// このリファクタの要点で、将来`AnalyzeKind`に新しいバリアントを追加してこの
/// matchへのarm追加を忘れると`cargo build`が失敗する（`registry::RuleSet::for_obj_type`
/// と同じ設計思想）。
///
/// 第9弾（項目3）: `lint`/`fmt`と同じく`args.config`から`LintConfig`を読み込み、
/// `run_analyze_coupling`へ渡して`couplings.rs`が出す`Diagnostic`にも
/// include/exclude（`config.is_enabled`）を適用する。
pub fn run_analyze(args: &AnalyzeArgs, language: Language) -> ExitCode {
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
