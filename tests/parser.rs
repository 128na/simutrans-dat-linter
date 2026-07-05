//! `parser::DatFile` の統合テスト。実際のmakeobj (`tabfileobj_t::put()`) が
//! 重複キーを「先勝ち」で扱う（`tabfile.h`のdocコメント「the first value is used」、
//! `put()`実装 `if(objinfo.get(key).str) return false;`）ことをミラーできているか、
//! および行番号追跡が正しいかを検証する。

use dat_linter::diagnostics::Severity;
use dat_linter::i18n::Language;
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
fn parse_all_splits_dash_separated_records() {
    // 建物の複数ステージ等、1ファイルに`-`区切りで複数のobj定義が連結されている
    // 実例（refs/building.JpClassicTerminal/JpClassicTerminal.dat）を模したfixture。
    // real makeobj (tabfile_t::read(): `while(read_line(...) && *line != '-')`) は
    // 行頭が`-`の行でobj定義を区切って1つずつ読む。
    let path = testdata_dir().join("multi_object_building.dat");
    let records = DatFile::parse_all(&path).expect("パースに失敗");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].get("name"), Some("StageA"));
    assert_eq!(records[1].get("name"), Some("StageB"));
    assert_eq!(records[2].get("name"), Some("StageC"));
}

#[test]
fn parse_all_does_not_report_cross_record_duplicates() {
    // レコード跨ぎの obj=/name= 等が「重複キー」に誤判定されないことを確認する
    // （このバグの再現・回帰防止が本テストの目的）。
    let path = testdata_dir().join("multi_object_building.dat");
    let records = DatFile::parse_all(&path).expect("パースに失敗");
    assert!(records[0].duplicates.is_empty());
    assert!(records[2].duplicates.is_empty());
}

#[test]
fn parse_all_still_detects_duplicate_within_a_single_record() {
    // レコード内の本物の重複（fixtureの2番目のオブジェクトに仕込んだ
    // `name=StageB` / `name=StageB_dup`）は引き続き検知されるべき。
    let path = testdata_dir().join("multi_object_building.dat");
    let records = DatFile::parse_all(&path).expect("パースに失敗");
    assert_eq!(records[1].duplicates.len(), 1);
    assert_eq!(records[1].duplicates[0].key, "name");
}

#[test]
fn parse_returns_only_first_record_for_backward_compat() {
    // 既存の単一オブジェクトAPI(`DatFile::parse`)は、複数obj定義があっても
    // 最初の1件のみを返す（25箇所の既存呼び出し元との後方互換のため）。
    let path = testdata_dir().join("multi_object_building.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    assert_eq!(dat.get("name"), Some("StageA"));
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
fn ribi_parameter_expansion_matches_real_pak128_crossing_syntax() {
    // E:\simutrans_addon\pak128\infrastructure\road_rail_crossings\
    // p128_crossing_road040_rail080.dat の実際の記述をそのまま再現したfixture:
    //   OpenImage[NS,EW][0-1]=p128_crossing_road040_rail080.<0+$1>.<2*$0+1>
    // 展開ロジックの手計算結果:
    //   field0(ribi)=ns(idx0)/ew(idx1) が $0、field1(numeric)=0/1 が $1
    //   ns,0 -> "0.1"(0+0, 2*0+1) / ns,1 -> "1.1"(0+1, 2*0+1)
    //   ew,0 -> "0.3"(0+0, 2*1+1) / ew,1 -> "1.3"(0+1, 2*1+1)
    let path = testdata_dir().join("crossing_ribi_param_expansion_arithmetic.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    assert_eq!(
        dat.get("openimage[ns][0]"),
        Some("p128_crossing.0.1"),
        "openimage[ns][0]の展開結果が実データの手計算と一致しない"
    );
    assert_eq!(
        dat.get("openimage[ns][1]"),
        Some("p128_crossing.1.1"),
        "openimage[ns][1]の展開結果が実データの手計算と一致しない"
    );
    assert_eq!(
        dat.get("openimage[ew][0]"),
        Some("p128_crossing.0.3"),
        "openimage[ew][0]の展開結果が実データの手計算と一致しない"
    );
    assert_eq!(
        dat.get("openimage[ew][1]"),
        Some("p128_crossing.1.3"),
        "openimage[ew][1]の展開結果が実データの手計算と一致しない"
    );
}

#[test]
fn check_duplicate_keys_reports_warning_with_location() {
    // rules::check_duplicate_keys はobj種別を問わずrun_lintから無条件に呼ばれる
    // （パーサレベルの一般的な問題のため）。ここではその診断生成自体を検証する。
    let path = testdata_dir().join("duplicate_key.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let diags = rules::check_duplicate_keys(&dat, Language::default());
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.code, dat_linter::codes::DiagnosticCode::DuplicateKey);
    let loc = d.location.as_ref().expect("locationが付与されているべき");
    assert_eq!(loc.line, 3);
    assert_eq!(loc.key, "name");
}
