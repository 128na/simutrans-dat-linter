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
        .map(|d| (d.severity, d.code))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
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
