//! `rules::check_groundobj` の統合テスト。testdata/ の正常系2件（固定物/移動物）・
//! 異常系5件で、期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/way_obj_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した ground_obj dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_groundobj(&dat, &dir)
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
fn valid_groundobj_has_no_errors_or_warnings() {
    let diags = check("groundobj_valid.dat");
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
fn valid_moving_groundobj_has_no_errors_or_warnings() {
    let diags = check("groundobj_moving_valid.dat");
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

/// waytype省略はFATALにならない（groundobj固有: ignore_wtにフォールバック）ため、
/// 画像0枚（no-images info）以外にerror/warningが出ないことを確認する。
#[test]
fn missing_waytype_and_images_is_not_an_error() {
    let diags = check("groundobj_no_images.dat");
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
    assert!(has(&diags, Severity::Info, "no-images"));
    assert!(has(&diags, Severity::Info, "waytype-omitted"));
}

#[test]
fn speed_truncated_to_zero_uses_fixed_branch() {
    // speed=65536はuint16へ切り詰めると65536 mod 65536=0になり、
    // groundobj_writer.cc:39の`uint16 const speed = obj.get_int("speed", 0);`が
    // 実際には固定物分岐（speed==0）に入る。以前の実装はi64の生値（非ゼロ）で
    // 分岐を選んでいたため、誤って移動物分岐（8方向必須）に入り、image[0][0]
    // しか定義していないこのfixtureで8件の偽陽性missing-season-imageを
    // 出していた。
    let diags = check("groundobj_speed_truncates_to_fixed.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "uint16切り詰め後に0になるspeed=65536は固定物分岐に入り、image[0][0]のみで\
         足りるはず: {errors:?}"
    );
}

#[test]
fn speed_hex_prefix_selects_moving_branch_and_is_accepted() {
    // groundobj_writer.cc:39の`uint16 const speed = obj.get_int("speed", 0);`は
    // strtol相当の基数自動判定を行うため、`speed=0xA`（10進10、非ゼロ）のような
    // 16進表記も正しく解釈され、移動物分岐（8方向必須）が選ばれる。以前の実装は
    // `.parse::<i64>()`（10進数限定）を使っており、`0xA`はパース失敗して
    // `.unwrap_or(0)`によりspeed=0（固定物分岐）へ誤ってフォールバックしていた
    // （第23弾、gemini-code-assistのレビュー指摘）。固定物分岐に誤って入ると
    // `image[0][0]`が無いため`no-images`（info）が出るが、正しく移動物分岐が
    // 選ばれれば8方向の画像は全て揃っているため`no-images`は出ないはず。
    let diags = check("groundobj_speed_hex_moving.dat");
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
    assert!(
        !has(&diags, Severity::Info, "no-images"),
        "speed=0xAは非ゼロとして解釈され移動物分岐が選ばれるべきで、\
         固定物分岐のno-images infoは出ないはず: {diags:?}"
    );
}

#[test]
fn missing_season_image_is_detected() {
    assert!(has_error(
        &check("groundobj_missing_season_image.dat"),
        "missing-season-image"
    ));
}

#[test]
fn unknown_waytype_is_detected() {
    assert!(has_error(
        &check("groundobj_unknown_waytype.dat"),
        "unknown-waytype"
    ));
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("groundobj_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("groundobj_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn moving_groundobj_missing_direction_is_detected() {
    assert!(has_error(
        &check("groundobj_moving_missing_direction.dat"),
        "missing-season-image"
    ));
}

#[test]
fn unknown_waytype_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `groundobj_unknown_waytype.dat`の
    // `waytype=nonexistent_waytype`は3行目。
    let dir = testdata_dir();
    let path = dir.join("groundobj_unknown_waytype.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_groundobj(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::UnknownWaytype)
        .expect("unknown-waytypeが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 3);
    assert_eq!(loc.key, "waytype");
}
