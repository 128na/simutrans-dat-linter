//! `rules::check_way` の統合テスト。testdata/ の正常系1件・異常系6件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/vehicle_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した way dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_way(&dat, &dir)
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
fn valid_way_has_no_errors_or_warnings() {
    let diags = check("way_valid.dat");
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
        &check("way_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("way_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_base_image_is_detected() {
    assert!(has_error(
        &check("way_missing_base_image.dat"),
        "missing-base-image"
    ));
}

#[test]
fn winter_image_exempts_base_image_check() {
    let diags = check("way_winter_image_exempt.dat");
    assert!(
        !has_error(&diags, "missing-base-image"),
        "image[-][0]（冬季season 0版）が定義されていれば image[-] 欠落は fatal にならない: {diags:?}"
    );
}

#[test]
fn clip_below_out_of_range_is_detected() {
    assert!(has(
        &check("way_clip_below_out_of_range.dat"),
        Severity::Warning,
        "clip-below-out-of-range"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("way_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn image_coordinate_out_of_bounds_is_detected() {
    // image[-]=station_cube.334.0: station_cube.png は128x128（1x1タイル）なので、
    // row=334はタイル数(1)を大きく超える。building.rsと同じ`common::check_image_ref`
    // 経由の共有ロジックが、way（別obj種別）でも同じcodeを出すことを確認する。
    assert!(has_error(
        &check("way_image_coordinate_out_of_bounds.dat"),
        "image-coordinate-out-of-bounds"
    ));
}

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // way_writer.cc:45-49のuint16へ静かにラップアラウンドする不具合。
    assert!(has(
        &check("way_date_index_overflow.dat"),
        Severity::Warning,
        "date-index-overflow"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // axle_load=100000はuint16の範囲(0..65535)外、system_type=300はuint8の範囲
    // (0..255)外。way_writer.cc:41,52のwrite_uint16/write_uint8へ静かに
    // 切り詰められる。
    let diags = check("way_narrow_int_overflow.dat");
    let overflow_count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Warning && *c == "narrow-int-overflow")
        .count();
    assert_eq!(
        overflow_count, 2,
        "axle_load/system_typeの2件が検出されるはず: {diags:?}"
    );
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has_error(
        &check("way_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_name_is_detected() {
    // name="ValidRoad\0Extra" は埋め込みNULバイトを含む。text_writer_t::write_obj
    // （text_writer.cc:18）はstrlen()で長さを計算するため、\0以降の"Extra"が
    // 警告無く切り詰められる。
    assert!(has(
        &check("way_embedded_nul_name.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}
