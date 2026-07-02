//! `rules::check_ground` の統合テスト。testdata/ の正常系2件（通常画像/`"-"`
//! センチネル混在）・異常系2件で、期待する診断コードが出る（または全く出ない）
//! ことを確認する。`tests/groundobj_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した ground dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_ground(&dat, &dir)
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
fn valid_ground_has_no_errors_or_warnings() {
    let diags = check("ground_valid.dat");
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

/// `"-"`（画像なしセンチネル）は空文字列と異なりphase走査を止めない
/// （ground_writer.cc:32-34の`str.empty()`判定はキー欠落のみを指す）。
/// `"-"`自体はcheck_image_refの対象外（ファイル参照ではないため）。
#[test]
fn dash_sentinel_is_not_an_error() {
    let diags = check("ground_dash_sentinel_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}

/// 画像キーが一切無い（image[0][0]すら未指定）ケースはmakeobj時点でFATALに
/// ならない（ground_writer.cc:38-40、slope走査がslope=0で即終了するだけ）ため、
/// 診断は何も出ない。
#[test]
fn no_images_is_not_an_error() {
    let diags = check("ground_no_images.dat");
    assert!(diags.is_empty(), "予期しない診断: {diags:?}");
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("ground_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("ground_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}
