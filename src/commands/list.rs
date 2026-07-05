//! `list`サブコマンドの実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で切り出した。振る舞いは分割前と完全に同一。

use crate::cli::{ListArgs, ListSourceArg};
use dat_linter::config::LintConfig;
use dat_linter::i18n::{Language, t};
use std::process::ExitCode;

/// 第9弾（項目2）: `dat_linter.toml`の`[rules] include/exclude`に書けるcode
/// （`Diagnostic.code`）の一覧を表示する。一覧自体は`codes::all_codes()`
/// （`codes::ALL`から導出。実ソースとの整合性は`tests/codes_completeness.rs`が保証）
/// から取得する。`--config`が指定された場合、各codeが現在の設定で有効か無効かも
/// 併記する（設定ファイルを編集する前に効果を確認できるようにするため）。
pub fn run_list(args: &ListArgs, language: Language) -> ExitCode {
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

    let wanted_source = args.source.map(ListSourceArg::to_code_source);
    let mut shown = 0usize;
    for info in dat_linter::codes::all_codes() {
        if let Some(w) = wanted_source
            && w != info.source
        {
            continue;
        }
        shown += 1;
        let enabled = config.is_enabled(info.code);
        let status = match (enabled, language) {
            (true, Language::Japanese) => "有効",
            (true, Language::English) => "enabled",
            (false, Language::Japanese) => "無効",
            (false, Language::English) => "disabled",
        };
        println!(
            "{:<12} {:<45} {status}",
            info.source.as_str(),
            info.code.as_str()
        );
    }

    println!(
        "{}",
        t!(language,
            ja: "合計 {shown} 件のcode",
            en: "Total: {shown} code(s)",
            shown = shown,
        )
    );

    ExitCode::SUCCESS
}
