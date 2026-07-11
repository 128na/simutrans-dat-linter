//! obj種別ごとに「実際に有効なキー一覧」を宣言的に持つレジストリ。
//!
//! ## 位置づけ（`KNOWN_WAYTYPES`/`DIR_CODES`との関係）
//!
//! このファイルは`rules/common.rs`の`KNOWN_WAYTYPES`/`DIR_CODES`と全く同じ、
//! **手動キュレーションされたデータ**である。`keys_for`のmatch自体が機械的に
//! 強制するのは「`ObjType`の22種別を1つも取りこぼさずカバーしているか」という
//! **網羅性のみ**（`registry::RuleSet::for_obj_type`・`formatter::order::order_for`と
//! 同じ設計、ワイルドカードarmを持たない網羅match）であり、各定数（`BUILDING_KEYS`等）
//! に列挙した個々のキー文字列が実際に正しいかどうかはコンパイラの関与しない
//! 人力の正確性に依存する。
//!
//! 各キーの正しさの根拠は次の2箇所にある:
//!
//! 1. `rules/<obj種別>.rs`のモジュール冒頭docコメント（各キーがmakeobjの
//!    どの分岐で読まれ、欠落・不正値がどう扱われるかを記載）
//! 2. `formatter/order.rs`の`<OBJ>_NAMED`/`<OBJ>_*_ORDER`（`.dat`記述順の慣習を
//!    導出した際に、対応する`descriptor/writer/*.cc`のフィールド読み取り順を
//!    直接読んで確認済みのキー一覧。本ファイルの一次情報源として最も網羅的）
//!
//! 個々のキーの妥当性に疑問があれば、上記2箇所およびそれらが参照する
//! `refs/simutrans`（vanilla simutransのC++ソース、pinned commit）の
//! `descriptor/writer/*.cc`を直接確認すること。
//!
//! ## 添字の扱い
//!
//! `frontimage[l][y][x][h][phase][season]`のような角括弧添字付きキーは、
//! 添字を全て除いた**ベース名のみ**を保持する（`frontimage`のように）。
//! `constraint[prev][N]`/`constraint[next][N]`のような名前付き添字も同様に
//! ベース名（`constraint`）へ畳み込む。これはVSCode拡張のシンタックス
//! ハイライト・スニペット機能が「このobj種別でこのキー名は有効か」を
//! 判定するための一覧であり、添字の内部構造（何番目か、prev/nextどちらか等）
//! までは表現しない。

use crate::registry::ObjType;

/// 全22obj種別で共有される2フィールド（`obj_writer_t::write_name_and_copyright`が
/// factory以外の21種で直接呼ばれ、factoryはbuilding経由で間接的に対象になる。
/// 実質22種全てが対象。`rules/common.rs`の`NameAndCopyrightStringFieldRule`参照）。
const COMMON_KEYS: &[&str] = &["name", "copyright"];

/// `formatter/order.rs`の`BUILDING_NAMED`/`BUILDING_CURSOR_ICON`/Bracket
/// （`frontimage[`/`backimage[`）から導出。
const BUILDING_KEYS: &[&str] = &[
    "type",
    "extension_building",
    "waytype",
    "enables_pax",
    "enables_post",
    "enables_ware",
    "level",
    "noinfo",
    "noconstruction",
    "needs_ground",
    "climates",
    "dims",
    "chance",
    "animation_time",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "preservation_year",
    "preservation_month",
    "capacity",
    "station_capacity",
    "maintenance",
    "station_maintenance",
    "cost",
    "station_price",
    "allow_underground",
    "cursor",
    "icon",
    "frontimage",
    "backimage",
];

/// `formatter/order.rs`の`VEHICLE_NAMED`/Bracket（`constraint[prev][`/
/// `constraint[next][` -> `constraint`、`emptyimage[`/`freightimage[`/
/// `freightimagetype[`）から導出。
const VEHICLE_KEYS: &[&str] = &[
    "cost",
    "payload",
    "loading_time",
    "speed",
    "weight",
    "axle_load",
    "power",
    "runningcost",
    "fixed_cost",
    "maintenance",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "gear",
    "waytype",
    "sound",
    "engine_type",
    "length",
    "freight",
    "smoke",
    "constraint",
    "emptyimage",
    "freightimage",
    "freightimagetype",
];

/// `formatter/order.rs`の`WAY_NAMED`/`WAY_CURSOR_ICON`/Bracket
/// （`image[`/`frontimage[`/`imageup[`/`frontimageup[`/`diagonal[`/
/// `frontdiagonal[`）から導出。
const WAY_KEYS: &[&str] = &[
    "cost",
    "maintenance",
    "topspeed",
    "max_weight",
    "axle_load",
    "clip_below",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "waytype",
    "system_type",
    "draw_as_ding",
    "cursor",
    "icon",
    "image",
    "frontimage",
    "imageup",
    "frontimageup",
    "diagonal",
    "frontdiagonal",
];

/// `formatter/order.rs`の`GOOD_NAMED`から導出。good_writer.ccは画像・cursor/icon・
/// waytype系フィールドを一切読まないため、それ以外のキーは無い。
const GOOD_KEYS: &[&str] = &[
    "metric",
    "value",
    "catg",
    "speed_bonus",
    "weight_per_unit",
    "mapcolor",
];

/// `formatter/order.rs`の`BRIDGE_NAMED`/`BRIDGE_CURSOR_ICON`/Bracket
/// （front/back × image/start/ramp/pillar、無印・2番目の2系統）から導出。
const BRIDGE_KEYS: &[&str] = &[
    "waytype",
    "topspeed",
    "cost",
    "maintenance",
    "pillar_distance",
    "pillar_asymmetric",
    "max_lenght",
    "max_length",
    "max_height",
    "axle_load",
    "clip_below",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "cursor",
    "icon",
    "backimage",
    "frontimage",
    "backstart",
    "frontstart",
    "backramp",
    "frontramp",
    "backpillar",
    "frontpillar",
    "backimage2",
    "frontimage2",
    "backstart2",
    "frontstart2",
    "backramp2",
    "frontramp2",
    "backpillar2",
    "frontpillar2",
];

/// `formatter/order.rs`の`TUNNEL_NAMED`/`TUNNEL_CURSOR_ICON`/Bracket
/// （`frontimage[`/`backimage[`）から導出。
const TUNNEL_KEYS: &[&str] = &[
    "topspeed",
    "cost",
    "maintenance",
    "waytype",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "axle_load",
    "cursor",
    "icon",
    "frontimage",
    "backimage",
];

/// `formatter/order.rs`の`ROADSIGN_NAMED`/`ROADSIGN_CURSOR_ICON`/Bracket
/// （`image[`）から導出。
const ROADSIGN_KEYS: &[&str] = &[
    "cost",
    "maintenance",
    "min_speed",
    "offset_left",
    "waytype",
    "is_signal",
    "free_route",
    "is_presignal",
    "is_prioritysignal",
    "is_longblocksignal",
    "single_way",
    "is_private",
    "no_foreground",
    "end_of_choose",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "image",
    "cursor",
    "icon",
];

/// `formatter/order.rs`の`CROSSING_NAMED`（`waytype[0]`/`waytype[1]` ->
/// `waytype`、`speed[0]`/`speed[1]` -> `speed`）/Bracket
/// （`openimage[`/`front_openimage[`/`closedimage[`/`front_closedimage[`）から導出。
/// crossingにはcursor/iconフィールドへの言及が無い（`rules/crossing.rs`参照）。
const CROSSING_KEYS: &[&str] = &[
    "waytype",
    "speed",
    "animation_time_open",
    "animation_time_closed",
    "sound",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "openimage",
    "front_openimage",
    "closedimage",
    "front_closedimage",
];

/// `formatter/order.rs`の`WAY_OBJ_NAMED`/`WAY_OBJ_CURSOR_ICON`/Bracket
/// （front/back × image/imageup/imageup2/diagonal）から導出。
const WAY_OBJECT_KEYS: &[&str] = &[
    "cost",
    "maintenance",
    "topspeed",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "waytype",
    "own_waytype",
    "frontimage",
    "backimage",
    "frontimageup",
    "backimageup",
    "frontimageup2",
    "backimageup2",
    "frontdiagonal",
    "backdiagonal",
    "cursor",
    "icon",
];

/// `formatter/order.rs`の`GROUNDOBJ_NAMED`/Bracket（`image[`）から導出。
/// ground_objにはcursor/iconフィールドへの言及が無い（`rules/groundobj.rs`参照）。
const GROUND_OBJ_KEYS: &[&str] = &[
    "climates",
    "seasons",
    "distributionweight",
    "cost",
    "speed",
    "trees_on_top",
    "waytype",
    "image",
];

/// `formatter/order.rs`の`TREE_NAMED`/Bracket（`image[`）から導出。
/// treeにはwaytype/cursor/iconフィールドへの言及が無い（`rules/tree.rs`参照）。
const TREE_KEYS: &[&str] = &["climates", "seasons", "distributionweight", "image"];

/// `formatter/order.rs`の`CITYCAR_NAMED`/Bracket（`image[`）から導出。
/// citycarにはwaytype/cursor/iconフィールドへの言及が無い（`rules/citycar.rs`参照）。
const CITYCAR_KEYS: &[&str] = &[
    "distributionweight",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "speed",
    "image",
];

/// `formatter/order.rs`の`PEDESTRIAN_NAMED`/Bracket（`image[`）から導出。
/// pedestrianにはwaytype/cursor/iconフィールドへの言及が無い（`rules/pedestrian.rs`参照）。
const PEDESTRIAN_KEYS: &[&str] = &[
    "distributionweight",
    "steps_per_frame",
    "offset",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "image",
];

/// `formatter/order.rs`の`FACTORY_NAMED_PRE_BUILDING`/`FACTORY_NAMED_BUILDING`/
/// `FACTORY_CURSOR_ICON`/`FACTORY_NAMED_POST_BUILDING`/Bracketから導出。
/// factoryは`building_writer_t::write_obj`をそのまま呼び出すため
/// （`rules/factory.rs`参照）、buildingの主要フィールドもそのまま有効なキーになる。
/// `type`はfactory_writer.cc:220が`obj.put("type","fac")`で上書きしようとするが
/// 既存キーがあれば先勝ちで失敗する（`TypeOverrideRule`参照）ため、書くこと自体は
/// 構文として有効（推奨されないだけ）としてここに含める。`fields`はNamed
/// （単一形）とBracket `fields[`（インデックス形）の両方の書き方があるため1つに
/// まとめてある。
const FACTORY_KEYS: &[&str] = &[
    "location",
    "productivity",
    "range",
    "distributionweight",
    "mapcolor",
    "pax_level",
    "expand_probability",
    "expand_minimum",
    "expand_range",
    "expand_times",
    "electricity_boost",
    "passenger_boost",
    "mail_boost",
    "electricity_amount",
    "electricity_demand",
    "passenger_demand",
    "mail_demand",
    "sound_interval",
    "sound",
    "type",
    "waytype",
    "enables_pax",
    "enables_post",
    "enables_ware",
    "level",
    "noinfo",
    "noconstruction",
    "needs_ground",
    "climates",
    "dims",
    "chance",
    "animation_time",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "preservation_year",
    "preservation_month",
    "capacity",
    "maintenance",
    "cost",
    "allow_underground",
    "cursor",
    "icon",
    "smoke",
    "probability_to_spawn",
    "max_fields",
    "min_fields",
    "start_fields",
    "fields",
    "smokeuplift",
    "smokelifetime",
    "frontimage",
    "backimage",
    "inputgood",
    "inputsupplier",
    "inputcapacity",
    "inputfactor",
    "outputgood",
    "outputcapacity",
    "outputfactor",
    "has_snow",
    "production_per_field",
    "storage_capacity",
    "spawn_weight",
    "smoketile",
    "smokeoffset",
];

/// `formatter/order.rs`の`SOUND_NAMED`から導出。soundは画像・waytype・cursor/icon
/// 系フィールドを一切読まない（`rules/sound.rs`参照）。
const SOUND_KEYS: &[&str] = &["sound_nr", "sound_name"];

/// `formatter/order.rs`の`GROUND_NAMED`/Bracket（`image[`）から導出。groundは
/// waytype/climates/cursor/icon系フィールドを一切読まない（`rules/ground.rs`参照）。
const GROUND_KEYS: &[&str] = &["image"];

/// `menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6種別は全て共通の基底クラス
/// `skin_writer_t::write_obj`をそのまま使い、`image[i]`の1次元・無制限走査 +
/// name/copyrightのみという全く同一の構造を持つ（`rules/menu.rs`冒頭docコメント
/// 参照）。`formatter/order.rs`の`MENU_ORDER`〜`MISC_ORDER`も全て
/// `Section::Named(&["obj","name","copyright"])` + `Section::Bracket(&["image["])`
/// という同一構成であり、この6種別のobj固有キーは`image`の1つだけ。
const SKIN_STYLE_KEYS: &[&str] = &["image"];

/// obj種別に対応する固有キー一覧（`COMMON_KEYS`を含まない）を返す。
///
/// **ワイルドカードなしの網羅match**にすること
/// （`registry::RuleSet::for_obj_type`・`formatter::order::order_for`と同じ設計。
/// 23番目のvariantを追加した際にここへのarm追加を忘れると`cargo build`が
/// 非網羅match errorで失敗する）。
pub fn keys_for(obj_type: ObjType) -> Vec<&'static str> {
    let specific: &'static [&'static str] = match obj_type {
        ObjType::Building => BUILDING_KEYS,
        ObjType::Vehicle => VEHICLE_KEYS,
        ObjType::Way => WAY_KEYS,
        ObjType::Good => GOOD_KEYS,
        ObjType::Bridge => BRIDGE_KEYS,
        ObjType::Tunnel => TUNNEL_KEYS,
        ObjType::Roadsign => ROADSIGN_KEYS,
        ObjType::Crossing => CROSSING_KEYS,
        ObjType::WayObject => WAY_OBJECT_KEYS,
        ObjType::GroundObj => GROUND_OBJ_KEYS,
        ObjType::Tree => TREE_KEYS,
        ObjType::Citycar => CITYCAR_KEYS,
        ObjType::Pedestrian => PEDESTRIAN_KEYS,
        ObjType::Factory => FACTORY_KEYS,
        ObjType::Sound => SOUND_KEYS,
        ObjType::Ground => GROUND_KEYS,
        ObjType::Menu => SKIN_STYLE_KEYS,
        ObjType::Cursor => SKIN_STYLE_KEYS,
        ObjType::Symbol => SKIN_STYLE_KEYS,
        ObjType::Smoke => SKIN_STYLE_KEYS,
        ObjType::Field => SKIN_STYLE_KEYS,
        ObjType::Misc => SKIN_STYLE_KEYS,
    };
    COMMON_KEYS.iter().chain(specific).copied().collect()
}

/// 特定のキーが取りうる既知の値一覧（VSCode拡張が値側の補完候補を出すためのもの）。
/// `keys_for`とは独立した、キー名 -> 値候補のマッピング。
///
/// - `waytype`: `rules/common.rs`の`KNOWN_WAYTYPES`（`get_waytype()`が受理する13値）
/// - `direction`: `rules/common.rs`の`DIR_CODES`（vehicle/citycar/pedestrianが共有する
///   8方向。`.dat`上のキー名としては`emptyimage[s]`のように添字として現れるため、
///   `direction`という仮想的な名前で値一覧のみを提供する）
pub fn known_values() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        ("waytype", crate::rules::common::KNOWN_WAYTYPES),
        ("direction", &crate::rules::common::DIR_CODES),
    ]
}
