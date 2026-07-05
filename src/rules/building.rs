//! `obj=building` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。

use super::common::{
    CursorIconPolicy, CursorIconRule, DimsRule, KNOWN_WAYTYPES, TileImageRule, resolve_dims,
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
