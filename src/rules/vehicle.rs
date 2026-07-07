//! `obj=vehicle` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/vehicle_writer.cc` / `xref_writer.cc` /
//! `get_waytype.cc`）を直接読んで確認した。OTRP側の個別diffはまだ行っていない
//! （building側のように「vanilla/OTRPで一致確認済み」とは言えない状態）。
//!
//! `PowerGearMismatchRule`のみ根拠の種類が異なる: 他の全ルールは
//! makeobj自体（コンパイル時、`descriptor/writer/`）の`dbg->fatal`/`dbg->warning`を
//! 根拠とするが、このルールはゲームエンジンのランタイムコード（`src/simutrans/simconvoi.cc`）
//! を根拠とする「静的解析」層のルールである（`couplings`サブコマンドと同種の位置づけ）。
//! makeobj自身はこのフィールドの組み合わせを一切検証しない。
//!
//! `NarrowIntFieldsRule`（`payload`/`speed`/`axle_load`/`length`）も同じ「静的解析」層
//! のルールで、`DateIndexOverflowRule`と同種（common.rsの`check_narrow_int_overflow_field`
//! 参照）。`NameAndCopyrightStringFieldRule`はobj種別を問わず共有される
//! `name`/`copyright`フィールドの検証（common.rs参照）。

use super::common::{
    DIR_CODES, NameAndCopyrightStringFieldRule, check_date_index_overflow_field,
    check_narrow_int_overflow_field,
};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::t;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// vehicle_writer.cc:26-53 get_engine_type() がSTRICMPで受理する既知値。
/// これ以外の値（typo・空文字含む）は fatal/error なしで初期値 diesel に
/// 静かにフォールバックする。
///
/// REJECTED（第6弾で再調査、"none"を追加しないと判断）: pak128実データの
/// `lint`実行で`engine_type=none`が`unknown-engine-type`警告として161件検出され、
/// 「無動力車両（貨車等）の正当な慣習値では」という仮説が立ったため、
/// `vehicle_writer.cc`（`get_engine_type()`, 26-53行目）と`vehicle_desc.h`
/// （`enum engine_t`, 51-61行目）を再確認した。結果:
/// - `enum engine_t`は`{unknown=-1, steam, diesel, electric, bio, sail,
///   fuel_cell, hydrogene, battery}`の9値のみで、`none`に相当する値は存在しない
/// - `get_engine_type()`のSTRICMP if-elseチェーンにも`"none"`の分岐は無く、
///   一致しない場合は関数冒頭の`uv8 = vehicle_desc_t::diesel;`にそのまま
///   フォールバックする（本ルールの警告文言通りの実際の挙動）
/// - `engine_type`フィールド自体は`write_obj`内で無条件に書き込まれる
///   （`waytype=electrified_track`の場合のみ`electric`に強制される特別扱いが
///   あるが、`power=0`の無動力車両を`engine_type`の検証から除外する分岐は無い）
/// - `"none"`という文字列自体は`get_waytype.cc:18`（`waytype=none`）と
///   `vehicle_writer.cc:277,295`（`constraint[prev/next]=none`）で特別扱いされる
///   既知の慣習語であり、pak128の記述者が`engine_type`にも同じ「none」慣習を
///   誤って転用した可能性が高い
///
/// 結論: `engine_type=none`は実際のmakeobjでも`diesel`へ静かにフォールバックする
/// **本物の**問題（記述者の意図した値と異なる値が使われている）であり、
/// linterの誤検知ではない。よって`KNOWN_ENGINE_TYPES`に`"none"`は追加しない。
const KNOWN_ENGINE_TYPES: &[&str] = &[
    "diesel",
    "electric",
    "steam",
    "bio",
    "sail",
    "fuel_cell",
    "hydrogene",
    "battery",
    "unknown",
];

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypeRequiredRule),
        Box::new(EngineTypeRule),
        Box::new(DirectionImageRule),
        Box::new(FreightImageTypeRule),
        Box::new(PowerGearMismatchRule),
        Box::new(DateIndexOverflowRule),
        Box::new(NameAndCopyrightStringFieldRule),
        Box::new(NarrowIntFieldsRule),
    ]
}

/// `tests/vehicle_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
/// `dat_dir`は現時点のvehicleルールでは未使用だが、building同様のAPI形状を保ち、
/// 将来vehicle画像のファイル存在確認（このマイルストーンでは非対象、
/// rules/mod.rsのREADME参照）を追加する際にシグネチャ変更が不要になるようにしている。
pub fn check_vehicle(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("vehicle", dat, dat_dir)
}

/// vehicle_writer.cc:146-147 は get_waytype(obj.get("waytype")) を無条件に呼ぶ
/// （buildingと異なりtypeによる分岐が無い）。get_waytype.cc:14-49はSTRICMPが
/// 既知12種のいずれにも一致しなければ dbg->fatal("get_waytype()","invalid
/// waytype \"%s\"\n", waytype) で落とす。tabfileobj_t::get()はNULLを返さず
/// 欠落キーには空文字列を返す（tabfile.h:148）ため、waytype未指定も同じ
/// fatalパスに入る。実際のチェックロジックは`common::check_waytype_field`に
/// 集約されている（way/bridge/tunnel/roadsign/vehicle/way-object/crossingで共有）。
struct WaytypeRequiredRule;
impl Rule for WaytypeRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        super::common::check_waytype_field(ctx.dat, "waytype", ctx.language)
    }
}

/// vehicle_writer.cc:26-53 get_engine_type(): 既知値以外は fatal/error なしで
/// 初期値 diesel に静かにフォールバックする。vehicle_writer.cc:155-158で
/// waytype=electrified_track の場合は engine_type を無条件 electric にし、
/// このフィールド自体読まれないためチェック対象外にする。
///
/// engine_type が完全に未指定（空文字列）のケースは意図的にこのルールの対象外とする:
/// 無動力車両（貨車等）ではengine_type省略が一般的な慣習であり、これを毎回警告すると
/// 実際のミスではないケースでノイズになる。typoなど「値は書いたが不正」なケースのみ
/// 検出する（waytype省略時とは異なり、こちらは既存の慣習との衝突を避けるための
/// 意図的なスコープ限定）。
struct EngineTypeRule;
impl Rule for EngineTypeRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let waytype = ctx.dat.get_lower("waytype");
        if waytype == "electrified_track" {
            return vec![Diagnostic::debug(
                DiagnosticCode::EngineTypeSkipped,
                t!(ctx.language,
                    ja: "waytype=electrified_track のため engine_type は読まれません（無条件 electric）",
                    en: "engine_type is not read because waytype=electrified_track \
                         (unconditionally electric)",
                ),
            )];
        }

        let engine_type = ctx
            .dat
            .get("engine_type")
            .unwrap_or("")
            .to_ascii_lowercase();
        if !engine_type.is_empty() && !KNOWN_ENGINE_TYPES.contains(&engine_type.as_str()) {
            vec![Diagnostic::warning(
                DiagnosticCode::UnknownEngineType,
                t!(ctx.language,
                    ja: "engine_type={engine_type} は不明な値です。makeobjはfatal/errorを出さず、\
                         黙って diesel にフォールバックします",
                    en: "engine_type={engine_type} is an unknown value. makeobj does not emit \
                         fatal/error, but silently falls back to diesel",
                    engine_type = engine_type,
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

/// vehicle_writer.cc:191-199 freightimage[N][s] を N=0..127 でプローブし、
/// 最初に空だったNが freight_image_type。全て定義されていれば0のまま
/// （C++の初期値未更新の挙動を厳密にミラーする）。
fn freight_image_type(dat: &DatFile) -> usize {
    let mut freight_image_type = 0;
    for i in 0..127 {
        let key = format!("freightimage[{i}][s]");
        let is_empty = dat.get(&key).map(str::is_empty).unwrap_or(true);
        if is_empty {
            freight_image_type = i;
            break;
        }
    }
    freight_image_type
}

/// vehicle_writer.cc:202-218 emptyimage[dir_codes[i]] を i=0..8 で順に読み、
/// 最初に欠落した方向でループを打ち切る（後方に飛び番不可）。
/// has_8_images は index>=4 のいずれかが実際にappendされたら true になる。
fn empty_image_state(dat: &DatFile) -> (usize, bool) {
    let mut count = 0;
    let mut has_8_images = false;
    for (i, dir) in DIR_CODES.iter().enumerate() {
        let key = format!("emptyimage[{dir}]");
        let present = dat.get(&key).map(|v| !v.is_empty()).unwrap_or(false);
        if present {
            count += 1;
            if i >= 4 {
                has_8_images = true;
            }
        } else {
            break;
        }
    }
    (count, has_8_images)
}

/// vehicle_writer.cc:219-247 の方向別画像完全性チェック3種をまとめたルール:
/// - 8方向画像の不完全（4-7いずれか定義済みなのに8未満）: FATAL (line 242-244)
/// - 非indexed freightimageの個数不一致: FATAL (line 245-247)
/// - indexed freightimage[N][dir]の欠落: FATAL (line 234)
struct DirectionImageRule;
impl Rule for DirectionImageRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let (empty_count, has_8_images) = empty_image_state(dat);
        let mut diags = Vec::new();

        if has_8_images && empty_count < 8 {
            diags.push(Diagnostic::error(
                DiagnosticCode::Incomplete8DirectionImages,
                t!(ctx.language,
                    ja: "n/e/ne/nwのいずれかの方向画像(emptyimage)が定義されているのに、\
                         連続して定義された方向が{empty_count}個しかありません。\
                         8方向は全て揃っているか、4方向以下で止めるかのどちらかが必要です \
                         （makeobjはFATAL ERRORになります）",
                    en: "One of the n/e/ne/nw direction images (emptyimage) is defined, but only \
                         {empty_count} consecutive direction(s) are defined. You must either \
                         define all 8 directions or stop at 4 or fewer \
                         (makeobj treats this as a FATAL ERROR)",
                    empty_count = empty_count,
                ),
            ));
        }

        let ft = freight_image_type(dat);
        if ft == 0 {
            let mut old_style_count = 0;
            for dir in DIR_CODES.iter().take(empty_count) {
                let key = format!("freightimage[{dir}]");
                if dat.get(&key).map(|v| !v.is_empty()).unwrap_or(false) {
                    old_style_count += 1;
                }
            }
            if old_style_count > 0 && old_style_count != empty_count {
                diags.push(Diagnostic::error(
                    DiagnosticCode::FreightimageCountMismatch,
                    t!(ctx.language,
                        ja: "非indexedのfreightimage[<dir>]が{old_style_count}個定義されていますが、\
                             emptyimageは{empty_count}個です。両者は完全一致している必要があります \
                             （makeobjはFATAL ERRORになります）",
                        en: "{old_style_count} non-indexed freightimage[<dir>] entries are defined, \
                             but emptyimage has {empty_count}. These must match exactly \
                             (makeobj treats this as a FATAL ERROR)",
                        old_style_count = old_style_count,
                        empty_count = empty_count,
                    ),
                ));
            }
        } else {
            for dir in DIR_CODES.iter().take(empty_count) {
                for n in 0..ft {
                    let key = format!("freightimage[{n}][{dir}]");
                    if dat.get(&key).map(str::is_empty).unwrap_or(true) {
                        diags.push(Diagnostic::error(
                            DiagnosticCode::MissingIndexedFreightimage,
                            t!(ctx.language,
                                ja: "{key} が未指定です。freightimage[0][s]が定義されている\
                                     （indexed形式）ため、emptyimageが定義された全方向×全freight\
                                     typeの組み合わせでfreightimageが必須です（makeobjはFATAL \
                                     ERRORになります）",
                                en: "{key} is unspecified. Since freightimage[0][s] is defined \
                                     (indexed form), freightimage is required for every combination \
                                     of direction (where emptyimage is defined) x freight type \
                                     (makeobj treats this as a FATAL ERROR)",
                                key = key,
                            ),
                        ));
                    }
                }
            }
        }

        diags
    }
}

/// vehicle_writer.cc:303-321: freight_image_typeが2以上のとき、各indexの
/// freightimagetype[i]がgoodへのxref（何をこのfreightimage indexが表すか）
/// として必須。i=0..freight_image_typeで欠落はFATAL (line 317-319)。
/// 使用範囲より1つ多い freightimagetype[freight_image_type] の定義は
/// fatalではなくWARNING（line 311-314、freight_image_type=0でも実行される
/// 分岐なので常にチェックする）。
struct FreightImageTypeRule;
impl Rule for FreightImageTypeRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let ft = freight_image_type(dat);
        let mut diags = Vec::new();

        for i in 0..ft {
            let key = format!("freightimagetype[{i}]");
            if dat.get(&key).map(str::is_empty).unwrap_or(true) {
                diags.push(Diagnostic::error(
                    DiagnosticCode::MissingFreightimagetype,
                    t!(ctx.language,
                        ja: "{key} が未指定です。freight_image_type={ft}個のindexed freightimageが\
                             使われているため、各indexに対応するfreightimagetype[i]（goodへのxref）\
                             が必須です（makeobjはFATAL ERRORになります）",
                        en: "{key} is unspecified. Since {ft} indexed freightimage entries are \
                             used, freightimagetype[i] (an xref to a good) is required for each \
                             index (makeobj treats this as a FATAL ERROR)",
                        key = key,
                        ft = ft,
                    ),
                ));
            }
        }

        let extra_key = format!("freightimagetype[{ft}]");
        if dat.get(&extra_key).map(|v| !v.is_empty()).unwrap_or(false) {
            diags.push(Diagnostic::warning(
                DiagnosticCode::ExtraFreightimagetype,
                t!(ctx.language,
                    ja: "{extra_key} は使用範囲(0..{ft})より1つ多いindexです。\
                         makeobjはFATALにはしませんが警告を出します（超過定義）",
                    en: "{extra_key} is one index beyond the used range (0..{ft}). makeobj does \
                         not treat this as FATAL, but warns (excess definition)",
                    extra_key = extra_key,
                    ft = ft,
                ),
            ));
        }

        diags
    }
}

/// 根拠: `vehicle_writer.cc:142` `uint16 gear = (obj.get_int("gear", 100) * 64) / 100;`
/// （整数除算）。`simconvoi.cc`（例: 1698, 1704, 1755, 1763, 2365行目）で
/// `sum_gear_and_power += info->get_power() * info->get_gear();` として編成全体の
/// 実効出力に積算され、これが`calc_max_speed()`（simconvoi.cc:834-）の`total_power`
/// になる。変換後`gear`が0だと、`power`をいくら宣言していてもその車両の出力寄与は
/// 常に0になる（`power`自体が無視されるわけではなく、`gear`という別フィールドが
/// 出力を無効化する）。整数除算`(raw*64)/100`は`raw`が0または1のとき0になる
/// （`raw=2`なら`(2*64)/100=1`で非ゼロ）。makeobj自体はこの組み合わせを一切
/// 検証しない（`dbg->fatal`/`dbg->warning`なし）。他のルールと異なり、
/// 根拠はコンパイル時のmakeobjソースではなくランタイムコード（`simconvoi.cc`）。
struct PowerGearMismatchRule;
impl Rule for PowerGearMismatchRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let Some(power) = ctx
            .dat
            .get("power")
            .and_then(|v| v.trim().parse::<i64>().ok())
        else {
            return Vec::new();
        };
        if power <= 0 {
            return Vec::new();
        }

        let gear_raw = ctx
            .dat
            .get("gear")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(100);
        let gear_transformed = (gear_raw * 64) / 100;
        if gear_transformed == 0 {
            vec![Diagnostic::warning(
                DiagnosticCode::PowerGearMismatch,
                t!(ctx.language,
                    ja: "power={power} を宣言していますが gear={gear_raw} は変換後 \
                         (gear*64/100={gear_transformed}) になり、編成内でのこの車両の\
                         実効出力寄与が常に0になります（simconvoi.cc: sum_gear_and_power \
                         += get_power() * get_gear()）。makeobjはこれを検証しません",
                    en: "power={power} is declared, but gear={gear_raw} becomes \
                         (gear*64/100={gear_transformed}) after conversion, so this vehicle's \
                         effective power contribution in a convoy is always 0 \
                         (simconvoi.cc: sum_gear_and_power += get_power() * get_gear()). \
                         makeobj does not validate this",
                    power = power,
                    gear_raw = gear_raw,
                    gear_transformed = gear_transformed,
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

/// `vehicle_writer.cc:134,138`: intro_date/retire_dateがそれぞれ`year*12+month-1`で
/// 計算されuint16に無条件代入される。根拠・設計は
/// `common::check_date_index_overflow_field`のdocコメント参照
/// （`PowerGearMismatchRule`と同種の静的解析ルール）。
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
        diags
    }
}

/// vehicle_writer.cc:98,99 / 106,107 / 115,116 / 166,167: `payload`/`speed`/
/// `axle_load`/`length`はいずれも`tabfileobj_t::get_int()`（範囲チェック無しの
/// 無条件フォールバック）で読まれた後、無条件に狭いC++整数型へ代入・書き込み
/// される（`payload`/`speed`/`axle_load`は`uint16`、`length`は`uint8`）。
/// 根拠・設計は`common::check_narrow_int_overflow_field`のdocコメント参照
/// （`DateIndexOverflowRule`/`PowerGearMismatchRule`と同種の静的解析ルール）。
struct NarrowIntFieldsRule;
impl Rule for NarrowIntFieldsRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "payload",
            0,
            16,
            false,
            ctx.language,
        ));
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "speed",
            0,
            16,
            false,
            ctx.language,
        ));
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "axle_load",
            0,
            16,
            false,
            ctx.language,
        ));
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "length",
            8,
            8,
            false,
            ctx.language,
        ));
        diags
    }
}
