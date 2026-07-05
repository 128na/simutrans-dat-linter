//! `rules::check_tunnel` の統合テスト。testdata/ の正常系2件・異常系3件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/bridge_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した tunnel dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_tunnel(&dat, &dir)
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
fn valid_tunnel_has_no_errors_or_warnings() {
    let diags = check("tunnel_valid.dat");
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
fn broad_portal_short_key_has_no_errors_or_warnings() {
    // frontimage[nl] のような短縮形（[0]省略）は number_portals=4 判定の
    // 対象かつ実際に読まれる画像キーでもある（tunnel_writer.cc:49-60,90-94）。
    let diags = check("tunnel_broad_portal_short_key.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}

#[test]
fn missing_waytype_is_detected() {
    assert!(has_error(
        &check("tunnel_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("tunnel_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("tunnel_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("tunnel_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}
