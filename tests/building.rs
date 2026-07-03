//! `rules::check_building` の統合テスト。testdata/ の正常系1件・異常系5件で、
//! 期待する診断コードが出る（または全く出ない）ことを確認する。
//! 元は try-out/dat_linter/README.md の手動検証表だったものをテスト化した。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した building dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_building(&dat, &dir)
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
fn valid_building_has_no_errors_or_warnings() {
    let diags = check("roundtrip_test.dat");
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
fn image_ref_without_literal_png_resolves_correctly() {
    // 実際に配布されているpak128.japan系アドオン（例:
    // refs/building.JpClassicTerminal/JpClassicTerminal.dat の
    // `icon=> JpClassicTerminal.4.0`）は画像参照に".png"を含めない。
    // image_writer_t::write_obj（image_writer.cc:372-388）は最初の'.'より前を
    // 幹として取り出し無条件で".png"を付与するため、"station_icon.4.0"は
    // "station_icon.png.0.0"と全く同じ"station_icon.png"を指す。以前の実装は
    // ".png"が参照文字列に literal に含まれていることを前提にしており、
    // このケースで存在するファイルを「見つからない」と誤検知していた。
    let diags = check("building_ref_without_png_valid.dat");
    assert!(
        !has_error(&diags, "missing-image-file"),
        "実在する画像なのに missing-image-file が出た: {diags:?}"
    );
}

#[test]
fn missing_cursor_and_icon_is_detected() {
    assert!(has_error(
        &check("broken_no_icon.dat"),
        "missing-cursor-icon"
    ));
}

#[test]
fn missing_tile_image_is_detected() {
    assert!(has_error(
        &check("broken_missing_tile.dat"),
        "missing-tile-image"
    ));
}

#[test]
fn obsolete_type_is_detected() {
    assert!(has_error(
        &check("broken_obsolete_type.dat"),
        "obsolete-type"
    ));
}

#[test]
fn missing_waytype_for_stop_is_detected() {
    assert!(has_error(
        &check("broken_missing_waytype.dat"),
        "missing-waytype"
    ));
}

#[test]
fn image_not_multiple_of_128_is_detected() {
    assert!(has_error(
        &check("broken_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

#[test]
fn leading_space_in_value_is_trimmed_and_not_an_error() {
    // `cursor= station_icon.png.0.0` の先頭スペースは、`DatFile`のパース時点では
    // トリムされない（parser.rsのdocコメントどおり値は一切トリムしない）が、
    // makeobj側の実際の解決経路（cursorskin_writer_t::write_obj経由の
    // `image_writer_t::write_obj`、image_writer.cc:364）は`'>'`の有無に関わらず
    // 無条件で`trim(an_imagekey)`を呼ぶため、makeobj自体はこの先頭スペースを
    // 無視して正しく"station_icon.png"を解決する（menuマイルストーンでの
    // image_writer.cc再調査により、以前このテストが前提としていた「先頭スペースは
    // トリムされない」という認識が誤りだったと判明したため訂正した。
    // common::check_image_refのstrip_zoomable_prefix_and_trim参照）。
    let errors: Vec<_> = check("broken_space_in_value.dat")
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}
