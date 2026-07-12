//! `lint`サブコマンドの実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で切り出した。振る舞いは分割前と完全に同一。
//!
//! 第1弾（VSCode拡張のDiagnostics統合）: `--format json`を追加した。
//! `args.format == LintFormat::Text`の経路（`run_lint`後半・`lint_one_file`・
//! `lint_one_file_counts`）はこの変更の前後で完全に同一の`eprintln!`/`println!`
//! 出力・exit codeを維持する（既存テストがそのまま通ることを要求されているため、
//! 既存関数の内部にif/elseで分岐を混ぜ込まず、`args.format`による分岐は
//! `run_lint`の冒頭で行い、`LintFormat::Json`は完全に独立した経路
//! （`run_lint_json`/`lint_one_file_json`）へ委譲する設計にした）。
//! JSON経路はstdoutへ`serde_json::to_string`を1回だけ出力し、stderrには
//! 何も書かない。

use crate::cli::{LintArgs, LintFormat};
use crate::commands::common::{aggregate_multi_file, exit_code_for, resolve_paths_or_exit};
use crate::fs_walk::supported_obj_list;
use dat_linter::codes::DiagnosticCode;
use dat_linter::config::LintConfig;
use dat_linter::diagnostics::{Diagnostic, JsonDiagnostic, Severity};
use dat_linter::i18n::{Language, t};
use dat_linter::parser::DatFile;
use dat_linter::registry::{RuleContext, RuleSet};
use dat_linter::rules;
use std::path::{Path, PathBuf};
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

    let (paths, had_unreadable_dir) = match resolve_paths_or_exit(&args.path, language) {
        Ok(p) => p,
        Err(code) => return code,
    };

    if args.format == LintFormat::Json {
        let code = run_lint_json(&paths, level, &config, language, args.tile_size);
        return if had_unreadable_dir && code == ExitCode::SUCCESS {
            ExitCode::FAILURE
        } else {
            code
        };
    }

    // 単一ファイル指定時は従来通りの出力・終了コードのみ（サマリ行を追加しない）。
    // ただし、走査中に読み取れなかったサブディレクトリが1件でもあれば
    // （権限エラー等で一部を見ていない状態）、個々のファイル結果に関わらず
    // 失敗扱いにする（fs_walk.rs::collect_dat_files_recursiveのdocコメント参照）。
    if paths.len() == 1 {
        let code = lint_one_file(&paths[0], level, &config, language, args.tile_size);
        return if had_unreadable_dir && code == ExitCode::SUCCESS {
            ExitCode::FAILURE
        } else {
            code
        };
    }

    let counts = aggregate_multi_file(&paths, |path| {
        lint_one_file_counts(path, level, &config, language, args.tile_size)
    });

    // 第10弾（項目1）: 指摘が一切無い（合計error/warningが共に0、かつ個々のファイルで
    // unsupported等の失敗も無い）場合は合計行も出力しない（サイレント成功）。
    // ただし読み取れなかったサブディレクトリがあった場合は「サイレント成功」を
    // 名乗れないため、合計行自体は出さなくてもexit codeは失敗にする
    // （`exit_code_for`呼び出し側で`had_unreadable_dir`を畳み込む）。
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

    exit_code_for(counts.any_failure || had_unreadable_dir)
}

/// 1ファイルを検証し、`ExitCode`を返す（単一ファイル指定時の従来どおりの
/// 出力・終了コードそのもの）。
fn lint_one_file(
    path: &Path,
    level: Severity,
    config: &LintConfig,
    language: Language,
    tile_size_override: Option<u32>,
) -> ExitCode {
    let (_, _, failed) = lint_one_file_counts(path, level, config, language, tile_size_override);
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
    tile_size_override: Option<u32>,
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

        // タイルサイズの解決優先順位（高い方が勝つ。dat_linter::config::LintConfig
        // モジュール冒頭docコメント「tile_size」参照）:
        // 1. `--tile-size` CLI引数（1回限りの明示的な上書き）
        // 2. `.dat`自身の`cell_size=`フィールド（`obj_writer.cc:50`の
        //    `obj.get_int("cell_size", default_image_size)`に対応する実在フィールド）
        // 3. `dat_linter.toml`の`[tile_size]`（`LintConfig::tile_size_for`が
        //    overrides/defaultを解決する）
        let tile_size = tile_size_override
            .or_else(|| dat.get("cell_size").and_then(|s| s.trim().parse().ok()))
            .unwrap_or_else(|| config.tile_size_for(path));

        let ctx = RuleContext {
            dat,
            dat_dir,
            language,
            tile_size,
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

// --- `--format json`専用の経路 --------------------------------------------

/// `--format json`のトップレベル出力スキーマ。
/// `{ "files": [...], "summary": { "error_count": N, "warning_count": N } }`。
#[derive(serde::Serialize)]
struct JsonLintOutput {
    files: Vec<JsonFileReport>,
    summary: JsonLintSummary,
}

#[derive(serde::Serialize)]
struct JsonFileReport {
    path: String,
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(serde::Serialize)]
struct JsonLintSummary {
    error_count: usize,
    warning_count: usize,
}

/// `--format json`のトップレベル。単一ファイル・複数ファイルを区別せず、
/// 常に`files`配列（要素数1以上）+ `summary`の1つのJSONオブジェクトを
/// stdoutへ1回だけ出力する。stderrへは一切書かない。
///
/// exit codeの成否判定（error/warningが1件でもあれば非0）は`Text`と同じ
/// 挙動を維持する（`unsupported`はerror severityの`unsupported-obj-type`
/// 診断としてerror_countに畳み込まれるため、`Text`モードの
/// `error_count>0||warning_count>0||unsupported>0`と等価になる）。
fn run_lint_json(
    paths: &[PathBuf],
    level: Severity,
    config: &LintConfig,
    language: Language,
    tile_size_override: Option<u32>,
) -> ExitCode {
    let mut files = Vec::with_capacity(paths.len());
    let mut total_error_count = 0usize;
    let mut total_warning_count = 0usize;

    for path in paths {
        let (diagnostics, error_count, warning_count) =
            lint_one_file_json(path, level, config, language, tile_size_override);
        total_error_count += error_count;
        total_warning_count += warning_count;
        files.push(JsonFileReport {
            path: path.display().to_string(),
            diagnostics,
        });
    }

    let output = JsonLintOutput {
        files,
        summary: JsonLintSummary {
            error_count: total_error_count,
            warning_count: total_warning_count,
        },
    };

    match serde_json::to_string(&output) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            // シリアライズ自体が失敗するのはバグ（`JsonDiagnostic`は
            // `String`/`&'static str`/`Option<usize>`のみで構成され、通常は
            // 失敗し得ない）だが、`--format text`との対称性のためstdoutへは
            // 有効なJSONを出さずstderrへ報告する（JSON出力の途中まで書いて
            // 壊れたJSONをstdoutへ流すよりは安全）。
            eprintln!("failed to serialize JSON lint output: {e}");
            return ExitCode::FAILURE;
        }
    }

    if total_error_count > 0 || total_warning_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// 1ファイルをJSON向けに検証する。`lint_one_file_counts`（Text版）と同じ
/// 検証ロジック（`DatFile::parse_all`→未対応objチェック→`RuleSet::run`）を
/// たどるが、`eprintln!`/`println!`する代わりに`JsonDiagnostic`の`Vec`として
/// 蓄積して返す。`(diagnostics, error_count, warning_count)`を返す
/// （`error_count`/`warning_count`はText版の集計と同じく`level`によらず
/// 全診断を数える。`diagnostics`自体はText版が実際に1行ずつ表示する診断と
/// 同じ`d.severity <= level`でフィルタする）。
fn lint_one_file_json(
    path: &Path,
    level: Severity,
    config: &LintConfig,
    language: Language,
    tile_size_override: Option<u32>,
) -> (Vec<JsonDiagnostic>, usize, usize) {
    let records = match DatFile::parse_all(path) {
        Ok(r) => r,
        Err(e) => {
            let diag = Diagnostic::error(
                DiagnosticCode::FileReadFailed,
                t!(language,
                    ja: "読み込みに失敗しました ({e})",
                    en: "Failed to read the file ({e})",
                    e = e,
                ),
            );
            return (vec![JsonDiagnostic::from(&diag)], 1, 0);
        }
    };

    // Text版と同じく、レコード0件（obj定義が無い）は「obj=は未対応です」相当。
    if records.is_empty() {
        let diag = Diagnostic::error(
            DiagnosticCode::UnsupportedObjType,
            t!(language,
                ja: "obj= は未対応です。{list} のみ検証できます",
                en: "obj= is not supported. Only {list} can be validated",
                list = supported_obj_list(),
            ),
        );
        return (vec![JsonDiagnostic::from(&diag)], 1, 0);
    }

    let dat_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut json_diagnostics = Vec::new();
    let mut error_count = 0usize;
    let mut warning_count = 0usize;

    for dat in records.iter() {
        let obj_type = dat.get("obj").unwrap_or("").to_string();

        let Some(rule_set) = RuleSet::for_obj_type(&obj_type, dat) else {
            let diag = Diagnostic::error(
                DiagnosticCode::UnsupportedObjType,
                t!(language,
                    ja: "obj={obj_type} は未対応です。{list} のみ検証できます",
                    en: "obj={obj_type} is not supported. Only {list} can be validated",
                    obj_type = obj_type,
                    list = supported_obj_list(),
                ),
            );
            json_diagnostics.push(JsonDiagnostic::from(&diag));
            error_count += 1;
            continue;
        };

        // Text版（`lint_one_file_counts`）と同じ優先順位でタイルサイズを解決する。
        let tile_size = tile_size_override
            .or_else(|| dat.get("cell_size").and_then(|s| s.trim().parse().ok()))
            .unwrap_or_else(|| config.tile_size_for(path));

        let ctx = RuleContext {
            dat,
            dat_dir,
            language,
            tile_size,
        };
        let mut record_diags = rules::check_duplicate_keys(dat, language);
        record_diags.extend(rule_set.run(&ctx));
        record_diags.retain(|d| config.is_enabled(d.code));

        // カウントはText版の集計（`level`によらず全診断を数える）と揃える。
        for d in &record_diags {
            match d.severity {
                Severity::Error => error_count += 1,
                Severity::Warning => warning_count += 1,
                _ => {}
            }
        }
        // diagnostics配列自体は、Text版が実際にeprintln!する行と同じ
        // `d.severity <= level`でフィルタする。
        json_diagnostics.extend(
            record_diags
                .iter()
                .filter(|d| d.severity <= level)
                .map(JsonDiagnostic::from),
        );
    }

    (json_diagnostics, error_count, warning_count)
}
