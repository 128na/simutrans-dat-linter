// 各obj種別（building/vehicle/way/good/bridge/tunnel/roadsign/crossing/way-object/
// ground_obj/tree/citycar/pedestrian/factory/sound/ground/menu/cursor/symbol/
// smoke/field/misc の22種）の検証ルールは、いずれもmakeobj本体（vanilla simutrans
// pinned commit `1d2799f9a73adf94751e2d8357fea9dabcc4f740`）の対応する
// `descriptor/writer/*.cc`をソースとして直接ミラーしている。
// 各obj種別の検証根拠・調査経緯・OTRP側diffの有無等の詳細は、各ファイル
// （`rules/building.rs`, `rules/vehicle.rs`, ... 等）冒頭の`//!`doc comment参照。

pub mod bridge;
pub mod building;
pub mod citycar;
pub mod common;
pub mod crossing;
pub mod cursor;
pub mod factory;
pub mod field;
pub mod good;
pub mod ground;
pub mod groundobj;
pub mod keys;
pub mod menu;
pub mod misc;
pub mod pedestrian;
pub mod roadsign;
pub mod smoke;
pub mod sound;
pub mod symbol;
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
pub use cursor::check_cursor;
pub use factory::check_factory;
pub use field::check_field;
pub use good::check_good;
pub use ground::check_ground;
pub use groundobj::check_groundobj;
pub use menu::check_menu;
pub use misc::check_misc;
pub use pedestrian::check_pedestrian;
pub use roadsign::check_roadsign;
pub use smoke::check_smoke;
pub use sound::check_sound;
pub use symbol::check_symbol;
pub use tree::check_tree;
pub use tunnel::check_tunnel;
pub use vehicle::check_vehicle;
pub use way::check_way;
pub use way_obj::check_way_obj;
