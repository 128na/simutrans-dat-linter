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
