//! `rules::check_tree` の統合テスト。testdata/ の正常系2件（季節なし/4季節）・
//! 異常系4件で、期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/groundobj_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した tree dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_tree(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
}

#[test]
fn valid_tree_has_no_errors_or_warnings() {
    let diags = check("tree_valid.dat");
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
fn valid_seasonal_tree_has_no_errors_or_warnings() {
    let diags = check("tree_seasons_valid.dat");
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
fn missing_age_image_is_detected() {
    assert!(has_error(
        &check("tree_missing_age_image.dat"),
        "missing-age-season-image"
    ));
}

#[test]
fn missing_season_image_is_detected() {
    assert!(has_error(
        &check("tree_missing_season_image.dat"),
        "missing-age-season-image"
    ));
}

#[test]
fn seasons_zero_requires_no_images() {
    // seasons=0はtree_writer.cc:34の`uint8 const number_of_seasons =
    // obj.get_int("seasons", 1);`によりuint8へ切り詰められてそのまま0になる
    // （0 mod 256 = 0）。season方向のループが0..0で空になるため、画像0枚でも
    // FATALにならない。以前の実装は`.max(1)`で1にクランプしていたため、
    // image[<age>][0]が無いことを誤ってFATAL相当のerrorとして報告していた。
    let diags = check("tree_seasons_zero.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "seasons=0は画像0枚を許容するはず: {errors:?}"
    );
}

#[test]
fn seasons_overflow_wraps_to_uint8() {
    // seasons=257はuint8へ切り詰めると257 mod 256=1になる
    // （tree_writer.cc:34）。よって各ageはseason 0のみが必須（5age分で
    // 計5件のmissing-age-season-image）。以前の実装はクランプ無しで257の
    // ままだったため、5*257=1285件という大量の偽陽性を出していた。
    let diags = check("tree_seasons_overflow.dat");
    let count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Error && *c == "missing-age-season-image")
        .count();
    assert_eq!(
        count, 5,
        "seasons=257はuint8切り詰めで257 mod 256=1になるため、\
         age0..4のseason0のみ（5件）が不足として検出されるはず: {diags:?}"
    );
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("tree_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("tree_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn missing_image_file_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `tree_missing_image_file.dat`の
    // `image[4][0]=nonexistent_tree.png.0.0`は12行目（common::check_image_refに
    // 新規配線したline引数、AgeSeasonImageRuleから`dat.line_of(&key)`を渡す）。
    let dir = testdata_dir();
    let path = dir.join("tree_missing_image_file.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_tree(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::MissingImageFile)
        .expect("missing-image-fileが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 12);
    assert_eq!(loc.key, "image[4][0]");
}
