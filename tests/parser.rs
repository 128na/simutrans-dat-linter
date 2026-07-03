//! `parser::DatFile` の統合テスト。実際のmakeobj (`tabfileobj_t::put()`) が
//! 重複キーを「先勝ち」で扱う（`tabfile.h`のdocコメント「the first value is used」、
//! `put()`実装 `if(objinfo.get(key).str) return false;`）ことをミラーできているか、
//! および行番号追跡が正しいかを検証する。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::Path;

fn testdata_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

#[test]
fn duplicate_key_keeps_first_value() {
    let path = testdata_dir().join("duplicate_key.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    // 2行目 name=FirstName / 3行目 name=SecondName -> 実際のmakeobjは1行目(先勝ち)を採用する
    assert_eq!(dat.get("name"), Some("FirstName"));
}

#[test]
fn duplicate_key_is_recorded() {
    let path = testdata_dir().join("duplicate_key.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    assert_eq!(dat.duplicates.len(), 1);
    let dup = &dat.duplicates[0];
    assert_eq!(dup.key, "name");
    assert_eq!(dup.first_line, 2);
    assert_eq!(dup.duplicate_line, 3);
}

#[test]
fn line_of_reports_first_occurrence() {
    let path = testdata_dir().join("duplicate_key.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    assert_eq!(dat.line_of("obj"), Some(1));
    assert_eq!(dat.line_of("name"), Some(2));
    assert_eq!(dat.line_of("nonexistent-key"), None);
}

#[test]
fn no_duplicates_in_clean_file() {
    let path = testdata_dir().join("roundtrip_test.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    assert!(dat.duplicates.is_empty());
}

#[test]
fn shift_jis_encoded_file_is_decoded_as_fallback() {
    // 古いpak128.japan系アドオンはShift-JIS(CP932)のまま配布されていることがある。
    // UTF-8として不正でも「読み込み失敗」にせず、CP932としてデコードして継続する。
    let path = testdata_dir().join("shift_jis_encoded.dat");
    let dat = DatFile::parse(&path).expect("Shift-JISファイルもパースできるべき");
    assert_eq!(dat.get("name"), Some("SJISName"));
}

#[test]
fn check_duplicate_keys_reports_warning_with_location() {
    // rules::check_duplicate_keys はobj種別を問わずrun_lintから無条件に呼ばれる
    // （パーサレベルの一般的な問題のため）。ここではその診断生成自体を検証する。
    let path = testdata_dir().join("duplicate_key.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_duplicate_keys(&dat);
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.code, "duplicate-key");
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 3);
    assert_eq!(loc.key, "name");
}
