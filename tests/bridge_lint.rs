//! `rules::check_bridge` の統合テスト。testdata/ の正常系1件・異常系5件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/way_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した bridge dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_bridge(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code))
        .collect()
}

fn has(diags: &[(Severity, &str)], severity: Severity, code: &str) -> bool {
    diags.iter().any(|(s, c)| *s == severity && *c == code)
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    has(diags, Severity::Error, code)
}

#[test]
fn valid_bridge_has_no_errors_or_warnings() {
    let diags = check("bridge_valid.dat");
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
fn missing_waytype_is_detected() {
    assert!(has_error(
        &check("bridge_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("bridge_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn clamped_value_out_of_range_is_detected() {
    assert!(has(
        &check("bridge_clamped_out_of_range.dat"),
        Severity::Warning,
        "clamped-value-out-of-range"
    ));
}

#[test]
fn missing_front_image_is_warned() {
    assert!(has(
        &check("bridge_missing_front_image.dat"),
        Severity::Warning,
        "no-bridge-image-specified"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("bridge_missing_image_file.dat"),
        "missing-image-file"
    ));
}
