//! `lint`サブコマンドの実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で切り出した。振る舞いは分割前と完全に同一。

use crate::cli::LintArgs;
use crate::commands::common::{aggregate_multi_file, exit_code_for, resolve_paths_or_exit};
use crate::fs_walk::supported_obj_list;
use dat_linter::config::LintConfig;
use dat_linter::diagnostics::Severity;
use dat_linter::i18n::{Language, t};
use dat_linter::parser::DatFile;
use dat_linter::registry::{RuleContext, RuleSet};
use dat_linter::rules;
use std::path::Path;
use std::process::ExitCode;

pub fn run_lint(args: &LintArgs, language: Language) -> ExitCode {
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

    let paths = match resolve_paths_or_exit(&args.path, language) {
        Ok(p) => p,
        Err(code) => return code,
    };

    // 単一ファイル指定時は従来通りの出力・終了コードのみ（サマリ行を追加しない）。
    if paths.len() == 1 {
        return lint_one_file(&paths[0], level, &config, language);
    }

    let counts = aggregate_multi_file(&paths, |path| {
        lint_one_file_counts(path, level, &config, language)
    });

    // 第10弾（項目1）: 指摘が一切無い（合計error/warningが共に0、かつ個々のファイルで
    // unsupported等の失敗も無い）場合は合計行も出力しない（サイレント成功）。
    if counts.error_count > 0 || counts.warning_count > 0 || counts.any_failure {
        println!(
            "{}",
            t!(language,
                ja: "合計: 対象ファイル {n} 件 / error {total_error} 件 / warning {total_warning} 件",
                en: "Total: {n} file(s) / {total_error} error(s) / {total_warning} warning(s)",
                n = paths.len(),
                total_error = counts.error_count,
                total_warning = counts.warning_count,
            )
        );
    }

    exit_code_for(counts.any_failure)
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
