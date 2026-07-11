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

// --- 第10弾項目6: `describe`サブコマンドでcodeの説明を表示できること ------------

#[test]
fn describe_known_code_shows_why_and_how_to_fix() {
    let output = bin()
        .args(["describe", "obsolete-type"])
        .output()
        .expect("起動に失敗");
    assert!(output.status.success(), "既知のcodeはexit成功であるべき");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("obsolete-type"),
        "指定したcode自体が表示されるべき: {stdout}"
    );
    assert!(
        stdout.contains("Why:"),
        "英語(デフォルト)では\"Why:\"見出しが表示されるべき: {stdout}"
    );
    assert!(
        stdout.contains("How to fix:"),
        "\"How to fix:\"見出しが表示されるべき: {stdout}"
    );
}

#[test]
fn describe_unknown_code_fails_with_list_hint() {
    let output = bin()
        .args(["describe", "this-code-does-not-exist"])
        .output()
        .expect("起動に失敗");
    assert!(!output.status.success(), "不明なcodeはexit失敗であるべき");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("this-code-does-not-exist"),
        "指定した不明なcode自体がエラーメッセージに含まれるべき: {stderr}"
    );
    assert!(
        stderr.contains("dat_linter list"),
        "listコマンドへの案内が含まれるべき: {stderr}"
    );
}

#[test]
fn describe_switches_to_japanese_via_config() {
    let output = run_with_ja_config(&["describe", "obsolete-type"], "describe_ja");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("なぜNGか"),
        "config経由で見出しが日本語に切り替わるべき: {stdout}"
    );
    assert!(
        stdout.contains("どう直すか"),
        "config経由で見出しが日本語に切り替わるべき: {stdout}"
    );
}

#[test]
fn describe_help_arg_text_is_english_by_default() {
    let output = bin().args(["describe", "-h"]).output().expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Show the description"),
        "デフォルト(English)でaboutが翻訳されているべき: {stdout}"
    );
    assert!(
        stdout.contains("dat_linter list"),
        "CODE引数のhelpが表示されるべき: {stdout}"
    );
}

// --- pak64/pak192等マルチサイズ対応: `--tile-size`によるタイルサイズ上書き -------

#[test]
fn lint_help_arg_text_mentions_tile_size() {
    let output = bin().args(["lint", "-h"]).output().expect("起動に失敗");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--tile-size"),
        "lintのhelpに--tile-sizeが表示されるべき: {stdout}"
    );
}

/// `testdata/citycar_bad_image_size.dat`が参照する`bad_size.png`は64x64
/// （128の倍数ではないが64の倍数）。デフォルト（128）ではエラーになるが、
/// `--tile-size 64`を渡すとタイルサイズの倍数チェックを通ることを確認する
/// （CLI引数がconfig/cell_size=より優先される、という優先順位のエンドツーエンド確認）。
#[test]
fn default_tile_size_rejects_64x64_image() {
    let path = testdata_dir().join("citycar_bad_image_size.dat");
    let output = bin()
        .args(["lint", path.to_str().unwrap()])
        .output()
        .expect("起動に失敗");
    assert!(
        !output.status.success(),
        "デフォルト(128)では64x64画像はエラーになるべき"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("image-size-not-multiple-of-128"),
        "128の倍数でないエラーが出るべき: {stderr}"
    );
}

#[test]
fn tile_size_flag_overrides_default_and_accepts_64x64_image() {
    let path = testdata_dir().join("citycar_bad_image_size.dat");
    let output = bin()
        .args(["lint", path.to_str().unwrap(), "--tile-size", "64"])
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("image-size-not-multiple-of-128"),
        "--tile-size 64指定時は64x64画像がサイズエラーにならないべき: {stderr}"
    );
}

/// `--tile-size`は`dat_linter.toml`の`[tile_size] default`より優先される。
#[test]
fn tile_size_flag_overrides_config_default() {
    let tmp = std::env::temp_dir().join("dat_linter_cli_test_tile_size_cli_over_config");
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(tmp.join("dat_linter.toml"), "[tile_size]\ndefault = 32\n")
        .expect("config書き込みに失敗");
    let path = testdata_dir().join("citycar_bad_image_size.dat");

    let output = bin()
        .args(["lint", path.to_str().unwrap(), "--tile-size", "64"])
        .current_dir(&tmp)
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        !stderr.contains("image-size-not-multiple-of-128"),
        "config([tile_size] default=32)よりCLIの--tile-size 64が優先されるべき: {stderr}"
    );
}

/// `.dat`自身の`cell_size=`（`obj_writer.cc`の実在フィールド）は、`--tile-size`が
/// 指定されない限りconfig/デフォルトより優先される。
/// `testdata/citycar_cell_size_override.dat`は`citycar_bad_image_size.dat`と同じ
/// 64x64画像参照に`cell_size=64`を追加したもの。
#[test]
fn cell_size_field_overrides_config_default_when_no_cli_flag() {
    let path = testdata_dir().join("citycar_cell_size_override.dat");
    let output = bin()
        .args(["lint", path.to_str().unwrap()])
        .output()
        .expect("起動に失敗");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("image-size-not-multiple-of-128"),
        "cell_size=64指定時はデフォルト(128)ではなく64を基準に検証されるべき: {stderr}"
    );
}

// --- `fmt`: 改行コード(CRLF/LF)の保持 ------------------------------------------
//
// `fmt`は入力の改行コードに関わらず常にLFで出力してしまうバグがあった
// （`formatter::format_preserve_order`/`format_reordered`は内部的に常に`\n`で
// 行を組み立てるため）。テスト用のCRLF入力は`testdata/*.dat`としてgitに
// コミットしない（gitの`autocrlf`設定次第でcheckout時に改行コードが書き換わる
// リスクがあるため）。代わりに`std::env::temp_dir()`配下へ`\r\n`を明示的に
// 含むバイト列を都度書き出し、テスト終了時に削除する。

/// CRLFを明示的に含む一時`.dat`ファイルを作成し、そのパスを返す。
fn write_crlf_dat(tmp_subdir: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("dat_linter_cli_test_{tmp_subdir}"));
    let _ = std::fs::create_dir_all(&tmp);
    let path = tmp.join("crlf_test.dat");
    let content = "Obj=building\r\nname=crlf_test\r\ntype=extension\r\nwaytype=track\r\nenables_pax=1\r\nDims=1,1,4\r\ncursor=icon.0.0\r\nicon=icon.0.0\r\n";
    std::fs::write(&path, content).expect("CRLFテスト用ファイルの書き込みに失敗");
    path
}

/// `bytes`中に「直前が`\r`でない`\n`」（＝CRLFでない裸のLF）が1つでも
/// 含まれていれば`true`を返す。
fn has_bare_lf(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .enumerate()
        .any(|(i, &b)| b == b'\n' && (i == 0 || bytes[i - 1] != b'\r'))
}

#[test]
fn fmt_preserve_order_stdout_keeps_crlf_line_endings() {
    let path = write_crlf_dat("fmt_crlf_preserve_stdout");
    let output = bin()
        .args(["fmt", path.to_str().unwrap(), "--no-reorder"])
        .output()
        .expect("起動に失敗");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert!(
        output.stdout.windows(2).any(|w| w == b"\r\n"),
        "--no-reorder時、CRLF入力の標準出力はCRLFを保持するべき: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !has_bare_lf(&output.stdout),
        "CRLF入力の標準出力にLF単独の改行が混在するべきではない: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn fmt_reorder_stdout_keeps_crlf_line_endings() {
    let path = write_crlf_dat("fmt_crlf_reorder_stdout");
    let output = bin()
        .args(["fmt", path.to_str().unwrap()])
        .output()
        .expect("起動に失敗");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert!(
        output.stdout.windows(2).any(|w| w == b"\r\n"),
        "--reorder(デフォルト)時も、CRLF入力の標準出力はCRLFを保持するべき: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !has_bare_lf(&output.stdout),
        "--reorder時もCRLF入力の標準出力にLF単独の改行が混在するべきではない: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn fmt_write_keeps_crlf_line_endings() {
    let path = write_crlf_dat("fmt_crlf_write");
    let output = bin()
        .args(["fmt", path.to_str().unwrap(), "--write"])
        .output()
        .expect("起動に失敗");
    assert!(
        output.status.success(),
        "--write は成功するべき: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let written = std::fs::read(&path).expect("書き込み結果の読み込みに失敗");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert!(
        written.windows(2).any(|w| w == b"\r\n"),
        "--write でファイルへ書き戻された内容もCRLFを保持するべき: {:?}",
        String::from_utf8_lossy(&written)
    );
    assert!(
        !has_bare_lf(&written),
        "--write でファイルへ書き戻された内容にLF単独の改行が混在するべきではない: {:?}",
        String::from_utf8_lossy(&written)
    );
}

/// 既存のLF前提testdata（`fmt_example.dat`）に対する挙動が、この改行コード
/// 保持実装の前後で変わらないこと（LF入力はLF出力のまま）の対照実験。
#[test]
fn fmt_lf_input_stdout_has_no_crlf() {
    let path = testdata_dir().join("fmt_example.dat");
    let output = bin()
        .args(["fmt", path.to_str().unwrap(), "--no-reorder"])
        .output()
        .expect("起動に失敗");

    assert!(
        !output.stdout.windows(2).any(|w| w == b"\r\n"),
        "LF入力の標準出力にCRLFが混入するべきではない: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
}
