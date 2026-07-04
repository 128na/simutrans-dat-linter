//! `rules::check_way_obj` の統合テスト。testdata/ の正常系1件・異常系6件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/crossing_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した way-object dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_way_obj(&dat, &dir)
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
fn valid_way_obj_has_no_errors_or_warnings() {
    let diags = check("way_obj_valid.dat");
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
fn dash_sentinel_in_backdiagonal_is_not_a_false_positive() {
    // 第8弾: ユーザー報告された実際の誤検知
    // (`iss/way-object/road/wall_1.dat`の`backdiagonal[nw]=-`が
    // `missing-image-file: Referenced image - was not found`として誤検知されていた)
    // の最小再現。RIBI_CODES[9]="nw"（DIAGONAL_RIBI_INDICESの一つ）。
    // image_writer_t::write_obj（image_writer.cc:366）は"-"をfrontdiagonal/
    // backdiagonalに限らず全ての画像キーで共通して「画像なし」として扱うため、
    // check_image_ref（src/rules/common.rs）側で一元的に判定するよう修正した。
    let diags = check("way_obj_dash_sentinel_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "\"-\"センチネルのbackdiagonalはerrorを出さないべき: {errors:?}"
    );
}

#[test]
fn missing_waytype_is_detected() {
    assert!(has_error(
        &check("way_obj_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("way_obj_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_own_waytype_is_detected() {
    assert!(has_error(
        &check("way_obj_missing_own_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_own_waytype_is_detected() {
    assert!(has_error(
        &check("way_obj_unknown_own_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("way_obj_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("way_obj_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}
