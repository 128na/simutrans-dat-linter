//! `lint`/`fmt`共通のファイル収集ユーティリティ（単一ファイル・ディレクトリ再帰・
//! globパターンの3通りの入力を実在する`.dat`ファイルパス一覧へ解決する）。
//!
//! 第13弾: `src/main.rs`のSRP分割で、CLI定義（`src/cli.rs`）・各サブコマンドの
//! 実行ロジック（`src/commands/*.rs`）から独立したユーティリティとして
//! このモジュールに切り出した。振る舞いは分割前と完全に同一。

use dat_linter::i18n::{Language, t};
use dat_linter::registry::SUPPORTED_OBJ_TYPES;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 未対応obj種別のエラーメッセージ末尾に付ける対応obj一覧
/// （`obj=building / obj=vehicle / ...`）。obj種別名自体（`building`等）は
/// 翻訳対象外（`.dat`に実際に書く値そのものであり、変えるとユーザーが混乱する）。
pub fn supported_obj_list() -> String {
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
pub fn collect_dat_paths(input: &str, lang: Language) -> Result<Vec<PathBuf>, String> {
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
