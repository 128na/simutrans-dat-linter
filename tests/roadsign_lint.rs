//! `rules::check_roadsign` の統合テスト。testdata/ の正常系複数件・異常系複数件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/tunnel_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した roadsign dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_roadsign(&dat, &dir)
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

fn assert_no_errors(diags: &[(Severity, &str)]) {
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}

#[test]
fn valid_roadsign_has_no_errors() {
    assert_no_errors(&check("roadsign_valid.dat"));
}

#[test]
fn two_state_2d_roadsign_has_no_errors() {
    // state 0 とstate 1 の両方を全方向定義した場合、途中で打ち切られず
    // 全キーが検証される（roadsign_writer.cc:42-58のstateループ）。
    assert_no_errors(&check("roadsign_2d_single_state_valid.dat"));
}

#[test]
fn private_road_sign_valid_has_no_errors() {
    // is_private=1 のときは dir_cnt=2（ns/ew）、threshold=1のため
    // state 0 と state 1 の両方が必須（roadsign_writer.cc:27-30,48）。
    assert_no_errors(&check("roadsign_private_road_valid.dat"));
}

#[test]
fn private_road_sign_missing_state1_is_detected() {
    // 私有地標識はstate 1（image[ns][1]/image[ew][1]）も必須。
    // state=1, idx=0で空 -> state>threshold(1)は 1>1=false のため打ち切られず fatal。
    assert!(has_error(
        &check("roadsign_private_road_missing_state1.dat"),
        "roadsign-image-missing"
    ));
}

#[test]
fn traffic_light_valid_has_no_errors() {
    // image[ne][0]が非空だと8方向のtraffic light扱いになる
    // （roadsign_writer.cc:31-35）。
    assert_no_errors(&check("roadsign_traffic_light_valid.dat"));
}

#[test]
fn numbered_syntax_valid_has_no_errors() {
    // image[0]が非空だとnumbered構文が使われる（roadsign_writer.cc:139-148）。
    // 4枚ちょうどで打ち切られるのはfatalにならない。
    assert_no_errors(&check("roadsign_numbered_valid.dat"));
}

#[test]
fn missing_waytype_is_detected() {
    assert!(has_error(
        &check("roadsign_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("roadsign_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("roadsign_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("roadsign_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn two_d_image_missing_in_middle_is_detected() {
    // state=0で idx=1(s)が空 -> idx!=0 のため無条件でfatal
    // （roadsign_writer.cc:47-54の"image in the middle is missing"分岐）。
    assert!(has_error(
        &check("roadsign_2d_image_missing.dat"),
        "roadsign-image-missing"
    ));
}

#[test]
fn numbered_count_not_multiple_of_4_is_detected() {
    assert!(has_error(
        &check("roadsign_numbered_count_not_multiple_of_4.dat"),
        "roadsign-image-count-not-multiple-of-4"
    ));
}

#[test]
fn numbered_missing_image_file_is_detected() {
    assert!(has_error(
        &check("roadsign_numbered_missing_image_file.dat"),
        "missing-image-file"
    ));
}
