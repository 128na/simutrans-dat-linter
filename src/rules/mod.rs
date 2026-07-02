// building/vehicle/way/good/bridge/tunnel/roadsign/crossing/way-object/ground_obj/tree/citycar/pedestrian/factory/sound の検証ロジック（cursor/icon省略時のスキップ、タイル画像欠落時の
// phases=0、frontimageのh>0、Dims size=0 fatal、vehicleのwaytype必須、engine_type
// フォールバック、8方向画像・freightimage完全性、wayのwaytype必須・base image必須・
// clip_belowクランプ、goodのfatal/warning分岐皆無の確認、bridgeのwaytype必須・
// get_int_clampedクランプ群・front画像未指定警告、tunnelのwaytype必須・
// 季節数/portal幅の可変画像キー走査、roadsignのwaytype必須・numbered/2D排他画像構文の
// fatal分岐再現、crossingの2waytype必須・解決後waytype一致検出・speed両方必須・
// openimage両方向必須、way-objectの2waytype（waytype/own_waytype）必須・
// ribi/slope/diagonal可変画像キー走査、ground_objのwaytype省略可・
// speed分岐によるphase/8方向季節画像走査、treeのage(5固定)×season全組み合わせ画像
// 必須、citycarのwaytype/engine_type/freight/constraint系フィールド皆無・
// 8方向画像の無条件（早期終了なし）走査、pedestrianのis_animated判定による
// 静止8方向画像/アニメーション画像（方向ごとに独立したフレーム走査）の排他分岐、
// factoryのbuilding_writer直接呼び出し（Dims/タイル画像/cursor・icon共有）・
// type上書きの罠・mapcolor必須・outputcapacity/smoketile-smokeoffsetの非fatal
// error・probability系クランプ、soundのfatal/warning分岐皆無の確認（goodと同型）等）は、
// いずれもmakeobjの
// building_writer.cc / vehicle_writer.cc / way_writer.cc / good_writer.cc /
// bridge_writer.cc / tunnel_writer.cc / roadsign_writer.cc / crossing_writer.cc /
// way_obj_writer.cc / groundobj_writer.cc / tree_writer.cc / citycar_writer.cc /
// pedestrian_writer.cc / factory_writer.cc / sound_writer.cc / get_waytype.cc /
// xref_writer.cc / tabfile.cc を
// ソースとして直接ミラーしている。
//
// 検証済み:
// - vanilla simutrans: このリポジトリの `simutrans` submodule, commit 1d2799f9a7 (2026-01-16)
// - OTRP (Simutrans-Extended系フォーク, https://github.com/teamhimeh/simutrans),
//   commit d6d3a5795b (2026-07-01時点のdefaultブランチ) で同等ファイルを diff した結果、
//   building dat の検証に関わるロジックは両者で完全に一致していた
//   （差分はnode書き込みのバイナリフォーマット詳細のみで、dat記述者から見える挙動は同一）
//
// どちらかの本体が更新され、上記コミット以降にtype/waytype一覧やcursor/icon・
// タイル画像・vehicle画像・way画像・goodフィールド・tunnel画像キー・roadsign画像キー・
// crossing画像キー・way-object画像キー・ground_obj画像キー・tree画像キー・citycar画像キー・
// pedestrian画像キー・factoryフィールド・soundフィールドのロジックが変わった場合は
// この定数表を再検証すること。
// vehicle系・way系・good系・bridge系・tunnel系・roadsign系・crossing系・way-object系・
// ground_obj系・tree系・citycar系・pedestrian系・factory系・sound系のルールはOTRP側での
// 個別diffはまだ行っていない（rules/vehicle.rs, rules/way.rs, rules/good.rs, rules/bridge.rs,
// rules/tunnel.rs, rules/roadsign.rs, rules/crossing.rs, rules/way_obj.rs,
// rules/groundobj.rs, rules/tree.rs, rules/citycar.rs, rules/pedestrian.rs,
// rules/factory.rs, rules/sound.rs参照）。

pub mod bridge;
pub mod building;
pub mod citycar;
pub mod common;
pub mod crossing;
pub mod factory;
pub mod good;
pub mod groundobj;
pub mod pedestrian;
pub mod roadsign;
pub mod sound;
pub mod tree;
pub mod tunnel;
pub mod vehicle;
pub mod way;
pub mod way_obj;

pub use bridge::check_bridge;
pub use building::check_building;
pub use citycar::check_citycar;
pub use common::check_duplicate_keys;
pub use crossing::check_crossing;
pub use factory::check_factory;
pub use good::check_good;
pub use groundobj::check_groundobj;
pub use pedestrian::check_pedestrian;
pub use roadsign::check_roadsign;
pub use sound::check_sound;
pub use tree::check_tree;
pub use tunnel::check_tunnel;
pub use vehicle::check_vehicle;
pub use way::check_way;
pub use way_obj::check_way_obj;
