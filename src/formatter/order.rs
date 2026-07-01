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
pub fn order_for(obj: &str) -> Option<&'static OrderSpec> {
    match obj {
        "building" => Some(&BUILDING_ORDER),
        "vehicle" => Some(&VEHICLE_ORDER),
        "way" => Some(&WAY_ORDER),
        "good" => Some(&GOOD_ORDER),
        "bridge" => Some(&BRIDGE_ORDER),
        "tunnel" => Some(&TUNNEL_ORDER),
        _ => None,
    }
}

// building dat の「慣習的な並び」。try-out/station_test/station_cube.dat の実例と
// building_writer.cc 内で obj.get(...)が呼ばれる順序を参考にした。
const BUILDING_NAMED: &[&str] = &[
    "obj",
    "name",
    "copyright",
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
