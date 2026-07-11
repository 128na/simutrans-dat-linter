//! `init`サブコマンドの実行ロジック。
//!
//! `lint`/`fmt`はかつて、`--config`未指定かつカレントディレクトリに
//! `dat_linter.toml`が無い場合にカレントディレクトリへ自動生成していたが、
//! この暗黙の副作用が意図しないディレクトリへの誤生成・テスト汚染の原因に
//! なったため廃止した。設定ファイルの生成はこの明示的な`dat_linter init`
//! サブコマンドに一本化する。テンプレート自体は`dat_linter::config`の
//! `generate_default_config_file`（`create_new`で書き込むため、既存ファイルへの
//! 上書きは`io::ErrorKind::AlreadyExists`のエラーになる）をそのまま再利用する。

use crate::cli::InitArgs;
use dat_linter::config::{DEFAULT_CONFIG_FILENAME, generate_default_config_file};
use dat_linter::i18n::{Language, t};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::ExitCode;

/// カレントディレクトリに`dat_linter.toml`を生成する。既に存在する場合は
/// 上書きせず、その旨を報告して`ExitCode::FAILURE`を返す。
pub fn run_init(_args: &InitArgs, language: Language) -> ExitCode {
    let path = PathBuf::from(DEFAULT_CONFIG_FILENAME);
    match generate_default_config_file(&path) {
        Ok(()) => {
            println!(
                "{}",
                t!(language,
                    ja: "{p} を作成しました",
                    en: "Created {p}",
                    p = path.display(),
                )
            );
            ExitCode::SUCCESS
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "{p} は既に存在します。上書きしません",
                    en: "{p} already exists. Not overwriting it",
                    p = path.display(),
                )
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!(
                "{}",
                t!(language,
                    ja: "{p} の作成に失敗しました ({e})",
                    en: "Failed to create {p} ({e})",
                    p = path.display(),
                    e = e,
                )
            );
            ExitCode::FAILURE
        }
    }
}
