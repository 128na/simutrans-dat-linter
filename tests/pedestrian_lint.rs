//! `rules::check_pedestrian` の統合テスト。testdata/ の正常系3件（静止8方向全定義/
//! 静止一部のみ定義/アニメーション画像）・異常系3件で、期待する診断コードが出る
//! （または全く出ない）ことを確認する。`tests/citycar_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した pedestrian dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_pedestrian(&dat, &dir)
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
fn valid_pedestrian_has_no_errors_or_warnings() {
    assert_no_errors_or_warnings(&check("pedestrian_valid.dat"));
}

#[test]
fn partial_direction_images_has_no_errors_or_warnings() {
    // pedestrian_writer.cc の静止分岐の8方向走査は無条件（早期終了なし）で、
    // vehicleのような「一部方向だけ定義」を検出するfatal分岐が存在しないため、
    // 2方向だけ定義してもerror/warningは出ない
    // （詳細はrules/pedestrian.rs冒頭のREJECTEDコメント参照）。
    assert_no_errors_or_warnings(&check("pedestrian_partial_images_valid.dat"));
}

#[test]
fn animated_pedestrian_has_no_errors_or_warnings() {
    // image[<dir>][0]がいずれかの方向で非空だとis_animatedが真になり、
    // アニメーション分岐（image[<dir>][<frame>]）で読まれる。他の方向が
    // 0フレーム（image[<dir>][0]自体が空）でもfatal/warningは出ない。
    assert_no_errors_or_warnings(&check("pedestrian_animated_valid.dat"));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("pedestrian_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("pedestrian_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn animated_missing_image_file_is_detected() {
    assert!(has_error(
        &check("pedestrian_animated_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // pedestrian_writer.cc:73-79のuint16へ静かにラップアラウンドする不具合。
    assert!(has_warning(
        &check("pedestrian_date_index_overflow.dat"),
        "date-index-overflow"
    ));
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has_error(
        &check("pedestrian_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_copyright_is_detected() {
    // copyright="fuga\0bar" は埋め込みNULバイトを含む。text_writer_t::write_obj
    // （text_writer.cc:18）はstrlen()で長さを計算するため、\0以降の"bar"が
    // 警告無く切り詰められる。
    assert!(has_warning(
        &check("pedestrian_embedded_nul_copyright.dat"),
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // distributionweight=100000/offset=100000はいずれもuint16の範囲(0..65535)外。
    // pedestrian_writer.cc:23,71,83,85のwrite_uint16へ静かに切り詰められる。
    let diags = check("pedestrian_narrow_int_overflow.dat");
    let overflow_count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Warning && *c == "narrow-int-overflow")
        .count();
    assert_eq!(
        overflow_count, 2,
        "distributionweight/offsetの2件が検出されるはず: {diags:?}"
    );
}

#[test]
fn missing_image_file_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `pedestrian_missing_image_file.dat`の
    // `image[w]=nonexistent_pedestrian.png.0.0`は7行目（common::check_image_refに
    // 新規配線したline引数、check_static_imagesから`dat.line_of(&key)`を渡す）。
    let dir = testdata_dir();
    let path = dir.join("pedestrian_missing_image_file.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_pedestrian(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::MissingImageFile)
        .expect("missing-image-fileが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 7);
    assert_eq!(loc.key, "image[w]");
}
