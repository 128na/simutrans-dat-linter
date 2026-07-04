//! CLIエントリポイント。引数パース（`--config`の先読み→言語決定→clapパース）と
//! 各サブコマンドへのディスパッチのみを担う。
//!
//! 第13弾: 以前はCLI引数定義・各サブコマンドの実行ロジック・ファイル収集
//! ユーティリティが全てこの1ファイルに同居していた（SRP的な肥大化）ため、
//! `src/cli.rs`（CLI定義）・`src/fs_walk.rs`（ファイル収集）・
//! `src/commands/*.rs`（サブコマンドごとの実行ロジック）へ分割した。
//! 振る舞い（stdout/stderr出力・exit code・ヘルプ文言）は分割前と完全に同一。

mod cli;
mod commands;
mod fs_walk;

use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, Command, apply_language_to_help, peek_config_arg};
use dat_linter::config::LintConfig;
use std::process::ExitCode;

fn main() -> ExitCode {
    // 1. 設定ファイル（言語含む）を、clapによる本来の引数解釈より先に読み込む。
    //    `--help`/`-h`はclapが`get_matches()`内で検出し即座に終了するため、
    //    ヘルプ表示にも翻訳後の言語を反映するにはこの順序が必須。
    //    設定ファイル自動生成（初回起動時）もこのタイミングで発生する。
    let explicit_config_path = peek_config_arg();
    let config = match LintConfig::load_or_default(explicit_config_path.as_deref()) {
        Ok(c) => c,
        Err(_) => {
            // 設定読み込み失敗時もヘルプ表示自体は継続できるよう、ここでは
            // デフォルト設定にフォールバックする。実際のエラー報告は
            // 各run_*関数内で`LintConfig::load_or_default`を再度呼んだ際に行う
            // （`--config`に不正なパスを渡した場合の従来のエラー経路を維持するため）。
            LintConfig::all_enabled()
        }
    };
    let language = config.language();

    // 2. `Cli::command()`（`CommandFactory`経由）で得たclap::Commandの短いabout
    //    文言を言語に応じて上書きしてから`get_matches()`する。これにより
    //    `--help`/`-h`表示にも翻訳が反映される。
    let cmd = apply_language_to_help(Cli::command(), language);
    let matches = cmd.get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    match cli.command {
        Command::Lint(args) => commands::lint::run_lint(&args, language),
        Command::Fmt(args) => commands::fmt::run_fmt(&args, language),
        Command::Analyze(args) => commands::analyze::run_analyze(&args, language),
        Command::List(args) => commands::list::run_list(&args, language),
        Command::Describe(args) => commands::describe::run_describe(&args, language),
    }
}
