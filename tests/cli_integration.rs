//! `main.rs`のCLI層（サブプロセス起動）の統合テスト。
//!
//! `apply_language_to_help`（引数helpの言語切り替え、第9弾項目1）や
//! `fmt`/`analyze`へのinclude/exclude適用（第9弾項目3）は`main.rs`の
//! プライベート関数・ロジックであり、ライブラリクレート経由では呼べないため、
//! `assert_cmd`のような追加クレートを増やさず`std::process::Command`で
//! コンパイル済みバイナリ（`CARGO_BIN_EXE_dat_linter`、Cargoが用意する
//! テスト専用の環境変数）を直接起動して検証する。

use std::path::{Path, PathBuf};
use std::process::Command;

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dat_linter"))
}

// --- 項目1: lint/fmt/analyzeの引数helpがJA/EN切り替え対応していること -----------

#[test]
fn lint_help_arg_text_is_english_by_default() {
    let output = bin().args(["lint", "-h"]).output().expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("A single file, directory, or glob pattern"),
        "デフォルト(English)でPATH引数のhelpが翻訳されているべき: {stdout}"
    );
    assert!(
        stdout.contains("Configuration file (TOML)"),
        "デフォルト(English)で--configのhelpが翻訳されているべき: {stdout}"
    );
}

#[test]
fn fmt_help_arg_text_is_english_by_default() {
    let output = bin().args(["fmt", "-h"]).output().expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Disable reordering"),
        "デフォルト(English)で--no-reorderのhelpが翻訳されているべき: {stdout}"
    );
    assert!(
        stdout.contains("Write the formatted output back to the file"),
        "デフォルト(English)で--writeのhelpが翻訳されているべき: {stdout}"
    );
}

#[test]
fn analyze_help_arg_text_is_english_by_default() {
    let output = bin().args(["analyze", "-h"]).output().expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Directory to analyze"),
        "デフォルト(English)でDIR引数のhelpが翻訳されているべき: {stdout}"
    );
    assert!(
        stdout.contains("Analysis kind"),
        "デフォルト(English)で--kindのhelpが翻訳されているべき: {stdout}"
    );
}

/// `[general] language = "ja"`を含む一時configディレクトリを作り、
/// そのディレクトリをカレントディレクトリにしてコマンドを実行する。
fn run_with_ja_config(args: &[&str], tmp_subdir: &str) -> std::process::Output {
    let tmp = std::env::temp_dir().join(format!("dat_linter_cli_test_{tmp_subdir}"));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("dat_linter.toml"),
        "[general]\nlanguage = \"ja\"\n",
    )
    .expect("config書き込みに失敗");
    let output = bin()
        .args(args)
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let _ = std::fs::remove_dir_all(&tmp);
    output
}

#[test]
fn lint_help_arg_text_switches_to_japanese_via_config() {
    let output = run_with_ja_config(&["lint", "-h"], "lint_help_ja");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("単一ファイル・ディレクトリ・globパターン"),
        "config経由でPATH引数のhelpが日本語に切り替わるべき: {stdout}"
    );
}

#[test]
fn analyze_help_arg_text_switches_to_japanese_via_config() {
    let output = run_with_ja_config(&["analyze", "-h"], "analyze_help_ja");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("解析対象のディレクトリ"),
        "config経由でDIR引数のhelpが日本語に切り替わるべき: {stdout}"
    );
    assert!(
        stdout.contains("解析種別"),
        "config経由で--kindのhelpが日本語に切り替わるべき: {stdout}"
    );
}

// --- 項目3: fmt/analyzeにもinclude/exclude制御が効くこと -----------------------

#[test]
fn fmt_excludes_configured_code() {
    // fmt_messy.datは行頭スペース行(fmt-leading-space-line)と
    // Malformed行(fmt-malformed-line)の両方でwarningを出す。
    // excludeでfmt-malformed-lineだけ抑制されることを確認する。
    let tmp = std::env::temp_dir().join("dat_linter_cli_test_fmt_exclude");
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("dat_linter.toml"),
        "[rules]\nexclude = [\"fmt-malformed-line\"]\n",
    )
    .expect("config書き込みに失敗");
    let dat_path = testdata_dir().join("fmt_messy.dat");

    let output = bin()
        .args(["fmt", dat_path.to_str().unwrap(), "--no-reorder"])
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        !stderr.contains("fmt-malformed-line"),
        "excludeで指定したfmt-malformed-lineの警告が出力されるべきではない: {stderr}"
    );
    assert!(
        stderr.contains("fmt-leading-space-line"),
        "exclude対象外のfmt-leading-space-lineは通常通り出力されるべき: {stderr}"
    );
}

#[test]
fn fmt_include_restricts_to_listed_code() {
    // includeにfmt-leading-space-lineのみ指定した場合、fmt-malformed-lineは
    // 出力されないべき（lintと同じinclude/exclude意味論）。
    let tmp = std::env::temp_dir().join("dat_linter_cli_test_fmt_include");
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("dat_linter.toml"),
        "[rules]\ninclude = [\"fmt-leading-space-line\"]\n",
    )
    .expect("config書き込みに失敗");
    let dat_path = testdata_dir().join("fmt_messy.dat");

    let output = bin()
        .args(["fmt", dat_path.to_str().unwrap(), "--no-reorder"])
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        stderr.contains("fmt-leading-space-line"),
        "includeに列挙したfmt-leading-space-lineは出力されるべき: {stderr}"
    );
    assert!(
        !stderr.contains("fmt-malformed-line"),
        "includeに列挙していないfmt-malformed-lineは出力されないべき: {stderr}"
    );
}

#[test]
fn analyze_excludes_configured_code() {
    // couplings_danglingは dangling-vehicle-constraint を必ず1件出す。
    // excludeで抑制されることを確認する。
    // 第10弾（項目4）で診断本文はstderrへ移動したため、ここではstderrを確認する。
    let tmp = std::env::temp_dir().join("dat_linter_cli_test_analyze_exclude");
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("dat_linter.toml"),
        "[rules]\nexclude = [\"dangling-vehicle-constraint\"]\n",
    )
    .expect("config書き込みに失敗");
    let dir = testdata_dir().join("couplings_dangling");

    let output = bin()
        .args(["analyze", dir.to_str().unwrap()])
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        !stderr.contains("dangling-vehicle-constraint"),
        "excludeで指定したdangling-vehicle-constraintは出力されるべきではない: {stderr}"
    );
}

// --- 項目2: `list`サブコマンドでcode一覧を取得できること ------------------------

#[test]
fn list_shows_codes_from_all_sources_by_default() {
    let output = bin().args(["list"]).output().expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("missing-cursor-icon"),
        "lint由来のcodeが表示されるべき: {stdout}"
    );
    assert!(
        stdout.contains("fmt-malformed-line"),
        "fmt由来のcodeが表示されるべき: {stdout}"
    );
    assert!(
        stdout.contains("dangling-vehicle-constraint"),
        "analyze由来のcodeが表示されるべき: {stdout}"
    );
}

#[test]
fn list_source_fmt_shows_only_fmt_codes() {
    let output = bin()
        .args(["list", "--source", "fmt"])
        .output()
        .expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fmt-malformed-line"),
        "--source fmtでfmt由来のcodeが表示されるべき: {stdout}"
    );
    assert!(
        !stdout.contains("missing-cursor-icon"),
        "--source fmtではlint由来のcodeが表示されないべき: {stdout}"
    );
    assert!(
        !stdout.contains("dangling-vehicle-constraint"),
        "--source fmtではanalyze由来のcodeが表示されないべき: {stdout}"
    );
}

#[test]
fn list_shows_disabled_status_when_excluded_via_config() {
    let tmp = std::env::temp_dir().join("dat_linter_cli_test_list_exclude");
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("dat_linter.toml"),
        "[rules]\nexclude = [\"fmt-malformed-line\"]\n",
    )
    .expect("config書き込みに失敗");

    let output = bin()
        .args(["list", "--source", "fmt"])
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_dir_all(&tmp);

    let malformed_line = stdout
        .lines()
        .find(|l| l.contains("fmt-malformed-line"))
        .unwrap_or_else(|| panic!("fmt-malformed-lineの行が見つかりません: {stdout}"));
    assert!(
        malformed_line.contains("disabled"),
        "excludeされたcodeはdisabledと表示されるべき: {malformed_line}"
    );

    let leading_space_line = stdout
        .lines()
        .find(|l| l.contains("fmt-leading-space-line"))
        .unwrap_or_else(|| panic!("fmt-leading-space-lineの行が見つかりません: {stdout}"));
    assert!(
        leading_space_line.contains("enabled"),
        "exclude対象外のcodeはenabledのままであるべき: {leading_space_line}"
    );
}

#[test]
fn analyze_without_config_shows_dangling_constraint_by_default() {
    // configを指定しない（デフォルト=all）場合は従来通り出力されることの対照実験。
    // 第10弾（項目4）で診断本文はstderrへ移動したため、ここではstderrを確認する。
    let tmp = std::env::temp_dir().join("dat_linter_cli_test_analyze_default");
    let _ = std::fs::create_dir_all(&tmp);
    // 明示的にdat_linter.tomlを置かない（自動生成されるデフォルトは
    // include/exclude空=all許可のため、このテストの意図には影響しない）。
    let dir = testdata_dir().join("couplings_dangling");

    let output = bin()
        .args(["analyze", dir.to_str().unwrap()])
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_file(tmp.join("dat_linter.toml"));
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        stderr.contains("dangling-vehicle-constraint"),
        "config指定無し(=all許可)ではdangling-vehicle-constraintが出力されるべき: {stderr}"
    );
}
