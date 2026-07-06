//! `rules::check_crossing` の統合テスト。testdata/ の正常系1件・異常系6件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/tunnel_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した crossing dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_crossing(&dat, &dir)
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
fn valid_crossing_has_no_errors_or_warnings() {
    let diags = check("crossing_valid.dat");
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
        &check("crossing_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("crossing_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn identical_waytypes_is_detected() {
    // schiene_tram と tram_track は別名だが、どちらも tram_wt に解決されるため
    // 「同一waytype同士の交差」としてFATALになる（crossing_writer.cc:80-82）。
    assert!(has_error(
        &check("crossing_identical_waytypes.dat"),
        "crossing-identical-waytypes"
    ));
}

#[test]
fn missing_speed_is_detected() {
    assert!(has_error(
        &check("crossing_missing_speed.dat"),
        "crossing-missing-speed"
    ));
}

#[test]
fn missing_openimage_is_detected() {
    assert!(has_error(
        &check("crossing_missing_openimage.dat"),
        "crossing-missing-openimage"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("crossing_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("crossing_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // crossing_writer.cc:110-114のuint16へ静かにラップアラウンドする不具合。
    assert!(has(
        &check("crossing_date_index_overflow.dat"),
        Severity::Warning,
        "date-index-overflow"
    ));
}

/// 第6弾: pak128実データ
/// （`infrastructure/road_rail_crossings/p128_crossing_road040_rail080.dat`の
/// `OpenImage[NS,EW][0-1]=...<0+$1>.<2*$0+1>`）で確認された、方向名（ribi）
/// 文字列パラメータ展開のcrossingでの回帰テスト。展開が効いていなければ
/// `openimage[ns][0]`/`openimage[ew][0]`が存在せず`crossing-missing-openimage`が
/// 誤検知される。
#[test]
fn ribi_parameter_expansion_resolves_openimage_and_avoids_false_positive() {
    let diags = check("crossing_ribi_param_expansion.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "ribiパラメータ展開後は openimage[ns][0]/openimage[ew][0] が存在するはずで、\
         crossing-missing-openimage 等のerrorは出ないべき: {errors:?}"
    );
}
