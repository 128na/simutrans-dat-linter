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

use super::common::DIR_CODES;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// vehicle_writer.cc:26-53 get_engine_type() がSTRICMPで受理する既知値。
/// これ以外の値（typo・空文字含む）は fatal/error なしで初期値 diesel に
/// 静かにフォールバックする。
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
    ]
}

/// `check_building`と対称的な薄いラッパー。`dat_dir`は現時点のvehicleルールでは
/// 未使用だが、building同様のAPI形状を保ち、将来vehicle画像のファイル存在確認
/// （このマイルストーンでは非対象、rules/mod.rsのREADME参照）を追加する際に
/// シグネチャ変更が不要になるようにしている。
pub fn check_vehicle(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext { dat, dat_dir };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
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
        super::common::check_waytype_field(ctx.dat, "waytype")
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
        let waytype = ctx.dat.get("waytype").unwrap_or("").to_ascii_lowercase();
        if waytype == "electrified_track" {
            return vec![Diagnostic::debug(
                "engine-type-skipped",
                "waytype=electrified_track のため engine_type は読まれません（無条件 electric）",
            )];
        }

        let engine_type = ctx
            .dat
            .get("engine_type")
            .unwrap_or("")
            .to_ascii_lowercase();
        if !engine_type.is_empty() && !KNOWN_ENGINE_TYPES.contains(&engine_type.as_str()) {
            vec![Diagnostic::warning(
                "unknown-engine-type",
                format!(
                    "engine_type={engine_type} は不明な値です。makeobjはfatal/errorを出さず、\
                     黙って diesel にフォールバックします"
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
                "incomplete-8-direction-images",
                format!(
                    "n/e/ne/nwのいずれかの方向画像(emptyimage)が定義されているのに、\
                     連続して定義された方向が{empty_count}個しかありません。\
                     8方向は全て揃っているか、4方向以下で止めるかのどちらかが必要です \
                     （makeobjはFATAL ERRORになります）"
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
                    "freightimage-count-mismatch",
                    format!(
                        "非indexedのfreightimage[<dir>]が{old_style_count}個定義されていますが、\
                         emptyimageは{empty_count}個です。両者は完全一致している必要があります \
                         （makeobjはFATAL ERRORになります）"
                    ),
                ));
            }
        } else {
            for dir in DIR_CODES.iter().take(empty_count) {
                for n in 0..ft {
                    let key = format!("freightimage[{n}][{dir}]");
                    if dat.get(&key).map(str::is_empty).unwrap_or(true) {
                        diags.push(Diagnostic::error(
                            "missing-indexed-freightimage",
                            format!(
                                "{key} が未指定です。freightimage[0][s]が定義されている\
                                 （indexed形式）ため、emptyimageが定義された全方向×全freight\
                                 typeの組み合わせでfreightimageが必須です（makeobjはFATAL \
                                 ERRORになります）"
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
                    "missing-freightimagetype",
                    format!(
                        "{key} が未指定です。freight_image_type={ft}個のindexed freightimageが\
                         使われているため、各indexに対応するfreightimagetype[i]（goodへのxref）\
                         が必須です（makeobjはFATAL ERRORになります）"
                    ),
                ));
            }
        }

        let extra_key = format!("freightimagetype[{ft}]");
        if dat.get(&extra_key).map(|v| !v.is_empty()).unwrap_or(false) {
            diags.push(Diagnostic::warning(
                "extra-freightimagetype",
                format!(
                    "{extra_key} は使用範囲(0..{ft})より1つ多いindexです。\
                     makeobjはFATALにはしませんが警告を出します（超過定義）"
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
                "power-gear-mismatch",
                format!(
                    "power={power} を宣言していますが gear={gear_raw} は変換後 \
                     (gear*64/100={gear_transformed}) になり、編成内でのこの車両の\
                     実効出力寄与が常に0になります（simconvoi.cc: sum_gear_and_power \
                     += get_power() * get_gear()）。makeobjはこれを検証しません"
                ),
            )]
        } else {
            Vec::new()
        }
    }
}
