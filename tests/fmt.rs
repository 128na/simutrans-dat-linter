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
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "building");
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
    let (once, _) =
        formatter::format_reordered(&formatter::parse_entries(&text).entries, "building");
    let (twice, _) =
        formatter::format_reordered(&formatter::parse_entries(&once).entries, "building");
    assert_eq!(once, twice, "並び替えフォーマットは冪等であるべき");
}

#[test]
fn reorder_unsupported_obj_falls_back_to_preserve_order() {
    let text = read("roundtrip_test.dat");
    let parsed = formatter::parse_entries(&text);
    let (out, warnings) = formatter::format_reordered(&parsed.entries, "wayobj");
    let preserved = formatter::format_preserve_order(&parsed.entries);
    assert_eq!(out, preserved);
    assert!(warnings.iter().any(|w| w.contains("obj=wayobj")));
}

#[test]
fn reorder_way_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_way_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "way");
    let expected = "\
obj=way
name=Highway
copyright=fuga
cost=1000
waytype=road

cursor=road_icon.png.0.0
icon=road_icon.png.0.0

image[-]=road.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_good_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_good_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "good");
    let expected = "\
obj=good
name=Passagiere
copyright=fuga
metric=Personen
value=100
mapcolor=255
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_bridge_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_bridge_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "bridge");
    let expected = "\
obj=bridge
name=Skyway
copyright=fuga
waytype=road
cost=1000

cursor=road_icon.png.0.0
icon=road_icon.png.0.0

frontimage[ns]=bridge.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_tunnel_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_tunnel_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "tunnel");
    let expected = "\
obj=tunnel
name=Underpass
copyright=fuga
cost=1000
waytype=road

cursor=road_icon.png.0.0
icon=road_icon.png.0.0

frontimage[n][0]=tunnel.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_roadsign_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_roadsign_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "roadsign");
    let expected = "\
obj=roadsign
name=Signal
copyright=fuga
cost=1000
waytype=track

image[n][0]=signal.png.0.0

cursor=signal_icon.png.0.0
icon=signal_icon.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_crossing_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_crossing_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "crossing");
    let expected = "\
obj=crossing
name=Level Crossing
copyright=fuga
waytype[0]=road
waytype[1]=track
speed[0]=80
speed[1]=120

openimage[ew][0]=crossing.png.0.0
openimage[ns][0]=crossing.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_vehicle_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_vehicle_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "vehicle");
    let expected = "\
obj=vehicle
name=Loco
copyright=fuga
cost=1000
speed=100
waytype=track
engine_type=electric
freight=Passagiere

constraint[next][0]=Wagon
constraint[prev][0]=none
";
    assert_eq!(out, expected);
}
