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

fn has_warning(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Warning && *c == code)
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

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // citycar_writer.cc:21-27のuint16へ静かにラップアラウンドする不具合。
    assert!(has_warning(
        &check("citycar_date_index_overflow.dat"),
        "date-index-overflow"
    ));
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has_error(
        &check("citycar_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_copyright_is_detected() {
    // copyright="fuga\0bar" は埋め込みNULバイトを含む。text_writer_t::write_obj
    // （text_writer.cc:18）はstrlen()で長さを計算するため、\0以降の"bar"が
    // 警告無く切り詰められる。
    assert!(has_warning(
        &check("citycar_embedded_nul_copyright.dat"),
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // distributionweight=100000はuint16の範囲(0..65535)外。citycar_writer.cc:19,35の
    // write_uint16へ静かに切り詰められる。
    assert!(has_warning(
        &check("citycar_narrow_int_overflow.dat"),
        "narrow-int-overflow"
    ));
}

#[test]
fn missing_image_file_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `citycar_missing_image_file.dat`の
    // `image[w]=nonexistent_citycar.png.0.0`は8行目（common::check_image_refに
    // 新規配線したline引数、DirectionImageRefRuleから`dat.line_of(&key)`を渡す）。
    let dir = testdata_dir();
    let path = dir.join("citycar_missing_image_file.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_citycar(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::MissingImageFile)
        .expect("missing-image-fileが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 8);
    assert_eq!(loc.key, "image[w]");
}
