//! `rules::check_factory` の統合テスト。testdata/ の正常系1件・異常系各種で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/pedestrian_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した factory dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_factory(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
}

fn has_warning(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Warning && *c == code)
}

fn assert_no_errors_or_warnings(diags: &[(Severity, &str)]) {
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
fn valid_factory_has_no_errors_or_warnings() {
    assert_no_errors_or_warnings(&check("factory_valid.dat"));
}

#[test]
fn missing_mapcolor_is_detected() {
    assert!(has_error(
        &check("factory_missing_mapcolor.dat"),
        "factory-missing-mapcolor"
    ));
}

#[test]
fn type_override_is_detected() {
    assert!(has_error(
        &check("factory_type_override.dat"),
        "factory-type-override"
    ));
}

#[test]
fn zero_dims_is_detected() {
    assert!(has_error(&check("factory_zero_dims.dat"), "zero-size"));
}

#[test]
fn missing_cursor_icon_is_detected() {
    assert!(has_error(
        &check("factory_missing_cursor_icon.dat"),
        "missing-cursor-icon"
    ));
}

#[test]
fn missing_tile_image_is_detected() {
    assert!(has_error(
        &check("factory_missing_tile_image.dat"),
        "missing-tile-image"
    ));
}

#[test]
fn output_capacity_too_small_is_detected() {
    assert!(has_warning(
        &check("factory_output_capacity_too_small.dat"),
        "factory-output-capacity-too-small"
    ));
}

#[test]
fn smoketile_without_offset_is_detected() {
    assert!(has_warning(
        &check("factory_smoketile_without_offset.dat"),
        "factory-smoketile-without-offset"
    ));
}

#[test]
fn probability_clamped_is_detected() {
    let diags = check("factory_probability_clamped.dat");
    assert!(has_warning(&diags, "factory-probability-clamped"));
    // probability_to_spawn と expand_probability の両方が対象になるため2件出る。
    let count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Warning && *c == "factory-probability-clamped")
        .count();
    assert_eq!(count, 2);
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("factory_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("factory_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}
