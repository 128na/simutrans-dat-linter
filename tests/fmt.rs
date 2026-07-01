//! `formatter` の統合テスト。`--reorder` の期待出力一致と、
//! デフォルト整形（順序保持）の冪等性を確認する。

use dat_linter::formatter;
use std::fs;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn read(file: &str) -> String {
    fs::read_to_string(testdata_dir().join(file))
        .unwrap_or_else(|e| panic!("{file} の読み込みに失敗: {e}"))
}

#[test]
fn reorder_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries);
    // 慣習順（obj, name, copyright, type, enables_pax）に並び替わり、
    // キーは小文字化、`Name = Hoge` の値は "Hoge" にトリムされる。
    let expected = "obj=building\nname=Hoge\ncopyright=fuga\ntype=station\nenables_pax=1\n";
    assert_eq!(out, expected);
}

#[test]
fn preserve_order_is_idempotent() {
    let text = read("roundtrip_test.dat");
    let once = formatter::format_preserve_order(&formatter::parse_entries(&text).entries);
    let twice = formatter::format_preserve_order(&formatter::parse_entries(&once).entries);
    assert_eq!(once, twice, "順序保持フォーマットは冪等であるべき");
}

#[test]
fn reorder_is_idempotent() {
    let text = read("roundtrip_test.dat");
    let (once, _) = formatter::format_reordered(&formatter::parse_entries(&text).entries);
    let (twice, _) = formatter::format_reordered(&formatter::parse_entries(&once).entries);
    assert_eq!(once, twice, "並び替えフォーマットは冪等であるべき");
}
