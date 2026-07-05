//! `rules::check_groundobj` の統合テスト。testdata/ の正常系2件（固定物/移動物）・
//! 異常系5件で、期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/way_obj_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した ground_obj dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_groundobj(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has(diags: &[(Severity, &str)], severity: Severity, code: &str) -> bool {
    diags.iter().any(|(s, c)| *s == severity && *c == code)
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    has(diags, Severity::Error, code)
}

#[test]
fn valid_groundobj_has_no_errors_or_warnings() {
    let diags = check("groundobj_valid.dat");
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
fn valid_moving_groundobj_has_no_errors_or_warnings() {
    let diags = check("groundobj_moving_valid.dat");
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

/// waytype省略はFATALにならない（groundobj固有: ignore_wtにフォールバック）ため、
/// 画像0枚（no-images info）以外にerror/warningが出ないことを確認する。
#[test]
fn missing_waytype_and_images_is_not_an_error() {
    let diags = check("groundobj_no_images.dat");
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
    assert!(has(&diags, Severity::Info, "no-images"));
    assert!(has(&diags, Severity::Info, "waytype-omitted"));
}

#[test]
fn missing_season_image_is_detected() {
    assert!(has_error(
        &check("groundobj_missing_season_image.dat"),
        "missing-season-image"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("groundobj_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("groundobj_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("groundobj_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn moving_groundobj_missing_direction_is_detected() {
    assert!(has_error(
        &check("groundobj_moving_missing_direction.dat"),
        "missing-season-image"
    ));
}
