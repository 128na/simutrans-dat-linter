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
    // "menu" は本ツールがまだ対応していないobj種別の例
    // （かつては "wayobj" -> "groundobj" -> "tree" -> "citycar" -> "pedestrian" ->
    // "factory" -> "sound" -> "ground" の順で使っていたが、obj=way-object /
    // obj=ground_obj / obj=tree / obj=citycar / obj=pedestrian / obj=factory /
    // obj=sound / obj=ground として順にサポートしたため、真に未対応の別のobj種別
    // 文字列に更新し続けている。
    // factory/sound/groundまででこのプロジェクトが対応してきたobj種別
    // （building/vehicle/way/good/bridge/tunnel/roadsign/crossing/way-object/
    // ground_obj/tree/citycar/pedestrian/factory/sound/groundの16種）は完了したが、
    // soundマイルストーンでの再調査（`descriptor/writer/`配下を機械的に
    // 網羅した結果）、makeobjには本ツール・過去の計画のどちらにも含まれて
    // いなかった独立したトップレベルobj種別がまだ複数存在することが判明した:
    // `ground_writer.h`の"ground"（ground_writer_t、register_writer(true)、
    // 本マイルストーンでobj=groundとしてサポート済み）、および`skin_writer.h`の
    // "menu"/"cursor"/"symbol"/"smoke"/"field"/"misc"
    // （menuskin_writer_t/cursorskin_writer_t/symbolskin_writer_t/
    // smoke_writer_t/field_writer_t/miscimages_writer_t、いずれも
    // register_writer(true)）。これらはpakset作者が直接.datを書く対象という
    // より、pakset全体で1つだけ書くメタ的スキン/アイコン定義に近く別途の
    // 検討が必要なため、本マイルストーンでは対象に含めない
    // （"menu"はその中から未対応プレースホルダとして選んだ一例）。
    // menuskin_writer_t::get_type_name()（skin_writer.h）は"menu"を返し、
    // registry::RuleSet::for_obj_typeのmatch armにまだ存在しないことを確認済み。
    let (out, warnings) = formatter::format_reordered(&parsed.entries, "menu");
    let preserved = formatter::format_preserve_order(&parsed.entries);
    assert_eq!(out, preserved);
    assert!(warnings.iter().any(|w| w.contains("obj=menu")));
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
fn reorder_way_obj_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_way_obj_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "way-object");
    let expected = "\
obj=way-object
name=Catenary
copyright=fuga
cost=1000
waytype=track
own_waytype=electrified_track

frontimage[-]=catenary.png.0.0

cursor=catenary_icon.png.0.0
icon=catenary_icon.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_groundobj_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_groundobj_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "ground_obj");
    let expected = "\
obj=ground_obj
name=Rock
copyright=fuga
climates=rocky,tundra

image[0][0]=rock.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_tree_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_tree_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "tree");
    let expected = "\
obj=tree
name=Oak
copyright=fuga
climates=temperate,tundra
seasons=1

image[0][0]=tree.png.0.0
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

#[test]
fn reorder_citycar_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_citycar_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "citycar");
    let expected = "\
obj=citycar
name=Sedan
copyright=fuga
distributionweight=2
speed=50

image[s]=citycar.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_pedestrian_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_pedestrian_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "pedestrian");
    let expected = "\
obj=pedestrian
name=Walker
copyright=fuga
distributionweight=8

image[s]=pedestrian.png.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_sound_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_sound_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "sound");
    let expected = "\
obj=sound
name=Cash
copyright=fuga
sound_nr=15
sound_name=cash.wav
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_factory_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_factory_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "factory");
    let expected = "\
obj=factory
name=Glassworks
copyright=fuga
location=land
mapcolor=194

dims=1,1

cursor=factory_icon.png.0.0
icon=factory_icon.png.0.0

outputcapacity[0]=400
outputgood[0]=glass
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_ground_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_ground_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "ground");
    let expected = "\
obj=ground
name=Slopes
copyright=fuga

image[0][0]=slope.png.0.0
";
    assert_eq!(out, expected);
}
