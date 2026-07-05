//! `lint`/`fmt`共通の制御フロー（config読み込み後の「パス解決→単一/複数ファイル
//! 分岐→集計→exit code判定」骨組み）を1箇所にまとめる。
//!
//! 第18弾（code smellレビュー・タスク14）: `run_lint`（`lint.rs`）と`run_fmt`
//! （`fmt.rs`）が、以下のほぼ同型の制御フローをそれぞれ独立に実装していた
//! （code smellレビューで指摘）:
//! 1. `collect_dat_paths`でパス解決（空ならエラー表示してFAILURE）
//! 2. 単一ファイルなら専用の出力・exit codeパスへ（サマリ行を追加しない）
//! 3. 複数ファイルならループして各ファイルを処理し、件数を集計
//! 4. 指摘が無ければサマリ行も含めて何も出力しない、あれば集計サマリ行を出力
//! 5. 集計結果に基づいてexit codeを決定
//!
//! `fmt`固有の追加分岐（`--write`無し・複数ファイルマッチ時のエラー等）は
//! `fmt.rs`側にそのまま残し、無理にこのヘルパーへ押し込めていない。
//! `analyze --kind coupling`はディレクトリ1つだけを処理する構造で、
//! 「パス解決→単一/複数分岐」という骨組み自体が無い（複数ファイル分岐が
//! 存在しない）ため、このヘルパーの対象外とした。

use crate::fs_walk::collect_dat_paths;
use dat_linter::i18n::{Language, t};
use std::path::PathBuf;
use std::process::ExitCode;

/// `collect_dat_paths`を呼び、空だった場合は共通のエラーメッセージを表示して
/// `Err(ExitCode::FAILURE)`を返す。`lint`/`fmt`双方の冒頭（config読み込み後・
/// pathの引数名は`arg_path`として渡す）で全く同じだった分岐。
pub fn resolve_paths_or_exit(arg_path: &str, language: Language) -> Result<Vec<PathBuf>, ExitCode> {
    let paths = collect_dat_paths(arg_path, language).map_err(|e| {
        eprintln!("{arg_path}: {e}");
        ExitCode::FAILURE
    })?;

    if paths.is_empty() {
        eprintln!(
            "{}",
            t!(language,
                ja: "{path}: 該当する .dat ファイルが見つかりません",
                en: "{path}: No matching .dat files were found",
                path = arg_path,
            )
        );
        return Err(ExitCode::FAILURE);
    }

    Ok(paths)
}

/// 複数ファイル処理時の集計結果。`error_count`/`warning_count`は全ファイル分の
/// 合計、`any_failure`はいずれか1ファイルでも失敗扱いだったか
/// （`lint`は`unsupported`等もここに畳み込む。`fmt`は`error_count`を常に0とし
/// `warning_count`のみで失敗判定するため、`error_count`はlint専用の集計軸）。
pub struct AggregateCounts {
    pub error_count: usize,
    pub warning_count: usize,
    pub any_failure: bool,
}

/// `paths`（2件以上、単一ファイル分岐は呼び出し元が別途処理済みの前提）を
/// `process_one`で1件ずつ処理し、`(error_count, warning_count, is_failure)`を
/// 集計する。`lint.rs::lint_one_file_counts`・`fmt.rs::fmt_one_file`（の
/// `warning_count`部分）がこの`process_one`に対応する。
pub fn aggregate_multi_file<F>(paths: &[PathBuf], mut process_one: F) -> AggregateCounts
where
    F: FnMut(&std::path::Path) -> (usize, usize, bool),
{
    let mut error_count = 0usize;
    let mut warning_count = 0usize;
    let mut any_failure = false;

    for path in paths {
        let (e, w, failed) = process_one(path);
        error_count += e;
        warning_count += w;
        any_failure |= failed;
    }

    AggregateCounts {
        error_count,
        warning_count,
        any_failure,
    }
}

/// 集計結果に基づく共通のexit code判定。`any_failure`（`lint`は
/// `error_count>0||warning_count>0||unsupported>0`を畳み込んだもの、`fmt`は
/// `warning_count>0`または書き込み失敗）が真なら`FAILURE`、そうでなければ`SUCCESS`。
pub fn exit_code_for(any_failure: bool) -> ExitCode {
    if any_failure {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
