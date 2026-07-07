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

#[test]
fn image_coordinate_out_of_bounds_is_detected() {
    // frontimage[n][0]=station_cube.334.0: station_cube.png は128x128
    // （1x1タイル）なので、row=334はタイル数(1)を大きく超える。building/wayと
    // 同じ`common::check_image_ref`経由の共有ロジックが、tunnel（3種類目のobj種別）
    // でも同じcodeを出すことを確認する。
    assert!(has_error(
        &check("tunnel_image_coordinate_out_of_bounds.dat"),
        "image-coordinate-out-of-bounds"
    ));
}

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // tunnel_writer.cc:29-33のuint16へ静かにラップアラウンドする不具合。
    assert!(has(
        &check("tunnel_date_index_overflow.dat"),
        Severity::Warning,
        "date-index-overflow"
    ));
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has_error(
        &check("tunnel_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_name_is_detected() {
    // name="ValidTunnel\0Extra" は埋め込みNULバイトを含む。text_writer_t::write_obj
    // （text_writer.cc:18）はstrlen()で長さを計算するため、\0以降の"Extra"が
    // 警告無く切り詰められる。
    assert!(has(
        &check("tunnel_embedded_nul_name.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // axle_load=100000はuint16の範囲(0..65535)外。tunnel_writer.cc:26のローカル
    // 変数がuint16であるため、obj.get_int()の戻り値（実質int）を代入する時点で
    // 静かに切り詰められる。
    assert!(has(
        &check("tunnel_narrow_int_overflow.dat"),
        Severity::Warning,
        "narrow-int-overflow"
    ));
}
