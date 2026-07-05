//! `describe`サブコマンドの実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で切り出した。振る舞いは分割前と完全に同一。

use crate::cli::DescribeArgs;
use dat_linter::codes::DiagnosticCode;
use dat_linter::config::LintConfig;
use dat_linter::i18n::{Language, t};
use std::process::ExitCode;

/// 第10弾（項目6）: 指定したcodeの説明（なぜNGか・どう直すか）を表示する。
/// `DiagnosticCode::from_str`（`codes::ALL`の線形探索、`list`と同じ一覧）で
/// 文字列からcodeを検索し、見つかれば`why`/`how_to_fix`をJA/ENに応じて表示する。
/// 見つからない場合は`list`コマンドの案内を添えてexit failureにする。
pub fn run_describe(args: &DescribeArgs, language: Language) -> ExitCode {
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

    let Some(code) = DiagnosticCode::from_str(&args.code) else {
        eprintln!(
            "{}",
            t!(language,
                ja: "{code}: 不明なcodeです。`dat_linter list` で有効なcode一覧を確認してください",
                en: "{code}: Unknown code. Run `dat_linter list` to see the list of valid codes",
                code = args.code,
            )
        );
        return ExitCode::FAILURE;
    };
    let info = code.info();

    let enabled = config.is_enabled(info.code);
    let status = match (enabled, language) {
        (true, Language::Japanese) => "有効",
        (true, Language::English) => "enabled",
        (false, Language::Japanese) => "無効",
        (false, Language::English) => "disabled",
    };

    println!(
        "{:<12} {} ({status})",
        info.source.as_str(),
        info.code.as_str()
    );
    println!();
    println!(
        "{}",
        t!(language,
            ja: "なぜNGか:",
            en: "Why:",
        )
    );
    println!("{}", info.why(language));
    println!();
    println!(
        "{}",
        t!(language,
            ja: "どう直すか:",
            en: "How to fix:",
        )
    );
    println!("{}", info.how_to_fix(language));

    ExitCode::SUCCESS
}
