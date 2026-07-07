//! `obj=building` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! `NarrowIntFieldsRule`（`capacity`）は`DateIndexOverflowRule`と同種の「静的解析」層の
//! ルール。`building_writer.cc:244` `sint32 capacity = obj.get_int("capacity", level * 32);`
//! はローカル変数こそ`sint32`だが、実際の書き込みは`building_writer.cc:365`
//! `node.write_uint16(fp, capacity);`であり、`sint32`→`uint16`という符号・幅の
//! 両方が変わる narrowing が発生する（ローカル変数の型だけを見て「sint32だから
//! 広くて安全」と判断してはいけない、という具体例。`common::check_narrow_int_overflow_field`
//! のdocコメント参照）。

use super::common::{
    CursorIconPolicy, CursorIconRule, DimsRule, KNOWN_WAYTYPES, NameAndCopyrightStringFieldRule,
    TileImageRule, check_date_index_overflow_field, check_narrow_int_overflow_field, resolve_dims,
};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::{Language, t};
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

const KNOWN_TYPES: &[&str] = &[
    "res",
    "com",
    "ind",
    "cur",
    "mon",
    "tow",
    "hq",
    "habour",
    "harbour",
    "dock",
    "fac",
    "stop",
    "extension",
    "depot",
    "any",
    "",
];
const OBSOLETE_TYPES: &[&str] = &[
    "station",
    "railstop",
    "monorailstop",
    "busstop",
    "carstop",
    "airport",
    "wharf",
    "hall",
    "post",
    "shed",
];
const TYPES_REQUIRING_WAYTYPE: &[&str] = &["stop", "depot"];

/// `cursor`/`icon`が両方未指定でも「ビルドメニューに表示されない」という
/// `missing-cursor-icon`の根拠が当てはまらない`type`値。
///
/// 第7弾でpak128実データ（`missing-cursor-icon`1368件）を再調査した結果、
/// `builder/hausbauer.cc`を根拠に次のことを確認した:
/// - 実際にプレイヤーが選んでビルドできる建物メニュー
///   （`hausbauer_t::fill_menu()`が`tool_selector`へ追加する唯一の経路）は、
///   `hausbauer_t::successfully_loaded()`で`station_building`リストに
///   登録される5種類の`btype`（`dock`/`flat_dock`/`depot`/`generic_stop`/
///   `generic_extension`。`.dat`の`type=`では`habour`/`harbour`/`dock`/
///   `depot`/`stop`/`extension`）のみが対象。この5種は
///   `hausbauer_t::register_desc()`（234-236行目）で`cursor`が実在する
///   場合のみビルドツールが生成されるため、`missing-cursor-icon`の根拠は
///   これらの`type`には正しく当てはまる
/// - `res`/`com`/`ind`（`city_res`/`city_com`/`city_ind`）は
///   `city_residential`/`city_commercial`/`city_industry`という**別の**
///   リストへ登録され、`get_residential()`/`get_commercial()`/
///   `get_industrial()`（`get_city_building_from_list()`経由）で
///   都市成長シミュレーション（`world/simcity.cc`）が自動選択・自動配置する。
///   `get_city_building_from_list()`の選定条件（997行目付近、
///   `is_allowed_climate`/`distribution_weight`/`is_available`/サイズ）に
///   cursorへの言及は一切無く、プレイヤーが選ぶビルドメニューにも
///   一切現れない（`fill_menu()`はこれらのリストを読まない）
/// - `mon`（`monument`）/`cur`（`attraction_land`/`attraction_city`）/
///   `tow`（`townhall`）も同様に`monuments`/`attractions_land`/
///   `attractions_city`/`townhalls`という別リストへ登録されるのみ。
///   `monuments`は`world/simcity.cc`側の特殊建造物自動配置ロジック
///   （`get_special()`）が使う`unbuilt_monuments`の初期値として使われるのみで、
///   `attractions_land`/`attractions_city`/`townhalls`を読むのは
///   `gui/curiosity_edit.cc`という**pakset編集者向けツール**のみ
///   （通常のプレイヤー向けビルドメニューではない）
/// - `hq`（`headquarters`）は`register_desc()`が`tool_headquarter_t`を
///   個別に生成する特殊経路を持ち、`city_res`等とは異なり実際に
///   cursorの有無がプレイヤーの建設可否に影響するため、この一覧には含めない
///
/// pak128実データで実際に`missing-cursor-icon`が出ていた1368件は、
/// この一覧の`res`(858)/`com`(173)/`ind`(158)/`cur`(63)/`mon`(18)/`tow`(2)と、
/// `obj=factory`（`type=`未指定で`fac`相当、`factory.rs`側で別途対応）の96件で
/// 完全に説明できることを確認済み（`stop`/`extension`/`depot`/`dock`/`hq`の
/// 実例は1件も含まれていなかった）。
const TYPES_WITHOUT_BUILD_MENU: &[&str] = &["res", "com", "ind", "cur", "mon", "tow"];

/// この obj 種別に対する検査項目一式。`DimsRule`が返す(size_x, size_y, layouts)を
/// `TileImageRule`のコンストラクタへ渡す必要があるため、ここで一度だけ`resolve_dims`を
/// 呼んで解決してから各ルールを構築する（interior mutabilityを使わないための設計）。
pub fn all(dat: &DatFile) -> Vec<Box<dyn Rule>> {
    let (size_x, size_y, layouts) = resolve_dims(dat);
    vec![
        Box::new(PreludeDebugRule),
        Box::new(TypeWaytypeRule),
        Box::new(ObsoleteKeywordRule),
        Box::new(DimsRule),
        Box::new(CursorIconRule {
            policy: CursorIconPolicy::Building {
                types_without_build_menu: TYPES_WITHOUT_BUILD_MENU,
            },
        }),
        Box::new(TileImageRule {
            size_x,
            size_y,
            layouts,
        }),
        Box::new(DateIndexOverflowRule),
        Box::new(BooleanStyleFieldRule),
        Box::new(NameAndCopyrightStringFieldRule),
        Box::new(CapacityNarrowIntRule),
    ]
}

/// `tests/building.rs`専用。本番と同じ`RuleSet::for_obj_type`経由でディスパッチする
/// （`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_building(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("building", dat, dat_dir)
}

struct PreludeDebugRule;
impl Rule for PreludeDebugRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let type_name = ctx.dat.get_lower("type");
        let waytype = ctx.dat.get_lower("waytype");
        vec![
            Diagnostic::debug(
                DiagnosticCode::ParsedPairs,
                t!(ctx.language,
                    ja: "{n} 個のkey=valueを読み込み",
                    en: "Loaded {n} key=value pair(s)",
                    n = ctx.dat.pairs.len(),
                ),
            ),
            Diagnostic::debug(
                DiagnosticCode::RawTypeWaytype,
                format!("type=\"{type_name}\" waytype=\"{waytype}\""),
            ),
        ]
    }
}

struct TypeWaytypeRule;
impl Rule for TypeWaytypeRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let type_name = ctx.dat.get_lower("type");
        let waytype = ctx.dat.get_lower("waytype");
        let mut diags = Vec::new();
        check_type_and_waytype(&type_name, &waytype, &mut diags, ctx.language);
        diags
    }
}

fn check_type_and_waytype(
    type_name: &str,
    waytype: &str,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    if OBSOLETE_TYPES.contains(&type_name) {
        diags.push(Diagnostic::error(
            DiagnosticCode::ObsoleteType,
            t!(lang,
                ja: "type={type_name} は obsolete です。stop/extension と waytype を使ってください",
                en: "type={type_name} is obsolete. Use stop/extension with waytype instead",
                type_name = type_name,
            ),
        ));
        return;
    }
    if !KNOWN_TYPES.contains(&type_name) {
        diags.push(Diagnostic::error(
            DiagnosticCode::UnknownType,
            t!(lang,
                ja: "type={type_name} は makeobj が認識できない値です（FATAL ERRORになります）",
                en: "type={type_name} is not a value makeobj recognizes (this becomes a FATAL ERROR)",
                type_name = type_name,
            ),
        ));
        return;
    }

    if TYPES_REQUIRING_WAYTYPE.contains(&type_name) {
        if waytype.is_empty() {
            diags.push(Diagnostic::error(
                DiagnosticCode::MissingWaytype,
                t!(lang,
                    ja: "type={type_name} では waytype が必須です（未指定だとmakeobjがFATAL ERRORになります）",
                    en: "waytype is required when type={type_name} (omitting it makes makeobj FATAL ERROR)",
                    type_name = type_name,
                ),
            ));
        } else if !KNOWN_WAYTYPES.contains(&waytype) {
            diags.push(Diagnostic::error(
                DiagnosticCode::UnknownWaytype,
                t!(lang,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            ));
        } else {
            diags.push(Diagnostic::info(
                DiagnosticCode::TypeWaytypeOk,
                format!("type={type_name} waytype={waytype}"),
            ));
        }
    } else if type_name == "extension" {
        if waytype.is_empty() {
            diags.push(Diagnostic::warning(
                DiagnosticCode::GenericExtension,
                t!(lang,
                    ja: "type=extension で waytype 未指定は「全waytypeに適合する汎用拡張」として解釈されます。意図的でなければ waytype を指定してください",
                    en: "type=extension without waytype is interpreted as a \"generic extension \
                         that fits any waytype\". Specify waytype unless this is intentional",
                ),
            ));
        } else if !KNOWN_WAYTYPES.contains(&waytype) {
            diags.push(Diagnostic::error(
                DiagnosticCode::UnknownWaytype,
                t!(lang,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            ));
        } else {
            diags.push(Diagnostic::info(
                DiagnosticCode::TypeWaytypeOk,
                format!("type={type_name} waytype={waytype}"),
            ));
        }
    } else {
        diags.push(Diagnostic::info(
            DiagnosticCode::TypeWaytypeOk,
            format!("type={type_name}"),
        ));
    }
}

struct ObsoleteKeywordRule;
impl Rule for ObsoleteKeywordRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        if ctx.dat.get("extension_building").is_some() {
            vec![Diagnostic::error(
                DiagnosticCode::ObsoleteKeyword,
                t!(ctx.language,
                    ja: "extension_building は obsolete です。type=stop/extension と waytype を使ってください",
                    en: "extension_building is obsolete. Use type=stop/extension with waytype instead",
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

// 第14弾: `resolve_dims`/`DimsRule`/`CursorIconRule`/`TileImageRule`はfactory.rsと
// ほぼ同一実装だったため、`super::common`へ1本化した（common.rs内のコメント参照）。
// このモジュールからは`use`（冒頭）経由でそのまま利用する。

/// `building_writer.cc:227-236`: intro_date/retire_date/preservation_dateの3つの
/// 日付インデックスがそれぞれ`year*12+month-1`で計算されuint16に無条件代入される。
/// 根拠・設計は`common::check_date_index_overflow_field`のdocコメント参照
/// （`PowerGearMismatchRule`と同種の静的解析ルール）。buildingはこの計算を3回
/// （intro/retire/preservation）行う唯一のobj種別である。
struct DateIndexOverflowRule;
impl Rule for DateIndexOverflowRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        diags.extend(check_date_index_overflow_field(
            dat,
            "intro_year",
            1900,
            Some("intro_month"),
            ctx.language,
        ));
        diags.extend(check_date_index_overflow_field(
            dat,
            "retire_year",
            2999,
            Some("retire_month"),
            ctx.language,
        ));
        diags.extend(check_date_index_overflow_field(
            dat,
            "preservation_year",
            2999,
            Some("preservation_month"),
            ctx.language,
        ));
        diags
    }
}

/// `building_writer.cc:244-247,365`: `capacity`はローカル変数こそ`sint32`
/// （`sint32 capacity = obj.get_int("capacity", level * 32);`）だが、実際の書き込みは
/// `node.write_uint16(fp, capacity);`であり、`sint32`→`uint16`という符号・幅の両方が
/// 変わるnarrowingが発生する。`obj.get_int()`自体は範囲チェック無しの無条件
/// フォールバックで、`get_int_clamped`ではない。`station_capacity`という別名キーも
/// 同じ`capacity`変数へ読み込まれる（`capacity == level*32`のときのみ、
/// building_writer.cc:245-246）が、こちらは`capacity`が未指定のときのみ評価される
/// 経路であり、`capacity=`が明示指定されていれば`station_capacity`は評価されない。
/// このルールは`capacity=`が明示指定された場合のみを検出する（`level`の値は
/// `type`分岐に依存する複雑な計算のため、`capacity`未指定時のデフォルト値
/// `level*32`自体の検証は行わない。デフォルト値は常に安全な範囲に収まる前提で
/// `0`を渡す）。根拠・設計は`common::check_narrow_int_overflow_field`の
/// docコメント参照（`DateIndexOverflowRule`と同種の静的解析ルール）。
struct CapacityNarrowIntRule;
impl Rule for CapacityNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        check_narrow_int_overflow_field(ctx.dat, "capacity", 0, 16, false, ctx.language)
    }
}

/// `building_writer.cc:112-114,203,207,210,213`: `noinfo`/`noconstruction`/
/// `needs_ground`/`extension_building`/`enables_pax`/`enables_post`/`enables_ware`は
/// いずれも`obj.get_int(key, 0) > 0`という比較でフラグ化されるだけで、1以外の正の値
/// （例: `NoInfo=999`）も1と全く同じ扱いになる。makeobjにとってはバグではない
/// （">0"の意図通りに動作する）が、`.dat`記述者が「0/1のフラグ」のつもりで
/// 999のような値を書いてしまった入力ミスの可能性が高い。機能的なバグではないため
/// warning（style note）に留め、"既に正しく動いている"ことを明記する。
const BOOLEAN_STYLE_FIELDS: &[&str] = &[
    "noinfo",
    "noconstruction",
    "needs_ground",
    "extension_building",
    "enables_pax",
    "enables_post",
    "enables_ware",
];
struct BooleanStyleFieldRule;
impl Rule for BooleanStyleFieldRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        for key in BOOLEAN_STYLE_FIELDS {
            let Some(raw) = dat.get(key) else { continue };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = trimmed.parse::<i64>() else {
                continue;
            };
            if value != 0 && value != 1 {
                diags.push(Diagnostic::warning(
                    DiagnosticCode::BooleanStyleFieldNotZeroOrOne,
                    t!(ctx.language,
                        ja: "{key}={value} は0/1以外の値です。building_writer.ccは\
                             `obj.get_int(\"{key}\", 0) > 0`という比較でフラグ化するため、\
                             1以外の正の値も1と全く同じに動作します（機能的な不具合ではありません）。\
                             0か1のつもりで書いた値であれば、意図を明確にするため0か1に修正することを\
                             推奨します",
                        en: "{key}={value} is a value other than 0 or 1. building_writer.cc converts \
                             this to a flag via `obj.get_int(\"{key}\", 0) > 0`, so any positive value \
                             other than 1 behaves identically to 1 (this is not a functional bug). If \
                             0 or 1 was intended, consider changing it to 0 or 1 to make the intent \
                             clear",
                        key = key,
                        value = value,
                    ),
                ));
            }
        }
        diags
    }
}
