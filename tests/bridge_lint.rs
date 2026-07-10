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

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has_error(
        &check("bridge_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_name_is_detected() {
    // name="ValidBridge\0Extra" は埋め込みNULバイトを含む。text_writer_t::write_obj
    // （text_writer.cc:18）はstrlen()で長さを計算するため、\0以降の"Extra"が
    // 警告無く切り詰められる。
    assert!(has(
        &check("bridge_embedded_nul_name.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // topspeed=100000はuint16の範囲(0..65535)外。bridge_writer.cc:102,120の
    // write_uint16へ静かに切り詰められる（get_intの無条件フォールバックで
    // get_int_clampedではないため、ClampedRangeRuleではなくこちらの対象）。
    assert!(has(
        &check("bridge_narrow_int_overflow.dat"),
        Severity::Warning,
        "narrow-int-overflow"
    ));
}

#[test]
fn clamped_value_out_of_range_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `bridge_clamped_out_of_range.dat`の
    // `pillar_asymmetric=5`は5行目（common::check_clamped_int_fieldに新規配線）。
    let dir = testdata_dir();
    let path = dir.join("bridge_clamped_out_of_range.dat");
    let dat = dat_linter::parser::DatFile::parse(&path).expect("パースに失敗");
    let diags = dat_linter::rules::check_bridge(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::ClampedValueOutOfRange)
        .expect("clamped-value-out-of-rangeが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 5);
    assert_eq!(loc.key, "pillar_asymmetric");
}

#[test]
fn no_bridge_image_specified_diagnostic_has_correct_line_number() {
    // 第2弾: `bridge_missing_front_image.dat`はfrontimage系24キーが全て未指定
    // （キー自体が無い）だが、値が"2文字以下"のケースには本来値が存在する
    // ケースも含まれるため、`.at()`は`line_of`が`Some`を返す場合のみ付与される
    // （キー自体が無いこのfixtureでは`location`が`None`のままであるべき）。
    let dir = testdata_dir();
    let path = dir.join("bridge_missing_front_image.dat");
    let dat = dat_linter::parser::DatFile::parse(&path).expect("パースに失敗");
    let diags = dat_linter::rules::check_bridge(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::NoBridgeImageSpecified)
        .expect("no-bridge-image-specifiedが検出されるべき");
    assert!(
        d.location.is_none(),
        "キー自体が欠落している場合はlocationがNoneのままであるべき: {d:?}"
    );
}
