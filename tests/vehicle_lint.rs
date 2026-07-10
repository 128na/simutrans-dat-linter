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

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // vehicle_writer.cc:134,138のuint16へ静かにラップアラウンドする不具合。
    assert!(has(
        &check("vehicle_date_index_overflow.dat"),
        Severity::Warning,
        "date-index-overflow"
    ));
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=Bad:Loco はWindowsのファイル名で使用できない':'を含む。
    // root_writer_t::write()のseparate出力・uncopy()がこの値をそのまま
    // fopen()するため、ビルド/分割が失敗する（src/rules/common.rs参照）。
    assert!(has_error(
        &check("vehicle_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_copyright_is_detected() {
    // copyright="fuga\0bar" は埋め込みNULバイトを含む。
    // text_writer_t::write_obj（text_writer.cc:18）はstrlen()で長さを計算するため、
    // \0以降の"bar"が警告無く切り詰められる。
    assert!(has(
        &check("vehicle_embedded_nul_copyright.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // speed=100000/payload=100000/axle_load=100000はuint16の範囲(0..65535)外、
    // length=300はuint8の範囲(0..255)外。vehicle_writer.cc:98,106,115,166の
    // write_uint16/write_uint8へ静かに切り詰められる。
    let diags = check("vehicle_narrow_int_overflow.dat");
    let overflow_count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Warning && *c == "narrow-int-overflow")
        .count();
    assert_eq!(
        overflow_count, 4,
        "speed/payload/axle_load/lengthの4件が検出されるはず: {diags:?}"
    );
}

#[test]
fn unknown_engine_type_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `vehicle_bad_engine_type.dat`の
    // `engine_type=elctric`は4行目。
    let dir = testdata_dir();
    let path = dir.join("vehicle_bad_engine_type.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_vehicle(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::UnknownEngineType)
        .expect("unknown-engine-typeが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 4);
    assert_eq!(loc.key, "engine_type");
}
