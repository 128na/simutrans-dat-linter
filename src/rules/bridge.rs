//! `obj=bridge` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/bridge_writer.cc` / `get_waytype.cc` /
//! `imagelist_writer.cc` / `image_writer.cc` / `dataobj/tabfile.cc`）を直接読んで
//! 確認した。OTRP側の個別diffはまだ行っていない（vehicle/way/goodと同様）。
//!
//! `bridge_writer_t::write_obj`（bridge_writer.cc:99-163）は building/vehicle/way/good
//! と異なり、数値フィールドのほぼ全て（`pillar_distance` / `pillar_asymmetric` /
//! `max_lenght` / `max_length` / `max_height` / `axle_load` / `clip_below` /
//! `intro_year` / `intro_month` / `retire_year` / `retire_month`）が
//! `tabfileobj_t::get_int_clamped()`（tabfile.cc:201-212、範囲外だと
//! `dbg->warning("tabfileobj_t::get_int_clamped()", "Value %d for key %s out of
//! range %d..%d, resetting to %d", ...)` を出して黙って範囲内にクランプする）
//! 経由で読まれる。`topspeed`（`get_int`）・`cost`/`maintenance`（`get_int64`）は
//! wayのtopspeed/max_weight/axle_loadと同じく無条件フォールバックのみで対象外。
//!
//! 画像は`write_bridge_images`（bridge_writer.cc:20-96）が
//! `back{name}[{index}]` / `front{name}[{index}]`（season付きの場合は末尾に
//! `[{season}]`が付く）という24種類のキー（image/start/ramp/pillarの通常季節・
//! 雪季節2版、各方向）を機械的に走査する。front側の値が`value.size() <= 2`
//! （空文字列 or "-" などの2文字以下）だと
//! `dbg->warning(obj_writer_t::last_name, "No %s specified (might still work)", keybuf)`
//! を出す（fatalではない。"might still work"という文言通り、コメントアウト的运用も
//! 許容される）。back側にはこの警告チェックが無い（front側のみ）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `cost`/`maintenance`（`get_int64`）の妥当性検証: 無条件フォールバックのみで
//!   `get_int_clamped`ではない（bridge_writer.cc:103-104）。wayのtopspeed/
//!   max_weight/axle_loadが見送られたのと同じ理由。`topspeed`
//!   （`get_int("topspeed", 999)`）自体は`node.write_uint16(outfp, topspeed)`と
//!   いう狭い型へ無条件代入されるため、`TopspeedNarrowIntRule`（静的解析層、
//!   `date-index-overflow`と同種）の対象にはした（fatal/warning根拠が無い点は
//!   変わらないが、狭い型への切り詰めという別の問題を検出する）。
//! - `max_lenght`（歴史的スペルミス）と`max_length`（正しいスペル）の二重キー
//!   挙動そのものの警告: `max_length = get_int_clamped("max_length", max_length, ...)`
//!   は`max_lenght`で読んだ値をdefaultとして`max_length`が存在すれば上書きする、
//!   という後方互換のための意図的な設計（bridge_writer.cc:107-108）。これは
//!   `dbg->warning`/`dbg->fatal`の分岐ではなく、両方指定時に「後勝ち」で
//!   `max_length`が使われるだけなので、`duplicate-key`的な意図しない上書きとは
//!   別物（tabfileobj_t側の重複キー検出ではなく、C++コード自身が意図して二重に
//!   読んでいる）。makeobj時点でのfatal/warning根拠が無いため見送り。
//! - `intro_year`/`retire_year`の`get_int_clamped(..., 0, INT32_MAX)`という
//!   極めて広い範囲（事実上、負値以外はまずクランプされない）: 技術的には
//!   `ClampedRangeRule`と同じ仕組みで検出可能だが、負の年という通常のtypoでは
//!   まず起こらない入力のみが対象になり、他のclampedフィールド
//!   （pillar_distance等）ほど実務的に踏みやすい範囲ではない。判断に迷ったが、
//!   `get_int_clamped`呼び出しである以上は根拠自体は明確なため、
//!   `ClampedRangeRule`に含めて実装した（見送りリストには入れない）。
//! - back側画像の`value.size() <= 2`警告: `write_bridge_images`の警告分岐は
//!   front側の`frontkeys.append(value)`直後にのみ存在し、back側
//!   （`backkeys.append(value)`）には対応する警告コードが無い
//!   （bridge_writer.cc:56-75、backは`imagelist_writer_t::write_obj`の
//!   count不一致警告のみが理論上ありうるが、backkeysは常に全24件appendされる
//!   ため`imagelist_writer.cc:28`の`count < keys.get_count()`には該当しない）。
//!   front側のみを検出対象にする。
//! - cursor/icon未指定チェック: `write_bridge_images`はseason<=0のときのみ
//!   `cursorskin_writer_t::instance()->write_obj`を呼ぶが、これは`skin_writer.cc`の
//!   `write_name_and_copyright` + `imagelist_writer_t::write_obj`を経由するだけで、
//!   cursor/iconが空でもfatal/warningを出さない（wayのcursor/iconが見送られた
//!   のと同じ理由）。

use super::common::{
    NameAndCopyrightStringFieldRule, check_image_ref, check_narrow_int_overflow_field,
};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::t;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// bridge_writer.cc:25-43 の`names`配列そのもの。keyname -> keyindex群。
/// NULL終端を`&[]`終端として素朴に再現する。
const IMAGE_GROUPS: &[(&str, &[&str])] = &[
    ("image", &["ns", "ew"]),
    ("start", &["n", "s", "e", "w"]),
    ("ramp", &["n", "s", "e", "w"]),
    ("pillar", &["s", "w"]),
    ("image2", &["ns", "ew"]),
    ("start2", &["n", "s", "e", "w"]),
    ("ramp2", &["n", "s", "e", "w"]),
    ("pillar2", &["s", "w"]),
];

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypeRequiredRule),
        Box::new(ClampedRangeRule),
        Box::new(FrontImageWarningRule),
        Box::new(NameAndCopyrightStringFieldRule),
        Box::new(TopspeedNarrowIntRule),
    ]
}

/// `tests/bridge_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_bridge(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("bridge", dat, dat_dir)
}

/// bridge_writer.cc:101 は get_waytype(obj.get("waytype")) を無条件に呼ぶ
/// （vehicle/wayと同じく分岐なしで常に評価される）。get_waytype.cc:14-49はSTRICMPが
/// 既知13種のいずれにも一致しなければ dbg->fatal("get_waytype()","invalid
/// waytype \"%s\"\n", waytype) で落とす。tabfileobj_t::get()はNULLを返さず
/// 欠落キーには空文字列を返す（tabfile.cc:48-56）ため、waytype未指定も同じ
/// fatalパスに入る。実際のチェックロジックは`common::check_waytype_field`に
/// 集約されている（way/bridge/tunnel/roadsign/vehicle/way-object/crossingで共有）。
struct WaytypeRequiredRule;
impl Rule for WaytypeRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        super::common::check_waytype_field(ctx.dat, "waytype", ctx.language)
    }
}

/// tabfile.cc:201-212 get_int_clamped(key, def, min, max): 値がmin..max範囲外だと
/// `dbg->warning("tabfileobj_t::get_int_clamped()", "Value %d for key %s out of
/// range %d..%d, resetting to %d", ...)` を出して値をクランプする（FATALにはしない）。
/// bridge_writer.cc:105-115 はこの関数を7つのキーに対して呼ぶ（`max_lenght`/
/// `max_length`は同じ範囲0..255を共有するペアとして扱う）。
///
/// 第19弾: 実際のクランプ判定ロジックは`way.rs`の`ClipBelowRangeRule`と共通化し、
/// `super::common::check_clamped_int_field`へ1本化した（common.rs内のdocコメント
/// 参照）。このRuleはbridge固有の複数フィールド一覧（`CLAMPED_FIELDS`）を
/// ループで回すだけの薄いラッパーになった。
struct ClampedRangeRule;

struct ClampedField {
    key: &'static str,
    min: i64,
    max: i64,
}

const CLAMPED_FIELDS: &[ClampedField] = &[
    ClampedField {
        key: "pillar_distance",
        min: 0,
        max: u8::MAX as i64,
    },
    ClampedField {
        key: "pillar_asymmetric",
        min: 0,
        max: 1,
    },
    ClampedField {
        key: "max_lenght",
        min: 0,
        max: u8::MAX as i64,
    },
    ClampedField {
        key: "max_length",
        min: 0,
        max: u8::MAX as i64,
    },
    ClampedField {
        key: "max_height",
        min: 0,
        max: u8::MAX as i64,
    },
    ClampedField {
        key: "axle_load",
        min: 0,
        max: u16::MAX as i64,
    },
    ClampedField {
        key: "clip_below",
        min: 0,
        max: 1,
    },
    ClampedField {
        key: "intro_year",
        min: 0,
        max: i32::MAX as i64,
    },
    ClampedField {
        key: "intro_month",
        min: 1,
        max: 12,
    },
    ClampedField {
        key: "retire_year",
        min: 0,
        max: i32::MAX as i64,
    },
    ClampedField {
        key: "retire_month",
        min: 1,
        max: 12,
    },
];

impl Rule for ClampedRangeRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        for field in CLAMPED_FIELDS {
            diags.extend(super::common::check_clamped_int_field(
                ctx.dat,
                field.key,
                field.min,
                field.max,
                DiagnosticCode::ClampedValueOutOfRange,
                ctx.language,
            ));
        }
        diags
    }
}

/// bridge_writer.cc:34-70,157-159: `backimage[ns][0]`が非空なら季節ありと判定し
/// `backimage[ns][1]`, `backimage[ns][2]`の非空判定でnumber_of_seasons(最大2)を
/// 決めてseason=0..number_of_seasons を1回ずつ`write_bridge_images`する。空なら
/// season=-1で1回だけ呼ぶ（bridge_writer.cc:136-160）。
fn season_range(dat: &DatFile) -> Vec<Option<u8>> {
    let season0 = dat.get("backimage[ns][0]").unwrap_or("");
    if season0.is_empty() {
        return vec![None];
    }
    let mut number_of_seasons: u8 = 0;
    while number_of_seasons < 2 {
        let key = format!("backimage[ns][{}]", number_of_seasons + 1);
        let present = dat.get(&key).map(|v| !v.is_empty()).unwrap_or(false);
        if present {
            number_of_seasons += 1;
        } else {
            break;
        }
    }
    (0..=number_of_seasons).map(Some).collect()
}

/// bridge_writer.cc:49-80: 24種類の front{keyname}[{keyindex}]（+季節ありなら
/// 末尾に[{season}]）を走査し、値が2文字以下（空文字列や"-"含む）なら
/// `dbg->warning(obj_writer_t::last_name, "No %s specified (might still work)",
/// keybuf)`。fatalではないためwarning止まり。値が実際に画像を指す場合は
/// 参照ファイルの存在・サイズも確認する（common::check_image_ref、
/// image_writer.cc の block_load/write_obj が根拠）。
struct FrontImageWarningRule;
impl Rule for FrontImageWarningRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        for season in season_range(dat) {
            for (keyname, indices) in IMAGE_GROUPS {
                for keyindex in *indices {
                    let key = match season {
                        Some(s) => format!("front{keyname}[{keyindex}][{s}]"),
                        None => format!("front{keyname}[{keyindex}]"),
                    };
                    let value = dat.get(&key).unwrap_or("");
                    if value.len() <= 2 {
                        diags.push(Diagnostic::warning(
                            DiagnosticCode::NoBridgeImageSpecified,
                            t!(ctx.language,
                                ja: "{key} が未指定です（\"No {key} specified (might still work)\"）。\
                                     makeobjはFATALにはしませんが警告を出します",
                                en: "{key} is not specified (\"No {key} specified (might still work)\"). \
                                     makeobj does not treat this as FATAL, but warns",
                                key = key,
                            ),
                        ));
                    } else {
                        check_image_ref(
                            value,
                            ctx.dat_dir,
                            &key,
                            &mut diags,
                            ctx.language,
                            ctx.tile_size,
                        );
                    }
                }
            }
        }

        diags
    }
}

/// `bridge_writer.cc:102,120`: `topspeed`は`obj.get_int("topspeed", 999)`
/// （範囲チェック無しの無条件フォールバック、`get_int_clamped`ではない）で
/// 読まれた後、`node.write_uint16(outfp, topspeed)`という狭い型へ無条件代入される。
/// 根拠・設計は`common::check_narrow_int_overflow_field`のdocコメント参照
/// （`ClampedRangeRule`が対象とする他の数値フィールドとは異なり`get_int_clamped`
/// 経由ではないため、静的解析層のこちらのルールで別途カバーする）。
struct TopspeedNarrowIntRule;
impl Rule for TopspeedNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        check_narrow_int_overflow_field(ctx.dat, "topspeed", 999, 16, false, ctx.language)
    }
}
