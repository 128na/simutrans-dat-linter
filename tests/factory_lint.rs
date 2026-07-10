//! `rules::check_factory` の統合テスト。testdata/ の正常系1件・異常系各種で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/pedestrian_lint.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した factory dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_factory(&dat, &dir)
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

fn has(diags: &[(Severity, &str)], severity: Severity, code: &str) -> bool {
    diags.iter().any(|(s, c)| *s == severity && *c == code)
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
fn valid_factory_has_no_errors_or_warnings() {
    assert_no_errors_or_warnings(&check("factory_valid.dat"));
}

#[test]
fn missing_mapcolor_is_detected() {
    assert!(has_error(
        &check("factory_missing_mapcolor.dat"),
        "factory-missing-mapcolor"
    ));
}

#[test]
fn type_override_is_detected() {
    assert!(has_error(
        &check("factory_type_override.dat"),
        "factory-type-override"
    ));
}

#[test]
fn zero_dims_is_detected() {
    assert!(has_error(&check("factory_zero_dims.dat"), "zero-size"));
}

#[test]
fn cursor_icon_not_applicable_for_factory() {
    // 第7弾（項目5）: `builder/hausbauer.cc`のsuccessfully_loaded()は
    // `case building_desc_t::factory: break;`でfactoryをどのリストにも登録せず、
    // プレイヤーが選ぶビルドメニュー（fill_menu()がstation_buildingのみを読む）
    // には現れない。配置は`builder/fabrikbauer.cc`（cursorへの言及が一切無い、
    // 別モジュール）が行うため、missing-cursor-iconをerrorとするのは誤りだった。
    // このfixtureは元々「missing-cursor-iconが正しく検出される」ことを確認する
    // 回帰テストだったが、再調査の結果、cursor/icon省略はerrorではなくinfoの
    // cursor-icon-not-applicableになるべきと判明したため期待値を修正した。
    let diags = check("factory_missing_cursor_icon.dat");
    assert!(
        !has_error(&diags, "missing-cursor-icon"),
        "factoryはビルドメニュー対象外なのでmissing-cursor-iconを出すべきではない: {diags:?}"
    );
    assert!(
        has(&diags, Severity::Info, "cursor-icon-not-applicable"),
        "factoryではcursor-icon-not-applicable(info)を出すべき: {diags:?}"
    );
}

#[test]
fn missing_tile_image_is_detected() {
    assert!(has_error(
        &check("factory_missing_tile_image.dat"),
        "missing-tile-image"
    ));
}

#[test]
fn dash_sentinel_tile_image_is_not_a_false_positive() {
    // 第6弾: pak128実データ（factories/cotton_farm_w_fields.dat の
    // `BackImage[0][0][0][0][0][0]=-`。"the building has three tiles and one
    // empty"というコメント通り、複数タイルのfactoryで意図的に1タイルだけ
    // 空にする実例）で確認された、"-"（画像なしセンチネル）の
    // missing-image-file誤検知が解消されていることの回帰テスト。
    assert_no_errors_or_warnings(&check("factory_dash_sentinel_valid.dat"));
}

#[test]
fn output_capacity_too_small_is_detected() {
    assert!(has_warning(
        &check("factory_output_capacity_too_small.dat"),
        "factory-output-capacity-too-small"
    ));
}

#[test]
fn smoketile_without_offset_is_detected() {
    assert!(has_warning(
        &check("factory_smoketile_without_offset.dat"),
        "factory-smoketile-without-offset"
    ));
}

#[test]
fn probability_clamped_is_detected() {
    let diags = check("factory_probability_clamped.dat");
    assert!(has_warning(&diags, "factory-probability-clamped"));
    // probability_to_spawn と expand_probability の両方が対象になるため2件出る。
    let count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Warning && *c == "factory-probability-clamped")
        .count();
    assert_eq!(count, 2);
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("factory_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("factory_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn productivity_zero_is_detected() {
    assert!(has_error(
        &check("factory_productivity_zero.dat"),
        "factory-productivity-zero"
    ));
}

#[test]
fn type_override_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `factory_type_override.dat`の`type=res`は
    // 5行目（`.at()`が新規に配線された「値は存在するが不正」パターン）。
    let dir = testdata_dir();
    let path = dir.join("factory_type_override.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_factory(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::FactoryTypeOverride)
        .expect("factory-type-overrideが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 5);
    assert_eq!(loc.key, "type");
}

#[test]
fn output_capacity_too_small_diagnostic_has_correct_line_number() {
    // 第2弾: `factory_output_capacity_too_small.dat`の`outputcapacity[0]=5`は9行目。
    let dir = testdata_dir();
    let path = dir.join("factory_output_capacity_too_small.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_factory(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::FactoryOutputCapacityTooSmall)
        .expect("factory-output-capacity-too-smallが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 9);
    assert_eq!(loc.key, "outputcapacity[0]");
}
