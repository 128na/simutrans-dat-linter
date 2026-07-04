//! `fmt --reorder`のobj種別ごとの並び順定義。
//! `tabfileobj_t::objinfo`は`stringhashtable_tpl`（記述順はmakeobjの動作に一切影響しない）
//! であるため、この並び順は技術的な必須要件ではなくスタイル上の慣習に過ぎない
//! （デフォルトでは適用せず`--reorder`でオプトイン）。

/// 1つの並び替えグループの分類方法。
pub enum Section {
    /// このリストに含まれるキーだけがこのグループに入り、リスト内の位置で並び替える。
    Named(&'static [&'static str]),
    /// いずれかのprefixで始まるキーがこのグループに入り、`bracket_indices`で並び替える。
    Bracket(&'static [&'static str]),
    /// 他のどのセクションにもマッチしなかったキー。パース順（挿入順）を保つ。
    /// 各`OrderSpec`にちょうど1つ含めること。
    Unknown,
}

pub struct OrderSpec {
    pub sections: &'static [Section],
}

/// `obj=`の値に応じた並び順を返す。未対応のobj種別には`None`。
///
/// 内部では`obj`文字列をまず`registry::ObjType`にパースし、`ObjType`に対する
/// **ワイルドカードarmを持たない網羅match**でディスパッチする。これにより
/// `ObjType`に23番目のvariantを追加したのにこのmatchへのarm追加を忘れると
/// `cargo build`が非網羅match errorで失敗する（`registry::RuleSet::for_obj_type`と
/// 対になる、このリファクタの要点）。公開シグネチャ（`&str`入力・
/// `Option<&'static OrderSpec>`出力）は変更しない。
pub fn order_for(obj: &str) -> Option<&'static OrderSpec> {
    use crate::registry::ObjType;
    let obj_type = ObjType::from_str(obj)?;
    match obj_type {
        ObjType::Building => Some(&BUILDING_ORDER),
        ObjType::Vehicle => Some(&VEHICLE_ORDER),
        ObjType::Way => Some(&WAY_ORDER),
        ObjType::Good => Some(&GOOD_ORDER),
        ObjType::Bridge => Some(&BRIDGE_ORDER),
        ObjType::Tunnel => Some(&TUNNEL_ORDER),
        ObjType::Roadsign => Some(&ROADSIGN_ORDER),
        ObjType::Crossing => Some(&CROSSING_ORDER),
        ObjType::WayObject => Some(&WAY_OBJ_ORDER),
        ObjType::GroundObj => Some(&GROUNDOBJ_ORDER),
        ObjType::Tree => Some(&TREE_ORDER),
        ObjType::Citycar => Some(&CITYCAR_ORDER),
        ObjType::Pedestrian => Some(&PEDESTRIAN_ORDER),
        ObjType::Factory => Some(&FACTORY_ORDER),
        ObjType::Sound => Some(&SOUND_ORDER),
        ObjType::Ground => Some(&GROUND_ORDER),
        ObjType::Menu => Some(&MENU_ORDER),
        ObjType::Cursor => Some(&CURSOR_ORDER),
        ObjType::Symbol => Some(&SYMBOL_ORDER),
        ObjType::Smoke => Some(&SMOKE_ORDER),
        ObjType::Field => Some(&FIELD_ORDER),
        ObjType::Misc => Some(&MISC_ORDER),
    }
}

// building dat の「慣習的な並び」。try-out/station_test/station_cube.dat の実例と
// building_writer.cc 内で obj.get(...)が呼ばれる順序を参考にした。
const BUILDING_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "type",
    // extension_buildingはtype=stop/extension+waytypeに置き換えられた廃止済み
    // キーワード（building_writer.cc:203-204でdbg->fatalになる）だが、実データ
    // （refs/linter_test/JpClassicTerminal.dat等の古いpakset）には現存する。
    // このNamed一覧に無いキーはSection::Unknownへ落ち、他に該当キーが無ければ
    // 前後を空行で挟まれた1行だけの孤立ブロックになってしまうため、typeの直後に
    // 明示的に位置を与えている（reorderの見た目を安定させるための対処であり、
    // obsolete判定自体は引き続きbuilding.rsのlintルールが担う）。
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
];
const BUILDING_CURSOR_ICON: &[&str] = &["cursor", "icon"];

static BUILDING_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(BUILDING_NAMED),
        Section::Named(BUILDING_CURSOR_ICON),
        Section::Unknown,
        Section::Bracket(&["frontimage[", "backimage["]),
    ],
};

// vehicle dat の「慣習的な並び」。vehicle_writer.cc:89-166のフィールド書き込み順
// （write_name_and_copyright -> cost -> payload -> loading_time -> speed -> weight ->
// axle_load -> power -> runningcost -> fixed_cost/maintenance -> intro/retire ->
// gear -> waytype -> sound -> engine_type -> length -> freight -> smoke）から導出。
const VEHICLE_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
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
];

static VEHICLE_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(VEHICLE_NAMED),
        Section::Unknown,
        Section::Bracket(&["constraint[prev][", "constraint[next]["]),
        Section::Bracket(&["emptyimage[", "freightimage[", "freightimagetype["]),
    ],
};

// way dat の「慣習的な並び」。way_writer.cc:37-90のフィールド読み取り・書き込み順
// （cost -> maintenance -> topspeed -> max_weight -> axle_load -> clip_below ->
// intro/retire -> waytype -> system_type -> draw_as_ding、その後
// write_name_and_copyright で name/copyright、続けて image[...] 系の並び）から導出。
const WAY_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
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
];
const WAY_CURSOR_ICON: &[&str] = &["cursor", "icon"];

static WAY_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(WAY_NAMED),
        Section::Named(WAY_CURSOR_ICON),
        Section::Unknown,
        Section::Bracket(&[
            "image[",
            "frontimage[",
            "imageup[",
            "frontimageup[",
            "diagonal[",
            "frontdiagonal[",
        ]),
    ],
};

// good dat の「慣習的な並び」。good_writer.cc:15-31のフィールド読み取り順
// （write_name_and_copyright -> metric -> value -> catg -> speed_bonus ->
// weight_per_unit -> mapcolor）から導出。good_writer.cc全文にimage/cursor/icon
// 系フィールドへの言及が無いため、Bracketセクションは無い
// （未知の追加キーは全てUnknownセクションでパース順のまま保持される）。
const GOOD_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "metric",
    "value",
    "catg",
    "speed_bonus",
    "weight_per_unit",
    "mapcolor",
];

static GOOD_ORDER: OrderSpec = OrderSpec {
    sections: &[Section::Named(GOOD_NAMED), Section::Unknown],
};

// bridge dat の「慣習的な並び」。bridge_writer.cc:101-115のフィールド読み取り順
// （waytype -> topspeed -> cost -> maintenance -> pillar_distance ->
// pillar_asymmetric -> max_lenght/max_length -> max_height -> axle_load ->
// clip_below -> intro/retire）、続けてwrite_name_and_copyrightでname/copyright
// （bridge_writer.cc:139,155）、その後write_bridge_images内でcursor/icon
// （season<=0のときのみ、bridge_writer.cc:85-89）とimage系キー
// （bridge_writer.cc:25-43のnames配列順）が書かれる、という順序から導出。
const BRIDGE_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
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
];
const BRIDGE_CURSOR_ICON: &[&str] = &["cursor", "icon"];

static BRIDGE_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(BRIDGE_NAMED),
        Section::Named(BRIDGE_CURSOR_ICON),
        Section::Unknown,
        Section::Bracket(&[
            "backimage[",
            "frontimage[",
            "backstart[",
            "frontstart[",
            "backramp[",
            "frontramp[",
            "backpillar[",
            "frontpillar[",
            "backimage2[",
            "frontimage2[",
            "backstart2[",
            "frontstart2[",
            "backramp2[",
            "frontramp2[",
            "backpillar2[",
            "frontpillar2[",
        ]),
    ],
};

// tunnel dat の「慣習的な並び」。tunnel_writer.cc:22-33のフィールド読み取り順
// （topspeed -> cost -> maintenance -> waytype -> intro/retire -> axle_load）、
// 続けてwrite_name_and_copyrightでname/copyright（tunnel_writer.cc:74）、
// その後season=0のときのみcursor/icon（tunnel_writer.cc:80-82,107）と
// front/backimage系キー（tunnel_writer.cc:84-98の走査順）が書かれる、
// という順序から導出。
const TUNNEL_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "topspeed",
    "cost",
    "maintenance",
    "waytype",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "axle_load",
];
const TUNNEL_CURSOR_ICON: &[&str] = &["cursor", "icon"];

static TUNNEL_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(TUNNEL_NAMED),
        Section::Named(TUNNEL_CURSOR_ICON),
        Section::Unknown,
        Section::Bracket(&["frontimage[", "backimage["]),
    ],
};

// roadsign dat の「慣習的な並び」。roadsign_writer.cc:83-132のフィールド読み取り・
// 書き込み順（cost -> maintenance -> min_speed -> offset_left -> waytype ->
// is_signal/free_route/is_presignal/is_prioritysignal/is_longblocksignal/
// single_way/is_private/no_foreground/end_of_choose -> intro/retire）、
// 続けてwrite_name_and_copyrightでname/copyright（roadsign_writer.cc:134）、
// その後image[...]系キー（roadsign_writer.cc:139-149）とcursor/icon
// （roadsign_writer.cc:152-158、*c||*iのときのみ）が書かれる、という順序から導出。
const ROADSIGN_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
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
];
const ROADSIGN_CURSOR_ICON: &[&str] = &["cursor", "icon"];

static ROADSIGN_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(ROADSIGN_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
        Section::Named(ROADSIGN_CURSOR_ICON),
    ],
};

// crossing dat の「慣習的な並び」。crossing_writer.cc:73-156のフィールド書き込み順
// （write_name_and_copyrightでname/copyright -> waytype[0]/waytype[1]
// （crossing_writer.cc:78-84） -> speed[0]/speed[1]（87-94） ->
// animation_time_open/animation_time_closed（97-100） -> sound（52-108、値の計算は
// node確保前だが実際に読むキーは"sound"のみ） -> intro_year/intro_month/
// retire_year/retire_month（110-117）、その後openimage/front_openimage/
// closedimage/front_closedimage系のimageキー（120-156の走査順）が書かれる、
// という順序から導出。crossingにはcursor/iconフィールドへの言及が
// crossing_writer.cc全文に無いため、他obj種別と異なりCURSOR_ICONセクションは無い。
const CROSSING_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "waytype[0]",
    "waytype[1]",
    "speed[0]",
    "speed[1]",
    "animation_time_open",
    "animation_time_closed",
    "sound",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
];

static CROSSING_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(CROSSING_NAMED),
        Section::Unknown,
        Section::Bracket(&[
            "openimage[",
            "front_openimage[",
            "closedimage[",
            "front_closedimage[",
        ]),
    ],
};

// way-object dat の「慣習的な並び」。way_obj_writer.cc:32-56のフィールド読み取り・
// 書き込み順（cost -> maintenance -> topspeed -> intro/retire -> waytype ->
// own_waytype、続けてwrite_name_and_copyrightでname/copyright、way_obj_writer.cc:56）、
// その後frontimage/backimage系（61-69） -> frontimageup/backimageup系（76-84） ->
// frontimageup2/backimageup2系（85-97） -> frontdiagonal/backdiagonal系（104-112） ->
// cursor/icon（116-119、way_obj_writer.cc内で常に最後に書かれる。他obj種別と異なり
// CURSOR_ICONセクションが末尾に来る点はroadsignと同じ配置）という順序から導出。
const WAY_OBJ_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "cost",
    "maintenance",
    "topspeed",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "waytype",
    "own_waytype",
];
const WAY_OBJ_CURSOR_ICON: &[&str] = &["cursor", "icon"];

static WAY_OBJ_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(WAY_OBJ_NAMED),
        Section::Unknown,
        Section::Bracket(&[
            "frontimage[",
            "backimage[",
            "frontimageup[",
            "backimageup[",
            "frontimageup2[",
            "backimageup2[",
            "frontdiagonal[",
            "backdiagonal[",
        ]),
        Section::Named(WAY_OBJ_CURSOR_ICON),
    ],
};

// ground_obj dat の「慣習的な並び」。groundobj_writer.cc:17-100のフィールド読み取り順
// （write_name_and_copyrightでname/copyright（groundobj_writer.cc:20） -> climates
// -> seasons -> distributionweight -> cost -> speed -> trees_on_top -> waytype、
// その後image[<phase|dir>][<season>]系キー（52-99の走査順）が書かれる、という順序
// から導出。ground_obj全文にcursor/iconフィールドへの言及が無いため、他obj種別と
// 異なりCURSOR_ICONセクションは無い（crossingと同じパターン）。
const GROUNDOBJ_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "climates",
    "seasons",
    "distributionweight",
    "cost",
    "speed",
    "trees_on_top",
    "waytype",
];

static GROUNDOBJ_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(GROUNDOBJ_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// tree dat の「慣習的な並び」。tree_writer.cc:17-69のフィールド読み取り順
// （climates -> seasons -> distributionweight、その後age(0..4固定)×season走査で
// image[<age>][<season>]系キーを読む。write_name_and_copyrightの呼び出し自体は
// フィールド読み取りループの後（tree_writer.cc:58）だが、他obj種別と同様に
// name/copyrightはobj直後に配置する慣習に揃えた）という順序から導出。
// tree_writer.cc全文にcursor/iconフィールドへの言及が無いため、他obj種別と
// 異なりCURSOR_ICONセクションは無い（crossing/ground_objと同じパターン）。
const TREE_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "climates",
    "seasons",
    "distributionweight",
];

static TREE_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(TREE_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// citycar dat の「慣習的な並び」。citycar_writer.cc:19-51のフィールド読み取り順
// （distributionweight -> intro_year/intro_month -> retire_year/retire_month ->
// speed、続けてwrite_name_and_copyrightでname/copyright（citycar_writer.cc:33）、
// その後image[<dir>]系キー（citycar_writer.cc:38-46のdir_codes走査順）が書かれる、
// という順序から導出。citycar_writer.cc全文にcursor/iconフィールドへの言及が無いため、
// 他obj種別と異なりCURSOR_ICONセクションは無い（crossing/ground_obj/treeと同じパターン）。
const CITYCAR_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "distributionweight",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
    "speed",
];

static CITYCAR_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(CITYCAR_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// pedestrian dat の「慣習的な並び」。pedestrian_writer.cc:15-89のフィールド読み取り順
// （write_name_and_copyrightでname/copyright（pedestrian_writer.cc:21） ->
// distributionweight -> (画像走査 image[<dir>] または image[<dir>][<frame>]) ->
// steps_per_frame（アニメーション時のみ評価されるがdat記述者から見た書く場所は
// 画像キーの近くが自然） -> offset -> intro_year/intro_month -> retire_year/
// retire_month）という順序から導出。citycarと同様、pedestrian_writer.cc全文に
// cursor/iconフィールドへの言及が無いため、CURSOR_ICONセクションは無い。
const PEDESTRIAN_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
    "distributionweight",
    "steps_per_frame",
    "offset",
    "intro_year",
    "intro_month",
    "retire_year",
    "retire_month",
];

static PEDESTRIAN_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(PEDESTRIAN_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// factory dat の「慣習的な並び」。factory_writer.cc:154-354のフィールド読み取り順
// （location -> productivity -> range -> distributionweight -> mapcolor ->
// pax_level -> expand_probability -> expand_minimum/expand_range/expand_times ->
// electricity_boost -> passenger_boost -> mail_boost -> electricity_amount/
// electricity_demand -> passenger_demand -> mail_demand -> sound_interval ->
// sound、その後building_writer_t::write_obj（factory_writer.cc:223）を直接
// 呼び出すため、name/copyright -> type（factory側では書かない） -> waytype
// （通常未使用） -> enables_pax/enables_post/enables_ware -> level -> noinfo/
// noconstruction/needs_ground -> climates -> dims -> chance -> animation_time
// -> intro/retire -> preservation -> capacity -> maintenance -> cost ->
// allow_underground -> cursor/icon -> タイル画像（building.rsのBUILDING_ORDERと
// 共通のセクション構成）が挟まる。building呼び出しの後、smoke（224） ->
// inputgood/inputsupplier/inputcapacity/inputfactor[N]（231-249） ->
// outputgood/outputcapacity/outputfactor[N]（251-272） -> fields/fields[N]
// と関連するhas_snow/production_per_field/storage_capacity/spawn_weight[N]・
// probability_to_spawn/max_fields/min_fields/start_fields（44-98、279で呼び出し）
// -> smoketile[N]/smokeoffset[N]/smokeuplift/smokelifetime（284-309）という順序
// から導出。
const FACTORY_NAMED_PRE_BUILDING: &[&str] = &[
    "obj",
    "name",
    "copyright",
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
];
// building_writer_t::write_obj経由でfactoryにもそのまま適用されるフィールド
// （BUILDING_NAMEDのうち、factoryのpakset実例で実際に使われ得るもの）。
const FACTORY_NAMED_BUILDING: &[&str] = &[
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
];
const FACTORY_CURSOR_ICON: &[&str] = &["cursor", "icon"];
const FACTORY_NAMED_POST_BUILDING: &[&str] = &[
    "smoke",
    "probability_to_spawn",
    "max_fields",
    "min_fields",
    "start_fields",
    "fields",
    "smokeuplift",
    "smokelifetime",
];

static FACTORY_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(FACTORY_NAMED_PRE_BUILDING),
        Section::Named(FACTORY_NAMED_BUILDING),
        Section::Named(FACTORY_CURSOR_ICON),
        Section::Named(FACTORY_NAMED_POST_BUILDING),
        Section::Unknown,
        Section::Bracket(&[
            "frontimage[",
            "backimage[",
            "inputgood[",
            "inputsupplier[",
            "inputcapacity[",
            "inputfactor[",
            "outputgood[",
            "outputcapacity[",
            "outputfactor[",
            "fields[",
            "has_snow[",
            "production_per_field[",
            "storage_capacity[",
            "spawn_weight[",
            "smoketile[",
            "smokeoffset[",
        ]),
    ],
};

// sound dat の「慣習的な並び」。sound_writer.cc:14-32のフィールド読み取り・書き込み順
// （sound_name を先に読むが、実際のノード書き込みは write_name_and_copyright で
// name/copyright（sound_writer.cc:19） -> sound_nr（27） -> sound_name本体（28-29）
// という順）から導出。goodと同様、sound_writer.cc全文にimage/cursor/icon/waytype
// 系フィールドへの言及が無いため、Bracketセクションは無い（未知の追加キーは全て
// Unknownセクションでパース順のまま保持される）。
const SOUND_NAMED: &[&str] = &["obj", "name", "copyright", "sound_nr", "sound_name"];

static SOUND_ORDER: OrderSpec = OrderSpec {
    sections: &[Section::Named(SOUND_NAMED), Section::Unknown],
};

// ground dat の「慣習的な並び」。ground_writer.cc:15-44のフィールド読み取り順
// （write_name_and_copyrightでname/copyright（ground_writer.cc:19） -> その後
// slope=0..127 x phase=0,1,2,...の走査順でimage[<slope>][<phase>]系キーが
// 書かれる）という順序から導出。good/soundと同様、ground_writer.cc全文に
// waytype/climates/cursor/icon系フィールドへの言及が無いため、Namedセクションは
// obj/name/copyrightのみで、それ以外はBracketセクション（image[）にまとめる。
const GROUND_NAMED: &[&str] = &["obj", "name", "copyright"];

static GROUND_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(GROUND_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// menu dat の「慣習的な並び」。skin_writer_t::write_obj（skin_writer.cc:21-51、
// menuskin_writer_tはこれをオーバーライドしない）のフィールド読み取り・書き込み順
// （image[0], image[1], ...を1次元・無制限に走査してkeysを構築した後、
// write_name_and_copyrightでname/copyright（skin_writer.cc:46） -> その後
// imagelist_writer_t::write_objでimage系ノードを書く）という順序から導出。
// dat記述者から見た自然な配置に揃え、他obj種別と同様name/copyrightをobj直後に置く。
// good/sound/groundと同様、skin_writer.cc/skin_writer.h全文に waytype/climates/
// cursor/icon系フィールドへの言及が無いため、Namedセクションはobj/name/copyright
// のみで、それ以外はBracketセクション（image[）にまとめる。
const MENU_NAMED: &[&str] = &["obj", "name", "copyright"];

static MENU_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(MENU_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// cursor dat の「慣習的な並び」。skin_writer_t::write_obj（skin_writer.cc:21-51、
// cursorskin_writer_tはこれをオーバーライドしない）のフィールド読み取り・書き込み順は
// menuと完全に同一（image[0], image[1], ...を1次元・無制限に走査してkeysを構築した後、
// write_name_and_copyrightでname/copyright（skin_writer.cc:46） -> その後
// imagelist_writer_t::write_objでimage系ノードを書く）ため、並び順もMENU_ORDERと
// 同一の構成にする。skin_writer.cc/skin_writer.h全文に waytype/climates/cursor/icon
// （フィールドとしての）系フィールドへの言及が無いため、Namedセクションは
// obj/name/copyrightのみで、それ以外はBracketセクション（image[）にまとめる。
const CURSOR_NAMED: &[&str] = &["obj", "name", "copyright"];

static CURSOR_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(CURSOR_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// symbol dat の「慣習的な並び」。skin_writer_t::write_obj（skin_writer.cc:21-51、
// symbolskin_writer_tはこれをオーバーライドしない）のフィールド読み取り・書き込み順は
// menu/cursorと完全に同一（image[0], image[1], ...を1次元・無制限に走査してkeysを構築
// した後、write_name_and_copyrightでname/copyright（skin_writer.cc:46） -> その後
// imagelist_writer_t::write_objでimage系ノードを書く）ため、並び順もMENU_ORDER/
// CURSOR_ORDERと同一の構成にする。skin_writer.cc/skin_writer.h全文に waytype/climates/
// cursor/icon（フィールドとしての）系フィールドへの言及が無いため、Namedセクションは
// obj/name/copyrightのみで、それ以外はBracketセクション（image[）にまとめる。
const SYMBOL_NAMED: &[&str] = &["obj", "name", "copyright"];

static SYMBOL_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(SYMBOL_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// smoke dat の「慣習的な並び」。skin_writer_t::write_obj（skin_writer.cc:21-51、
// smoke_writer_tはこれをオーバーライドしない）のフィールド読み取り・書き込み順は
// menu/cursor/symbolと完全に同一（image[0], image[1], ...を1次元・無制限に走査して
// keysを構築した後、write_name_and_copyrightでname/copyright（skin_writer.cc:46） ->
// その後imagelist_writer_t::write_objでimage系ノードを書く）ため、並び順も
// MENU_ORDER/CURSOR_ORDER/SYMBOL_ORDERと同一の構成にする。skin_writer.cc/
// skin_writer.h全文に waytype/climates/cursor/icon（フィールドとしての）系
// フィールドへの言及が無いため、Namedセクションはobj/name/copyrightのみで、
// それ以外はBracketセクション（image[）にまとめる。`obj=factory`の
// `smoketile[N]`/`smokeoffset[N]`（FACTORY_ORDERのBracketセクションに既存）とは
// 全くの別obj種別・別フィールドであることに注意。
const SMOKE_NAMED: &[&str] = &["obj", "name", "copyright"];

static SMOKE_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(SMOKE_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// field dat の「慣習的な並び」。skin_writer_t::write_obj（skin_writer.cc:21-51、
// field_writer_tはこれをオーバーライドしない）のフィールド読み取り・書き込み順は
// menu/cursor/symbol/smokeと完全に同一（image[0], image[1], ...を1次元・無制限に
// 走査してkeysを構築した後、write_name_and_copyrightでname/copyright
// （skin_writer.cc:46） -> その後imagelist_writer_t::write_objでimage系ノードを
// 書く）ため、並び順もMENU_ORDER/CURSOR_ORDER/SYMBOL_ORDER/SMOKE_ORDERと同一の
// 構成にする。skin_writer.cc/skin_writer.h全文に waytype/climates/cursor/icon
// （フィールドとしての）系フィールドへの言及が無いため、Namedセクションは
// obj/name/copyrightのみで、それ以外はBracketセクション（image[）にまとめる。
// `obj=factory`の`fields`/`max_fields`/`min_fields`/`start_fields`
// （FACTORY_ORDERのNamed/Bracketセクションに既存）とは全くの別obj種別・
// 別フィールドであることに注意。
const FIELD_NAMED: &[&str] = &["obj", "name", "copyright"];

static FIELD_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(FIELD_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};

// misc dat の「慣習的な並び」。skin_writer_t::write_obj（skin_writer.cc:21-51、
// miscimages_writer_tはこれをオーバーライドしない）のフィールド読み取り・書き込み順は
// menu/cursor/symbol/smoke/fieldと完全に同一（image[0], image[1], ...を1次元・
// 無制限に走査してkeysを構築した後、write_name_and_copyrightでname/copyright
// （skin_writer.cc:46） -> その後imagelist_writer_t::write_objでimage系ノードを
// 書く）ため、並び順もMENU_ORDER/CURSOR_ORDER/SYMBOL_ORDER/SMOKE_ORDER/FIELD_ORDERと
// 同一の構成にする。skin_writer.cc/skin_writer.h全文に waytype/climates/cursor/icon
// （フィールドとしての）系フィールドへの言及が無いため、Namedセクションは
// obj/name/copyrightのみで、それ以外はBracketセクション（image[）にまとめる。
const MISC_NAMED: &[&str] = &["obj", "name", "copyright"];

static MISC_ORDER: OrderSpec = OrderSpec {
    sections: &[
        Section::Named(MISC_NAMED),
        Section::Unknown,
        Section::Bracket(&["image["]),
    ],
};
