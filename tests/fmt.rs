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
fn reorder_handles_dash_separated_multi_object_file() {
    // 建物の複数ステージ等、1ファイルに`-`区切りで複数のobj定義が連結されている
    // 実例（refs/building.JpClassicTerminal/JpClassicTerminal.dat）を模したfixture。
    // 各obj定義は区切りを跨がず**独立して**並び替えられ、区切り行自体は
    // 原文のまま元の位置に復元されるべき。
    let parsed = formatter::parse_entries(&read("fmt_multi_object_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "building");
    let expected = "\
obj=building
name=StageA
copyright=fuga
type=station
-------------------------------------------------------------------------------
obj=building
name=StageB
copyright=fuga
type=station
";
    assert_eq!(out, expected);
}

#[test]
fn preserve_order_does_not_warn_on_separator_line() {
    // `-`始まりの区切り行はreal makeobjでも正常なobj定義の終端マーカーであり、
    // Malformed（`=`が無い不正行）としての警告を出すべきではない。
    let text = read("fmt_multi_object_example.dat");
    let parsed = formatter::parse_entries(&text);
    assert!(
        parsed.warnings.is_empty(),
        "区切り行だけのfixtureで警告が出ないべき: {:?}",
        parsed.warnings
    );
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
    // "nonexistent-obj-type" は本ツールが対応していない（そもそも実在しない）
    // obj種別文字列のプレースホルダ。かつては "wayobj" -> "groundobj" -> "tree" ->
    // "citycar" -> "pedestrian" -> "factory" -> "sound" -> "ground" -> "menu" ->
    // "cursor" -> "symbol" -> "smoke" -> "field" -> "misc" の順で実在する未対応
    // obj種別文字列を使い回してきたが、obj=way-object / obj=ground_obj / obj=tree /
    // obj=citycar / obj=pedestrian / obj=factory / obj=sound / obj=ground /
    // obj=menu / obj=cursor / obj=symbol / obj=smoke / obj=field / obj=misc として
    // 順にサポートしたため、真に未対応の候補が尽きた。
    //
    // misc マイルストーンで、`src/simutrans/descriptor/writer/` 配下の**全ヘッダ
    // ファイル**（`*_writer.h`という命名規則に限らず、ディレクトリ内の全`.h`、
    // 具体的には bridge/building/citycar/crossing/factory/good/ground/groundobj/
    // image/imagelist/imagelist2d/obj_node/obj_pak_exception/obj_writer/
    // pedestrian/roadsign/root/skin/sound/text/tree/tunnel/vehicle/way_obj/way/
    // xref の24ヘッダ全て）を対象に、`register_writer(true)`と`get_type_name()`を
    // 機械的に棚卸しした。`obj_writer_t::register_writer(bool main_obj)`
    // （obj_writer.cc:24-36）の実装は`if (main_obj) { writer_by_name->put(...) }`
    // であり、`writer_by_name`こそが`obj_writer_t::write`（obj_writer.cc:39-59、
    // `.dat`の`obj=`フィールドの値=`type`でルックアップする実体）が引く
    // ハッシュテーブルである。つまり`register_writer(true)`で登録される
    // クラスだけが「`.dat`に書ける`obj=`のトップレベル値」であり、
    // `register_writer(false)`（tile_writer_t/factory_field_class_writer_t/
    // factory_field_group_writer_t/factory_smoke_writer_t/factory_product_writer_t/
    // factory_supplier_writer_t/image_writer_t/imagelist_writer_t/
    // imagelist2d_writer_t/root_writer_t/text_writer_t/xref_writer_t）は
    // 内部専用の補助writerであり`.dat`の`obj=`には書けないことも確認した
    // （`writer_by_type`にのみ登録され、型IDでの内部参照専用）。
    //
    // 棚卸しの結果、`register_writer(true)`を持つクラスは厳密に22個であり、
    // その`get_type_name()`の返り値は次の22種で尽きる: building, vehicle, way,
    // good, bridge, tunnel, roadsign, crossing, way-object, ground_obj, tree,
    // citycar, pedestrian, factory, sound, ground, menu, cursor, symbol, smoke,
    // field, misc。これは`registry::RuleSet::for_obj_type`のmatch armと
    // 1対1で完全に一致する（本misc追加により22種目が埋まった）。よって
    // makeobjが認識するトップレベルobj種別のカバレッジは本マイルストームで
    // 完全（22/22）になったと判断し、このテストのプレースホルダには実在しない
    // 架空の文字列 "nonexistent-obj-type" を採用する。
    //
    // なお`cursor=`/`icon=`という**フィールド**は building/way/bridge等の多くの
    // obj種別に存在するが、これはトップレベルの`obj=cursor`（cursorskin_writer_t）
    // とは全くの別概念であり、混同しないこと。同様に`obj=factory`の
    // `smoketile[N]=`/`smokeoffset[N]=`/`smoke=`**フィールド**も、トップレベルの
    // `obj=smoke`（smoke_writer_t）とは全くの別概念である。さらに`obj=factory`の
    // `fields=`/`max_fields=`/`min_fields=`/`start_fields=`**フィールド**も、
    // トップレベルの`obj=field`（field_writer_t）とは全くの別概念である。
    let (out, warnings) = formatter::format_reordered(&parsed.entries, "nonexistent-obj-type");
    let preserved = formatter::format_preserve_order(&parsed.entries);
    assert_eq!(out, preserved);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("obj=nonexistent-obj-type"))
    );
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

#[test]
fn reorder_menu_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_menu_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "menu");
    let expected = "\
obj=menu
name=WindowSkin
copyright=fuga

image[0]=skins.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_cursor_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_cursor_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "cursor");
    let expected = "\
obj=cursor
name=MouseCursor
copyright=fuga

image[0]=mouse.1.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_symbol_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_symbol_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "symbol");
    let expected = "\
obj=symbol
name=Builder
copyright=fuga

image[0]=builder_symbol.1.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_smoke_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_smoke_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "smoke");
    let expected = "\
obj=smoke
name=Diesel
copyright=fuga

image[0]=misc-smoke-128.0.0
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_field_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_field_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "field");
    let expected = "\
obj=field
name=CornField
copyright=fuga

image[0]=corn_farm.4.3
";
    assert_eq!(out, expected);
}

#[test]
fn reorder_misc_matches_expected_output() {
    let parsed = formatter::parse_entries(&read("fmt_misc_example.dat"));
    let (out, _warnings) = formatter::format_reordered(&parsed.entries, "misc");
    let expected = "\
obj=misc
name=Construction
copyright=fuga

image[0]=construction.1.0
";
    assert_eq!(out, expected);
}
