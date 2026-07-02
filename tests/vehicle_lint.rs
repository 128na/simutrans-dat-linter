//! `rules::check_vehicle` の統合テスト。testdata/ の正常系1件・異常系8件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/building.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した vehicle dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_vehicle(&dat, &dir)
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
fn valid_vehicle_has_no_errors_or_warnings() {
    let diags = check("vehicle_valid.dat");
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
        &check("vehicle_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_engine_type_is_detected() {
    assert!(has(
        &check("vehicle_bad_engine_type.dat"),
        Severity::Warning,
        "unknown-engine-type"
    ));
}

#[test]
fn electrified_track_exempts_engine_type_check() {
    let diags = check("vehicle_electrified_engine_type_exempt.dat");
    assert!(
        !has(diags.as_slice(), Severity::Warning, "unknown-engine-type"),
        "waytype=electrified_track の場合、engine_typeの不正値チェックはスキップされるべき: {diags:?}"
    );
    assert!(!has_error(&diags, "missing-waytype"));
    assert!(!has_error(&diags, "unknown-waytype"));
}

#[test]
fn incomplete_8_direction_images_is_detected() {
    assert!(has_error(
        &check("vehicle_incomplete_8dir.dat"),
        "incomplete-8-direction-images"
    ));
}

#[test]
fn freightimage_count_mismatch_is_detected() {
    assert!(has_error(
        &check("vehicle_freightimage_count_mismatch.dat"),
        "freightimage-count-mismatch"
    ));
}

#[test]
fn missing_indexed_freightimage_is_detected() {
    assert!(has_error(
        &check("vehicle_missing_indexed_freightimage.dat"),
        "missing-indexed-freightimage"
    ));
}

#[test]
fn missing_freightimagetype_is_detected() {
    assert!(has_error(
        &check("vehicle_missing_freightimagetype.dat"),
        "missing-freightimagetype"
    ));
}

#[test]
fn power_gear_mismatch_is_detected() {
    assert!(has(
        &check("vehicle_power_gear_mismatch.dat"),
        Severity::Warning,
        "power-gear-mismatch"
    ));
}

#[test]
fn power_gear_boundary_is_not_a_mismatch() {
    // gear=2 -> (2*64)/100=1（非ゼロ）。整数除算の境界値でも警告が出ないことを確認する。
    let diags = check("vehicle_power_gear_boundary_ok.dat");
    assert!(!has(&diags, Severity::Warning, "power-gear-mismatch"));
}

#[test]
fn extra_freightimagetype_is_detected() {
    assert!(has(
        &check("vehicle_extra_freightimagetype.dat"),
        Severity::Warning,
        "extra-freightimagetype"
    ));
}
