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
/// 戻り値の1つ目は入力順に依らず安定した順序（パス文字列の辞書順）に揃えた
/// `.dat`パス一覧。2つ目（`bool`）は、再帰走査中に読み取れなかった
/// サブディレクトリが1件以上あったか（`true`なら呼び出し側は「サイレント成功」
/// にしてはいけない。詳細は`collect_dat_files_recursive`のdocコメント参照）。
pub fn collect_dat_paths(input: &str, lang: Language) -> Result<(Vec<PathBuf>, bool), String> {
    let has_glob_meta = input.contains(['*', '?', '[']);
    let mut had_unreadable_dir = false;

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
                        collect_dat_files_recursive(&p, &mut paths, lang, &mut had_unreadable_dir);
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
        return Ok((paths.into_iter().collect(), had_unreadable_dir));
    }

    let path = Path::new(input);
    if path.is_dir() {
        let mut paths = BTreeSet::new();
        collect_dat_files_recursive(path, &mut paths, lang, &mut had_unreadable_dir);
        return Ok((paths.into_iter().collect(), had_unreadable_dir));
    }

    Ok((vec![path.to_path_buf()], false))
}

fn is_dat_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("dat"))
        .unwrap_or(false)
}

/// `dir`配下を再帰的に走査し、`.dat`ファイルを`out`へ収集する。
///
/// ## ディレクトリsymlinkの循環対策
/// `entry.file_type()`（`DirEntry`が`read_dir`時点で保持しているlstat相当の情報）を
/// 使い、エントリ自身がsymlink（Windowsのjunction含む）かどうかを判定する。
/// symlinkであれば実体がディレクトリであっても**再帰対象から除外する**
/// （`path.is_dir()`はsymlinkを解決してしまうため、自己参照・祖先ディレクトリを
/// 指すディレクトリsymlinkが1つあるだけで無限再帰しスタックオーバーフローで
/// クラッシュしていた）。ファイルsymlinkは除外せず、従来通り拡張子判定で
/// `.dat`ファイル一覧に含める（symlink経由の**読み取り**自体の実害は小さいため。
/// 書き込み時の対策は`commands/fmt.rs`の`fmt_one_file`側で別途行う）。
///
/// ## 権限エラー等で読めないサブディレクトリの扱い
/// 以前は`read_dir`の失敗を`let-else`で握り潰し、無診断でスキップしていた。
/// このツールは「指摘が無ければ完全silent」という設計方針（第10弾）のため、
/// 黙ってスキップすると"権限エラーで一部ファイルを見ていない"状態と
/// "本当に全ファイルがクリーン"な状態が区別できなくなる。読めなかった場合は
/// 標準エラー出力へ警告を出し、`had_error`を`true`にする
/// （呼び出し側の`collect_dat_paths`経由でexit codeに反映される）。
fn collect_dat_files_recursive(
    dir: &Path,
    out: &mut BTreeSet<PathBuf>,
    lang: Language,
    had_error: &mut bool,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "{}",
                t!(lang,
                    ja: "ディレクトリ {dir} を読み取れませんでした ({e})",
                    en: "Failed to read directory {dir} ({e})",
                    dir = dir.display(),
                    e = e,
                )
            );
            *had_error = true;
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_real_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if is_real_dir {
            collect_dat_files_recursive(&path, out, lang, had_error);
        } else if is_dat_file(&path) {
            out.insert(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn make_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn make_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    fn unique_tmp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dat_linter_fs_walk_test_{name}"))
    }

    /// 修正1: 自己参照するディレクトリsymlinkがあっても無限再帰せず、
    /// 通常の`.dat`ファイルは正常に収集されることを確認する。
    ///
    /// Windowsではシンボリックリンク作成に管理者権限（またはデベロッパーモード）が
    /// 必要な場合があるため、作成に失敗する環境ではテストをスキップする
    /// （CIがそのような制限された環境で実行される可能性を考慮）。
    #[test]
    fn collect_dat_files_recursive_does_not_infinitely_recurse_on_self_referential_symlink() {
        let tmp = unique_tmp_dir("self_loop");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("一時ディレクトリの作成に失敗");

        std::fs::write(tmp.join("real.dat"), "obj=building\n").expect("dat書き込みに失敗");

        let link = tmp.join("self_loop");
        if make_dir_symlink(&tmp, &link).is_err() {
            eprintln!(
                "symlink作成に失敗したため collect_dat_files_recursive_does_not_infinitely_recurse_on_self_referential_symlink をスキップします\
                 （管理者権限/デベロッパーモードが無い環境の可能性）"
            );
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        let mut out = BTreeSet::new();
        let mut had_error = false;
        collect_dat_files_recursive(&tmp, &mut out, Language::default(), &mut had_error);

        assert!(
            out.iter()
                .any(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())
                    == Some("real.dat".to_string())),
            "自己参照symlinkの循環を無限再帰せず切り抜け、real.datは収集されるべき: {out:?}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 修正1: 祖先ディレクトリを指すディレクトリsymlink（自己参照よりも
    /// 一般的なケース）でも無限再帰しないことを確認する。
    #[test]
    fn collect_dat_files_recursive_does_not_infinitely_recurse_on_ancestor_symlink() {
        let tmp = unique_tmp_dir("ancestor_loop");
        let _ = std::fs::remove_dir_all(&tmp);
        let child = tmp.join("child");
        std::fs::create_dir_all(&child).expect("一時ディレクトリの作成に失敗");

        std::fs::write(tmp.join("real.dat"), "obj=building\n").expect("dat書き込みに失敗");

        // child/back_to_root -> tmp（祖先ディレクトリを指すsymlink）
        let link = child.join("back_to_root");
        if make_dir_symlink(&tmp, &link).is_err() {
            eprintln!(
                "symlink作成に失敗したため collect_dat_files_recursive_does_not_infinitely_recurse_on_ancestor_symlink をスキップします\
                 （管理者権限/デベロッパーモードが無い環境の可能性）"
            );
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        let mut out = BTreeSet::new();
        let mut had_error = false;
        collect_dat_files_recursive(&tmp, &mut out, Language::default(), &mut had_error);

        assert!(
            out.iter()
                .any(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())
                    == Some("real.dat".to_string())),
            "祖先ディレクトリへのsymlink循環を無限再帰せず切り抜け、real.datは収集されるべき: {out:?}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 修正2: 権限エラー等でサブディレクトリが読めない場合、無診断でスキップせず
    /// `had_error`が`true`になることを確認する。
    ///
    /// Windowsでの読み取り拒否の再現（`icacls`等）はテスト実行環境（CI含む）
    /// ごとの権限モデルに依存し再現性に乏しいため、`chmod`で確実に再現できる
    /// Unix環境限定のテストとする（Windows側は実装のみ。CLAUDE.mdの指示通り、
    /// 難しいテストは見送ってその旨を報告する）。
    #[cfg(unix)]
    #[test]
    fn collect_dat_files_recursive_reports_unreadable_subdirectory() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = unique_tmp_dir("unreadable_subdir");
        let _ = std::fs::remove_dir_all(&tmp);
        let unreadable = tmp.join("unreadable");
        std::fs::create_dir_all(&unreadable).expect("一時ディレクトリの作成に失敗");
        std::fs::write(unreadable.join("hidden.dat"), "obj=building\n").expect("dat書き込みに失敗");
        std::fs::write(tmp.join("visible.dat"), "obj=building\n").expect("dat書き込みに失敗");

        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))
            .expect("権限変更に失敗");

        // root権限で実行されている場合はパーミッションが無視され読めてしまうため、
        // その場合はテストの前提が成立しないとみなしてスキップする。
        if std::fs::read_dir(&unreadable).is_ok() {
            eprintln!(
                "root権限等でディレクトリ権限が無視される環境のため collect_dat_files_recursive_reports_unreadable_subdirectory をスキップします"
            );
            let _ = std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o755));
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        let mut out = BTreeSet::new();
        let mut had_error = false;
        collect_dat_files_recursive(&tmp, &mut out, Language::default(), &mut had_error);

        assert!(
            had_error,
            "権限エラーで読めないディレクトリがあればhad_errorがtrueになるべき"
        );
        assert!(
            out.iter()
                .any(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())
                    == Some("visible.dat".to_string())),
            "読み取り可能な他のファイルは通常通り収集されるべき: {out:?}"
        );

        // クリーンアップのため権限を戻してから削除する。
        let _ = std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o755));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
