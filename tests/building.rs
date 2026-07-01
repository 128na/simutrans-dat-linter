//! `rules::check_building` の統合テスト。testdata/ の正常系1件・異常系5件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! 元は try-out/dat_linter/README.md の手動検証表だったものをテスト化した。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した building dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_building(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
}

#[test]
fn valid_building_has_no_errors_or_warnings() {
    let diags = check("roundtrip_test.dat");
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
fn missing_cursor_and_icon_is_detected() {
    assert!(has_error(
        &check("broken_no_icon.dat"),
        "missing-cursor-icon"
    ));
}

#[test]
fn missing_tile_image_is_detected() {
    assert!(has_error(
        &check("broken_missing_tile.dat"),
        "missing-tile-image"
    ));
}

#[test]
fn obsolete_type_is_detected() {
    assert!(has_error(
        &check("broken_obsolete_type.dat"),
        "obsolete-type"
    ));
}

#[test]
fn missing_waytype_for_stop_is_detected() {
    assert!(has_error(
        &check("broken_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn image_not_multiple_of_128_is_detected() {
    assert!(has_error(
        &check("broken_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn leading_space_in_value_breaks_image_resolution() {
    // `cursor= station_icon.png.0.0` の先頭スペースは値としてトリムされないため、
    // 存在しないファイル " station_icon.png" を参照して missing-image-file になる。
    assert!(has_error(
        &check("broken_space_in_value.dat"),
        "missing-image-file"
    ));
}
