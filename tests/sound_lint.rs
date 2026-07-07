//! `rules::check_sound` の統合テスト。testdata/ の正常系1件で、
//! 期待する診断が出ない（＝空のRuleセット）ことを確認する。
//! `tests/good_lint.rs`と同じ形式に揃えている。
//!
//! `sound_writer.cc`自体にmakeobj時点でfatal/warningになる分岐は無いが
//! （`src/rules/sound.rs`冒頭のREJECTEDコメント参照）、全obj種別共通の
//! `name-forbidden-filename-character`/`embedded-nul-in-string-field`
//! （`NameAndCopyrightStringFieldRule`）と、`sound_nr`の`narrow-int-overflow`
//! （`SoundNrNarrowIntRule`）は本ファイルでテストする。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した sound dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_sound(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has(diags: &[(Severity, &str)], severity: Severity, code: &str) -> bool {
    diags.iter().any(|(s, c)| *s == severity && *c == code)
}

#[test]
fn valid_sound_has_no_errors_or_warnings() {
    let diags = check("sound_valid.dat");
    assert!(diags.is_empty(), "予期しない診断: {diags:?}");
}

#[test]
fn name_forbidden_filename_character_is_detected() {
    // name=CON はWindowsの予約デバイス名と完全一致する。root_writer_t::write()の
    // separate出力・uncopy()がこの値をそのままfopen()するため、ビルド/分割が
    // 失敗する（src/rules/common.rsのforbidden_filename_reason参照）。
    assert!(has(
        &check("sound_name_forbidden_filename_character.dat"),
        Severity::Error,
        "name-forbidden-filename-character"
    ));
}

#[test]
fn embedded_nul_in_copyright_is_detected() {
    // copyright="fuga\0bar" は埋め込みNULバイトを含む。text_writer_t::write_obj
    // （text_writer.cc:18）はstrlen()で長さを計算するため、\0以降の"bar"が
    // 警告無く切り詰められる。
    assert!(has(
        &check("sound_embedded_nul_copyright.dat"),
        Severity::Warning,
        "embedded-nul-in-string-field"
    ));
}

#[test]
fn narrow_int_overflow_is_detected() {
    // sound_nr=100000はuint16の範囲(0..65535)外。sound_writer.cc:29の
    // `(uint16)obj.get_int("sound_nr", NO_SOUND)`という明示キャストで
    // 静かに切り詰められる。
    assert!(has(
        &check("sound_narrow_int_overflow.dat"),
        Severity::Warning,
        "narrow-int-overflow"
    ));
}
