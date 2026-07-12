//! `rules::check_vehicle` の統合テスト。testdata/ の正常系1件・異常系8件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! `tests/building.rs`と同じ形式に揃えている。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した vehicle dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_vehicle(&dat, &dir)
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
fn valid_vehicle_has_no_errors_or_warnings() {
    let diags = check("vehicle_valid.dat");
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
        &check("vehicle_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn unknown_engine_type_is_detected() {
    assert!(has(
        &check("vehicle_bad_engine_type.dat"),
        Severity::Warning,
        "unknown-engine-type"
    ));
}

#[test]
fn electrified_track_exempts_engine_type_check() {
    let diags = check("vehicle_electrified_engine_type_exempt.dat");
    assert!(
        !has(diags.as_slice(), Severity::Warning, "unknown-engine-type"),
        "waytype=electrified_track の場合、engine_typeの不正値チェックはスキップされるべき: {diags:?}"
    );
    assert!(!has_error(&diags, "missing-waytype"));
    assert!(!has_error(&diags, "unknown-waytype"));
}

#[test]
fn incomplete_8_direction_images_is_detected() {
    assert!(has_error(
        &check("vehicle_incomplete_8dir.dat"),
        "incomplete-8-direction-images"
    ));
}

#[test]
fn freightimage_count_mismatch_is_detected() {
    assert!(has_error(
        &check("vehicle_freightimage_count_mismatch.dat"),
        "freightimage-count-mismatch"
    ));
}

#[test]
fn missing_indexed_freightimage_is_detected() {
    assert!(has_error(
        &check("vehicle_missing_indexed_freightimage.dat"),
        "missing-indexed-freightimage"
    ));
}

#[test]
fn missing_freightimagetype_is_detected() {
    assert!(has_error(
        &check("vehicle_missing_freightimagetype.dat"),
        "missing-freightimagetype"
    ));
}

#[test]
fn power_gear_mismatch_is_detected() {
    assert!(has(
        &check("vehicle_power_gear_mismatch.dat"),
        Severity::Warning,
        "power-gear-mismatch"
    ));
}

#[test]
fn power_gear_boundary_is_not_a_mismatch() {
    // gear=2 -> (2*64)/100=1（非ゼロ）。整数除算の境界値でも警告が出ないことを確認する。
    let diags = check("vehicle_power_gear_boundary_ok.dat");
    assert!(!has(&diags, Severity::Warning, "power-gear-mismatch"));
}

#[test]
fn extra_freightimagetype_is_detected() {
    assert!(has(
        &check("vehicle_extra_freightimagetype.dat"),
        Severity::Warning,
        "extra-freightimagetype"
    ));
}

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900 -> -1900*12+1-1=-22800（範囲外）。
    // retire_year=12999 -> 12999*12+1-1=155988（範囲外）。両方とも
    // vehicle_writer.cc:134,138のuint16へ静かにラップアラウンドする不具合。
    assert!(has(
        &check("vehicle_date_index_overflow.dat"),
        Severity::Warning,
        "date-index-overflow"
    ));
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=Bad:Loco はWindowsのファイル名で使用できない':'を含む。
    // root_writer_t::write()のseparate出力・uncopy()がこの値をそのまま
    // fopen()するため、ビルド/分割が失敗する（src/rules/common.rs参照）。
    assert!(has_error(
        &check("vehicle_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_copyright_is_detected() {
    // copyright="fuga\0bar" は埋め込みNULバイトを含む。
    // text_writer_t::write_obj（text_writer.cc:18）はstrlen()で長さを計算するため、
    // \0以降の"bar"が警告無く切り詰められる。
    assert!(has(
        &check("vehicle_embedded_nul_copyright.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // speed=100000/payload=100000/axle_load=100000はuint16の範囲(0..65535)外、
    // length=300はuint8の範囲(0..255)外。vehicle_writer.cc:98,106,115,166の
    // write_uint16/write_uint8へ静かに切り詰められる。
    let diags = check("vehicle_narrow_int_overflow.dat");
    let overflow_count = diags
        .iter()
        .filter(|(s, c)| *s == Severity::Warning && *c == "narrow-int-overflow")
        .count();
    assert_eq!(
        overflow_count, 4,
        "speed/payload/axle_load/lengthの4件が検出されるはず: {diags:?}"
    );
}

#[test]
fn gear_parse_failure_is_detected_and_not_treated_as_default() {
    // gear=abc は数値として解釈できない。以前の実装はこれを「未指定」
    // （default=100）として扱ってしまい、実際のmakeobj（strtolが0を返す）が
    // gear=(0*64)/100=0になり power-gear-mismatch 相当の欠陥を引き起こす
    // ケースを見逃していた。gear-parse-failure と power-gear-mismatch の
    // 両方が検出されるべき。
    let diags = check("vehicle_gear_parse_failure.dat");
    assert!(
        has(&diags, Severity::Warning, "gear-parse-failure"),
        "gear-parse-failure が検出されるべき: {diags:?}"
    );
    assert!(
        has(&diags, Severity::Warning, "power-gear-mismatch"),
        "gear=abc は実際には0扱いになりpower-gear-mismatchも引き起こすはず: {diags:?}"
    );
}

#[test]
fn gear_narrow_int_overflow_is_detected() {
    // gear=200000 -> gear*64/100=128000。uint16の範囲(0..65535)外。
    // vehicle_writer.cc:142のuint16へ切り詰め代入される不具合。
    // power=0のためpower-gear-mismatchは対象外（gear単体のoverflowのみ検出）。
    let diags = check("vehicle_gear_narrow_int_overflow.dat");
    assert!(
        has(&diags, Severity::Warning, "narrow-int-overflow"),
        "narrow-int-overflow が検出されるべき: {diags:?}"
    );
    assert!(
        !has(&diags, Severity::Warning, "power-gear-mismatch"),
        "power=0 のため power-gear-mismatch は対象外のはず: {diags:?}"
    );
    assert!(
        !has(&diags, Severity::Warning, "gear-parse-failure"),
        "gear=200000 は数値として解釈できるためgear-parse-failureは出ないはず: {diags:?}"
    );
}

#[test]
fn power_narrow_int_overflow_is_detected() {
    // power=-1 はvehicle_writer.cc:119の`const uint32 power = obj.get_int(...)`
    // へ代入される際に負数がuint32の巨大な正の値へ2の補数で切り詰められる不具合。
    let diags = check("vehicle_power_narrow_int_overflow.dat");
    assert!(
        has(&diags, Severity::Warning, "narrow-int-overflow"),
        "narrow-int-overflow が検出されるべき: {diags:?}"
    );
    assert!(
        !has(&diags, Severity::Warning, "power-gear-mismatch"),
        "power<=0 のため power-gear-mismatch は対象外のはず: {diags:?}"
    );
}

#[test]
fn gear_hex_prefix_is_accepted_and_not_a_parse_failure() {
    // vehicle_writer.cc:142の`obj.get_int("gear", 100)`はstrtol相当の基数自動
    // 判定を行うため、`gear=0x64`（10進100）のような16進表記も正しく解釈される。
    // 以前の実装は`.parse::<i64>()`（10進数限定）を使っており、`0x64`のパースに
    // 失敗すると`resolve_gear`が「パース失敗」（raw=0）と誤判定し、
    // gear-parse-failureとpower-gear-mismatchの両方を偽陽性で報告していた
    // （第23弾、gemini-code-assistのレビュー指摘）。
    let diags = check("vehicle_gear_hex.dat");
    assert!(
        !has(&diags, Severity::Warning, "gear-parse-failure"),
        "gear=0x64は正しく100として解釈されるべきでgear-parse-failureは\
         出ないはず: {diags:?}"
    );
    assert!(
        !has(&diags, Severity::Warning, "power-gear-mismatch"),
        "gear=0x64はtruncated=64(非ゼロ)になるためpower-gear-mismatchは\
         出ないはず: {diags:?}"
    );
}

#[test]
fn power_hex_prefix_is_accepted_and_mismatch_is_still_detected() {
    // `PowerGearMismatchRule`内の`power`直接パース（vehicle_writer.cc:119-120の
    // `const uint32 power = obj.get_int("power", 0);`相当）もstrtol基数自動判定を
    // 再現する必要がある。以前の実装は`.parse::<i64>()`のみで、`power=0x3E8`の
    // ような16進表記はパース失敗し`else { return diags; }`で早期returnして
    // いたため、gear=0による実効出力ゼロ問題（power-gear-mismatch）を検出できず
    // 見逃していた（偽陰性、第23弾、gemini-code-assistのレビュー指摘）。
    let diags = check("vehicle_power_hex.dat");
    assert!(
        has(&diags, Severity::Warning, "power-gear-mismatch"),
        "power=0x3E8(1000)は正しく解釈され、gear=0(truncated=0)との組み合わせで\
         power-gear-mismatchが検出されるべき: {diags:?}"
    );
}

#[test]
fn unknown_engine_type_diagnostic_has_correct_line_number() {
    // 第2弾（行番号付与の機械的配線）: `vehicle_bad_engine_type.dat`の
    // `engine_type=elctric`は4行目。
    let dir = testdata_dir();
    let path = dir.join("vehicle_bad_engine_type.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_vehicle(&dat, &dir);
    let d = diags
        .iter()
        .find(|d| d.code == dat_linter::codes::DiagnosticCode::UnknownEngineType)
        .expect("unknown-engine-typeが検出されるべき");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 4);
    assert_eq!(loc.key, "engine_type");
}
