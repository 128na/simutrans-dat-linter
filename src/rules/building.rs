//! `obj=building` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! `CapacityNarrowIntRule`（`capacity`）は`DateIndexOverflowRule`と同種の「静的解析」層の
//! ルール。`building_writer.cc:244` `sint32 capacity = obj.get_int("capacity", level * 32);`
//! はローカル変数こそ`sint32`だが、実際の書き込みは`building_writer.cc:365`
//! `node.write_uint16(fp, capacity);`であり、`sint32`→`uint16`という符号・幅の
//! 両方が変わる narrowing が発生する（ローカル変数の型だけを見て「sint32だから
//! 広くて安全」と判断してはいけない、という具体例。`common::check_narrow_int_overflow_field`
//! のdocコメント参照）。
//!
//! 同種のnarrowingは`capacity`以外にも4フィールドあり、それぞれ専用のRuleで検出する。
//! `ChanceNarrowIntRule`（`chance`、uint8）・`AnimationTimeNarrowIntRule`
//! （`animation_time`、uint16）・`AllowUndergroundNarrowIntRule`
//! （`allow_underground`、uint8）は`common::check_narrow_int_overflow_field`をそのまま
//! 呼ぶだけの薄いラッパー。`LevelNarrowIntRule`（`level`）のみ、`obj.get_int()`で読んだ
//! 値から`-1`した式全体がuint16へ代入されるという特殊な計算式（`level=0`のとき
//! アンダーフローする）のため専用実装になっている（各構造体のdocコメント参照）。

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

/// `building_writer.cc:119-201`のif-elseチェーン（STRICMP、大文字小文字を区別しない）が
/// 受理する`type=`の既知値。`""`（未指定）は`*type_name == '\0'`分岐（180行目、`any`と同列）で
/// 受理されるため値として含めている。
///
/// `rules/keys.rs`の`known_values_per_obj_type`が`(building, type)`の既知値一覧として
/// `OBSOLETE_TYPES`と合わせてそのまま再エクスポートする（重複キュレーションしない）。
pub const KNOWN_TYPES: &[&str] = &[
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
/// `building_writer.cc:184-196`でFATAL ERRORになる、makeobjが認識はするが拒否する
/// obsolete値（`ObsoleteType`診断参照）。`known_values_per_obj_type`はこの一覧も
/// `KNOWN_TYPES`と合わせて公開する。シンタックスハイライトの観点では「makeobjが構文として
/// 認識するキーワード」であることに変わりはなく（`lint`が別途`obsolete-type`エラーとして
/// 検出する）、値の妥当性判定とは別の関心事のため含める。
pub const OBSOLETE_TYPES: &[&str] = &[
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
        Box::new(ChanceNarrowIntRule),
        Box::new(AnimationTimeNarrowIntRule),
        Box::new(AllowUndergroundNarrowIntRule),
        Box::new(LevelNarrowIntRule),
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
        check_type_and_waytype(ctx.dat, &type_name, &waytype, &mut diags, ctx.language);
        diags
    }
}

fn check_type_and_waytype(
    dat: &DatFile,
    type_name: &str,
    waytype: &str,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    // `type`/`waytype`の行番号解決ヘルパー。「値は存在するが不正」パターンの
    // 診断（ObsoleteType/UnknownType/UnknownWaytype）にのみ`.at()`を付与する
    // （`key`が非空である呼び出し元でのみ使うため、`line_of`は必ず`Some`を
    // 返すはずだが、`None`の場合でも0行目のような嘘の位置情報は作らない）。
    let at_key = |diag: Diagnostic, key: &str| match dat.line_of(key) {
        Some(line) => diag.at(line, key.to_string()),
        None => diag,
    };

    if OBSOLETE_TYPES.contains(&type_name) {
        let diag = Diagnostic::error(
            DiagnosticCode::ObsoleteType,
            t!(lang,
                ja: "type={type_name} は obsolete です。stop/extension と waytype を使ってください",
                en: "type={type_name} is obsolete. Use stop/extension with waytype instead",
                type_name = type_name,
            ),
        );
        diags.push(at_key(diag, "type"));
        return;
    }
    if !KNOWN_TYPES.contains(&type_name) {
        let diag = Diagnostic::error(
            DiagnosticCode::UnknownType,
            t!(lang,
                ja: "type={type_name} は makeobj が認識できない値です（FATAL ERRORになります）",
                en: "type={type_name} is not a value makeobj recognizes (this becomes a FATAL ERROR)",
                type_name = type_name,
            ),
        );
        diags.push(at_key(diag, "type"));
        return;
    }

    if TYPES_REQUIRING_WAYTYPE.contains(&type_name) {
        if waytype.is_empty() {
            // waytypeキー自体が欠落している（または空文字列の）ケース。
            // `.at()`は呼ばず`location: None`のまま返す。
            diags.push(Diagnostic::error(
                DiagnosticCode::MissingWaytype,
                t!(lang,
                    ja: "type={type_name} では waytype が必須です（未指定だとmakeobjがFATAL ERRORになります）",
                    en: "waytype is required when type={type_name} (omitting it makes makeobj FATAL ERROR)",
                    type_name = type_name,
                ),
            ));
        } else if !KNOWN_WAYTYPES.contains(&waytype) {
            let diag = Diagnostic::error(
                DiagnosticCode::UnknownWaytype,
                t!(lang,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            );
            diags.push(at_key(diag, "waytype"));
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
            let diag = Diagnostic::error(
                DiagnosticCode::UnknownWaytype,
                t!(lang,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            );
            diags.push(at_key(diag, "waytype"));
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

/// `building_writer.cc:203-205`: `if (obj.get_int("extension_building", 0) > 0)`
/// という**値の大小を見た**条件でのみfatalになる。`obj.get_int()`はキー欠落時に
/// `default`（ここでは`0`）を返す（範囲チェック無しの無条件フォールバック、
/// tabfile.cc:183-198）ため、`extension_building`キー自体が存在するかどうか
/// （`.is_some()`）だけを見て判定してはならない。`extension_building=0`は
/// このif文の条件を満たさず、実makeobjは**全く問題なくビルドする**
/// （偽陽性の原因だった旧実装は`dat.get("extension_building").is_some()`のみを
/// 見ており、`extension_building=0`という明示指定でもerrorを出していた）。
/// 値のparseに失敗した場合は`tabfileobj_t::get_int()`が内部で使う`strtol`と同様
/// （`common::check_clamped_int_field`のdocコメント参照）、`0`として扱う
/// （非数値は0扱い、fatalにならない）。
struct ObsoleteKeywordRule;
impl Rule for ObsoleteKeywordRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let Some(raw) = ctx.dat.get("extension_building") else {
            return Vec::new();
        };
        let value = raw.trim().parse::<i64>().unwrap_or(0);
        if value <= 0 {
            return Vec::new();
        }
        let diag = Diagnostic::error(
            DiagnosticCode::ObsoleteKeyword,
            t!(ctx.language,
                ja: "extension_building は obsolete です。type=stop/extension と waytype を使ってください",
                en: "extension_building is obsolete. Use type=stop/extension with waytype instead",
            ),
        );
        // `dat.get("extension_building")`が`Some`を返している以上、
        // `line_of`は必ず`Some`を返す。
        vec![match ctx.dat.line_of("extension_building") {
            Some(line) => diag.at(line, "extension_building"),
            None => diag,
        }]
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

/// `building_writer.cc:223`: `uint8 const chance = obj.get_int("chance", 100);`。
/// `capacity`と同種のnarrowing（`get_int()`は範囲チェック無しの無条件フォールバック、
/// 結果は`uint8`へそのまま代入される。書き込みは`building_writer.cc:361`
/// `node.write_uint8(fp, chance);`）。`common::check_narrow_int_overflow_field`の
/// docコメント参照。
struct ChanceNarrowIntRule;
impl Rule for ChanceNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        check_narrow_int_overflow_field(ctx.dat, "chance", 100, 8, false, ctx.language)
    }
}

/// `building_writer.cc:116`: `uint16 const animation_time = obj.get_int("animation_time", 300);`。
/// `capacity`/`chance`と同種のnarrowing（書き込みは`building_writer.cc:364`
/// `node.write_uint16(fp, animation_time);`）。
struct AnimationTimeNarrowIntRule;
impl Rule for AnimationTimeNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        check_narrow_int_overflow_field(ctx.dat, "animation_time", 300, 16, false, ctx.language)
    }
}

/// `building_writer.cc:259`: `uint8 allow_underground = obj.get_int("allow_underground", 2);`。
/// `capacity`/`chance`/`animation_time`と同種のnarrowing（書き込みは
/// `building_writer.cc:368` `node.write_uint8(fp, allow_underground);`）。
struct AllowUndergroundNarrowIntRule;
impl Rule for AllowUndergroundNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        check_narrow_int_overflow_field(ctx.dat, "allow_underground", 2, 8, false, ctx.language)
    }
}

/// `building_writer.cc:102`: `uint16 level = obj.get_int("level", 1) - 1;`。
/// `capacity`等と同じ「狭いC++整数型への無条件代入」ファミリーだが、`level`は
/// `get_int()`の戻り値へ`-1`する**式全体**が`uint16`のローカル変数へ代入される点が
/// 異なる（`check_narrow_int_overflow_field`は単一フィールド値をそのまま型範囲と
/// 比較する設計のため、この`-1`変換をそのまま流用できず専用ルールとして実装する）。
/// `level=0`（`.dat`記述者が「レベル0」のつもりで書く、あるいはデフォルト値`1`を
/// 明示的に上書きしようとして`0`を書く入力ミス）だと`obj.get_int("level",1)`が`0`を
/// 返し、`0 - 1`という`uint16`への代入で`-1`が2の補数表現の`65535`へ静かに
/// ラップアラウンドする（`level`はその後`++level`（building_writer.cc:219、
/// stop/extension/dock/depot/factoryのみ）や`passengers`による上書き
/// （cur/mon/tow/hq）を経ることもあるが、これらは`level`が既に`65535`に
/// 汚染された後に作用するだけで問題を悪化させこそすれ解消はしない）。
/// `level`が未指定の場合はデフォルト値`1`（`get_int`のデフォルト引数）を使う。
struct LevelNarrowIntRule;
impl Rule for LevelNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let Some(raw) = dat.get("level") else {
            return Vec::new();
        };
        let level_raw = raw.trim().parse::<i64>().unwrap_or(0);
        let value = level_raw - 1;
        if (0..=65535).contains(&value) {
            return Vec::new();
        }
        let diag = Diagnostic::warning(
            DiagnosticCode::NarrowIntOverflow,
            t!(ctx.language,
                ja: "level={level_raw}（building_writer.ccの計算式`obj.get_int(\"level\", 1) - 1`\
                     により{value}）は格納先のunsigned 16bit整数の範囲(0..65535)外です。\
                     makeobjはこの計算結果を範囲チェック無しにuint16へ無条件代入するため、\
                     範囲外の値は2の補数による切り詰めで全く無関係な値へ静かに変わります\
                     （makeobj自体はこれを検証しません）",
                en: "level={level_raw} (which building_writer.cc's formula \
                     `obj.get_int(\"level\", 1) - 1` turns into {value}) is outside the range \
                     (0..65535) of the unsigned 16-bit integer it is stored in. makeobj \
                     unconditionally assigns this computed value into a uint16 with no range \
                     check, so an out-of-range value silently changes into an unrelated value \
                     via two's-complement truncation (makeobj itself does not validate this)",
                level_raw = level_raw,
                value = value,
            ),
        );
        vec![match dat.line_of("level") {
            Some(line) => diag.at(line, "level".to_string()),
            None => diag,
        }]
    }
}

/// `building_writer.cc:112-114,207,210,213`: `noinfo`/`noconstruction`/
/// `needs_ground`/`enables_pax`/`enables_post`/`enables_ware`は
/// いずれも`obj.get_int(key, 0) > 0`という比較でフラグ化されるだけで、1以外の正の値
/// （例: `NoInfo=999`）も1と全く同じ扱いになる。makeobjにとってはバグではない
/// （">0"の意図通りに動作する）が、`.dat`記述者が「0/1のフラグ」のつもりで
/// 999のような値を書いてしまった入力ミスの可能性が高い。機能的なバグではないため
/// warning（style note）に留め、"既に正しく動いている"ことを明記する。
///
/// `extension_building`はこの一覧に**含めない**: 同じ`obj.get_int(key, 0) > 0`という
/// 比較式こそ使うものの（building_writer.cc:203）、`true`になった場合の帰結が
/// この6フィールドとは全く異なる。`extension_building`は`> 0`ならその場で
/// `dbg->fatal("extension_building is obsolete keyword for %s; ...")`となり、単なる
/// 「1と同じ扱いになるだけ」のスタイルノートでは済まない（`ObsoleteKeywordRule`が
/// この特別扱いを別のerror診断として検出する）。
const BOOLEAN_STYLE_FIELDS: &[&str] = &[
    "noinfo",
    "noconstruction",
    "needs_ground",
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
                let diag = Diagnostic::warning(
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
                );
                // `dat.get(key)`が`Some`を返している（早期returnを通過済み）ため、
                // `key`は必ずパーサに登録済みで`line_of`は`Some`を返す。
                diags.push(match dat.line_of(key) {
                    Some(line) => diag.at(line, key.to_string()),
                    None => diag,
                });
            }
        }
        diags
    }
}
