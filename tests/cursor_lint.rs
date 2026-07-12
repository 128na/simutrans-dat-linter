//! `rules::check_cursor` の統合テスト。testdata/ の正常系3件（通常画像/`"-"`
//! センチネル混在/`"> "`ズーム不可プレフィックス）・異常系2件で、期待する診断コードが
//! 出る（または全く出ない）ことを確認する。`tests/menu_lint.rs`と同じ形式に
//! 揃えている（`cursorskin_writer_t`は`menuskin_writer_t`と挙動上完全に同一のため）。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した cursor dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_cursor(&dat, &dir)
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
fn valid_cursor_has_no_errors_or_warnings() {
    let diags = check("cursor_valid.dat");
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

/// `"-"`（画像なしセンチネル）は空文字列と異なり走査を止めない
/// （skin_writer.cc:28-30の`str.empty()`判定はキー欠落のみを指す）。
/// `"-"`自体はcheck_image_refの対象外（ファイル参照ではないため）。
#[test]
fn dash_sentinel_is_not_an_error() {
    let diags = check("cursor_dash_sentinel_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}

/// `AllImagesRule`から`value != "-"`という事前ガードを撤去した
/// （`common.rs`のdocコメント参照）ことで、`image[0]=-`が`check_image_ref`まで
/// 到達し、`image-ref-empty-sentinel`（info）が正しく出るようになったことを
/// 確認する回帰テスト。
#[test]
fn dash_sentinel_produces_image_ref_empty_sentinel_info() {
    assert!(has(
        &check("cursor_dash_sentinel_valid.dat"),
        Severity::Info,
        "image-ref-empty-sentinel"
    ));
}

/// 画像キーが一切無い（image[0]すら未指定）ケースはmakeobj時点でFATALにならない
/// （skin_writer.cc:22-30、走査がi=0で即終了するだけ）ため、診断は何も出ない。
#[test]
fn no_images_is_not_an_error() {
    let diags = check("cursor_no_images.dat");
    assert!(diags.is_empty(), "予期しない診断: {diags:?}");
}

#[test]
fn missing_image_file_is_detected() {
    assert!(has_error(
        &check("cursor_missing_image_file.dat"),
        "missing-image-file"
    ));
}

#[test]
fn bad_image_size_is_detected() {
    assert!(has_error(
        &check("cursor_bad_image_size.dat"),
        "image-size-not-multiple-of-128"
    ));
}

/// `image_writer_t::write_obj`の`"> "`（ズーム不可フラグ）構文
/// （image_writer.cc:356-364、実例: simutrans/pak128:base/misc_GUI/mouse.dat の
/// `Image[0]=> mouse.1.0`）は先頭の`'>'`を剥がして解決すべきであり、
/// `"> station_cube.png.0.0"`をそのままファイル名として探して「見つからない」と
/// 誤検知してはならない（`common::check_image_ref`の`strip_zoomable_prefix`で対応、
/// menuマイルストーンで既に実装済みのロジックがcursorにもそのまま適用される）。
#[test]
fn zoomable_prefix_is_not_a_false_positive() {
    let diags = check("cursor_zoomable_prefix_valid.dat");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}
