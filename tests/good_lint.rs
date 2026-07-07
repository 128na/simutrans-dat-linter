//! `rules::check_good` の統合テスト。testdata/ の正常系1件で、
//! 期待する診断が出ない（＝空のRuleセット）ことを確認する。
//! `tests/way_lint.rs`と同じ形式に揃えている。
//!
//! `good_writer.cc`自体にmakeobj時点でfatal/warningになる分岐は無いが
//! （`src/rules/good.rs`冒頭のREJECTEDコメント参照）、全obj種別共通の
//! `name-forbidden-filename-character`/`embedded-nul-in-string-field`
//! （`NameAndCopyrightStringFieldRule`）と、`catg`/`speed_bonus`/`mapcolor`の
//! `narrow-int-overflow`は本ファイルでテストする。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した good dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_good(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has(diags: &[(Severity, &str)], severity: Severity, code: &str) -> bool {
    diags.iter().any(|(s, c)| *s == severity && *c == code)
}

#[test]
fn valid_good_has_no_errors_or_warnings() {
    let diags = check("good_valid.dat");
    assert!(diags.is_empty(), "予期しない診断: {diags:?}");
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=Passagiere. （末尾ドット）は、Windowsのファイル名として保存できない
    // ため、root_writer_t::write()のseparate出力やuncopy()でfopen()が失敗する
    // （src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has(
        &check("good_name_forbidden_filename_character.dat"),
        Severity::Error,
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_name_is_detected() {
    // name=\"Passagiere\0Extra\" は埋め込みNULバイトを含む。
    // text_writer_t::write_obj（text_writer.cc:18）はstrlen()で長さを計算するため、
    // \0以降の"Extra"が警告無く切り詰められる。
    assert!(has(
        &check("good_embedded_nul_name.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // catg=300はuint8の範囲(0..255)外。good_writer.cc:25の
    // node.write_uint8(fp, obj.get_int("catg", 0))へ静かに切り詰められる。
    assert!(has(
        &check("good_narrow_int_overflow.dat"),
        Severity::Warning,
        "narrow-int-overflow"
    ));
}
