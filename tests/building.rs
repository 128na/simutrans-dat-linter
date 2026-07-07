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
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
}

fn has(diags: &[(Severity, &str)], severity: Severity, code: &str) -> bool {
    diags.iter().any(|(s, c)| *s == severity && *c == code)
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
fn dash_sentinel_tile_image_is_not_a_false_positive() {
    // 第6弾: pak128実データ（factories/cotton_farm_w_fields.dat の
    // `BackImage[0][0][0][0][0][0]=-`）で確認された、タイル画像の"-"（画像なし
    // センチネル）が missing-image-file として誤検知されないことの回帰テスト。
    // image_writer_t::write_obj（image_writer.cc:366）は"-"を空文字列と同様
    // 「画像なし」として無条件に扱う（tile画像に限らない共通ロジック）。
    let diags = check("building_dash_sentinel_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "\"-\"センチネルのタイル画像はerrorを出さないべき: {errors:?}"
    );
}

#[test]
fn relative_path_with_double_dot_in_directory_prefix_is_not_a_false_positive() {
    // 第10弾: 実際のユーザー報告（iss/building/depot/depot.dat の
    // `icon=> ../../icon_way3.1.0`）の再現。`.dat`が`testdata/`直下から2階層
    // 下（`testdata/nested/depot/`）にあり、`cursor`/`icon`が`../../station_icon.1.0`
    // という相対パスで`testdata/station_icon.png`を参照する。
    //
    // このテストは`check()`ヘルパー（`dat_dir`を常に`testdata_dir()`に固定する
    // 簡略化）を使わず、実際の本番コード（`main.rs`の`path.parent()`）と同じ
    // `dat_dir`解決（`.dat`ファイル自身の親ディレクトリ）を使う。これにより
    // `resolve_image_filename`（src/rules/common.rs）が値全体ではなく
    // ディレクトリ接頭辞を除いた部分の中で最初の'.'を探すよう修正されたことを
    // 実際のディレクトリ構造で確認できる。
    let file_path = testdata_dir()
        .join("nested")
        .join("depot")
        .join("depot_relative_icon.dat");
    let dat_dir = file_path.parent().unwrap();
    let dat = dat_linter::parser::DatFile::parse(&file_path)
        .unwrap_or_else(|e| panic!("パースに失敗: {e}"));
    let diags: Vec<_> = rules::check_building(&dat, dat_dir)
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect();

    assert!(
        !has_error(&diags, "missing-image-file"),
        "実在する画像（../../station_icon.png）が見つからないと誤検知された: {diags:?}"
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
fn cursor_icon_not_applicable_for_res_com_ind_cur_mon_tow() {
    // 第7弾（項目5）: `builder/hausbauer.cc`を根拠に再調査した結果、
    // res/com/ind/cur/mon/towはプレイヤーが選ぶビルドメニューに
    // そもそも現れない（hausbauer_t::fill_menu()はstation_buildingリスト
    // のみを読み、これらの種別は別リストにしか登録されない）ため、
    // cursor/icon省略はerrorではなくinfoにすべき。
    // pak128実データ（cityhouses/com/com_09_18.dat: type=com、cursor/icon
    // 無し）を最小再現したフィクスチャで、missing-cursor-iconが出ず、
    // 代わりにcursor-icon-not-applicableが出ることを確認する。
    let diags = check("building_res_no_cursor_icon_valid.dat");
    assert!(
        !has_error(&diags, "missing-cursor-icon"),
        "type=resはビルドメニュー対象外なのでmissing-cursor-iconを出すべきではない: {diags:?}"
    );
    assert!(
        has(&diags, Severity::Info, "cursor-icon-not-applicable"),
        "type=resではcursor-icon-not-applicable(info)を出すべき: {diags:?}"
    );
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
fn image_coordinate_out_of_bounds_is_detected() {
    // cursor=station_icon.334.0: station_icon.png は128x128（1x1タイル）なので、
    // row=334はタイル数(1)を大きく超える。image_writer.cc:419-422の
    // "invalid image number in ..." FATAL ERRORに対応する。
    assert!(has_error(
        &check("broken_image_coordinate_out_of_bounds.dat"),
        "image-coordinate-out-of-bounds"
    ));
}

#[test]
fn date_index_overflow_is_detected() {
    // intro_year=-1900/intro_month=13 -> -1900*12+13-1=-22788（範囲外）。
    // retire_year=12999/retire_month=0 -> 12999*12+0-1=155987（範囲外）。
    // どちらもbuilding_writer.cc:227-232のuint16へ静かにラップアラウンドする不具合
    // （refs/demo/station_cube.datの意図的な不正値と同じシナリオ）。
    assert!(has(
        &check("broken_date_index_overflow.dat"),
        Severity::Warning,
        "date-index-overflow"
    ));
}

#[test]
fn boolean_style_field_not_zero_or_one_is_detected() {
    // NoInfo=999/enables_pax=999は0/1以外の値だが、building_writer.ccは
    // `obj.get_int(key, 0) > 0`で判定するため機能的には1と同じ動作をする
    // （スタイルノート、warning）。
    assert!(has(
        &check("broken_boolean_style_field.dat"),
        Severity::Warning,
        "boolean-style-field-not-zero-or-one"
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

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has_error(
        &check("building_name_forbidden_filename_character.dat"),
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_name_is_detected() {
    // name="Passagiere\0Extra" は埋め込みNULバイトを含む。
    // text_writer_t::write_obj（text_writer.cc:18）はstrlen()で長さを計算するため、
    // \0以降の"Extra"が警告無く切り詰められる。
    assert!(has(
        &check("building_embedded_nul_name.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn capacity_narrow_int_overflow_is_detected() {
    // capacity=100000はuint16の範囲(0..65535)外。building_writer.cc:244,365の
    // sint32ローカル変数から`node.write_uint16(fp, capacity)`へ静かに
    // 切り詰められる（sint32→uint16という符号・幅の両方が変わるnarrowing）。
    assert!(has(
        &check("building_capacity_narrow_int_overflow.dat"),
        Severity::Warning,
        "narrow-int-overflow"
    ));
}
