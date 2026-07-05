//! `rules::check_citycar` の統合テスト。testdata/ の正常系2件（8方向全定義/一部のみ定義）・
//! 異常系2件で、期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/tree_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した citycar dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_citycar(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
}

#[test]
fn valid_citycar_has_no_errors_or_warnings() {
    let diags = check("citycar_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    let warnings: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
    assert!(warnings.is_empty(), "予期しない warning: {warnings:?}");
}

#[test]
fn partial_direction_images_has_no_errors_or_warnings() {
    // citycar_writer.cc の8方向走査は無条件（早期終了なし）で、vehicleのような
    // 「一部方向だけ定義」を検出するfatal分岐が存在しないため、2方向だけ定義しても
    // error/warningは出ない（詳細はrules/citycar.rs冒頭のREJECTEDコメント参照）。
    let diags = check("citycar_partial_images_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    let warnings: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
    assert!(warnings.is_empty(), "予期しない warning: {warnings:?}");
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("citycar_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("citycar_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}
