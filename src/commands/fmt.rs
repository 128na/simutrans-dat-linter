//! `fmt`サブコマンドの実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で切り出した。振る舞いは分割前と完全に同一。

use crate::cli::FmtArgs;
use crate::fs_walk::collect_dat_paths;
use dat_linter::codes::DiagnosticCode;
use dat_linter::config::LintConfig;
use dat_linter::formatter;
use dat_linter::i18n::{Language, t};
use dat_linter::parser::read_dat_text;
use std::path::Path;
use std::process::ExitCode;

pub fn run_fmt(args: &FmtArgs, language: Language) -> ExitCode {
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
    // 第11弾: 専用の[fmt] reorder設定を廃止し、[rules] include/excludeの
    // 仕組みに統合した（reorder自体をDiagnosticCode::FmtReorderAppliedという
    // codeで表現する。config.rs冒頭のdocコメント「`fmt`のreorder挙動」参照）。
    // 優先順位: --no-reorder指定 > config設定（excludeに無ければ有効＝デフォルトtrue相当）。
    let should_reorder = !args.no_reorder && config.is_enabled(DiagnosticCode::FmtReorderApplied);

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
        // 第12弾: 第11弾では、reorderが実際に適用されたことを示すInfo診断
        // （code DiagnosticCode::FmtReorderApplied）をここで生成・eprintln!して
        // いたが、これにより問題の無い通常のfmt実行が毎回1行stderrへ出力する
        // ようになり、「指摘が無ければ完全silent」というlint/analyzeと同じ方針に
        // 反する副作用があった（Main側で発見）。FmtReorderAppliedはreorder機能の
        // 有効/無効を`[rules] include/exclude`で切り替えるためだけの機能トグル
        // codeであり、実際に診断として発行する必要は無い（有効/無効の判定自体は
        // `should_reorder`の算出（上部の`config.is_enabled`呼び出し）に残っている）。
        // `tests/codes_completeness.rs::FEATURE_TOGGLE_ONLY_CODES`にこの種のcode向けの
        // 明示的allowlistを設けたため、ここで診断オブジェクト自体を生成・出力する
        // 必要が無くなった。
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
