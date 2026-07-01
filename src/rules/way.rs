//! `obj=way` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/way_writer.cc` / `get_waytype.cc` /
//! `dataobj/tabfile.cc`）を直接読んで確認した。OTRP側の個別diffはまだ行っていない
//! （vehicle側と同様、「vanilla/OTRPで一致確認済み」とは言えない状態）。
//!
//! REJECTED（cursor/icon以外の候補、根拠不十分のため実装しなかった）:
//! - `image[new2]`（switch images判定用プローブ）・`imageup[...]`/`imageup2[...]`
//!   （坂道画像）・`diagonal[...]`（対角画像）の欠落: いずれも
//!   imagelist_writer_t::write_obj経由で空文字列のキーがそのまま「空画像」として
//!   書かれるだけで、fatal/warningの分岐が無い（`image[-]`/`image[-][0]`だけが
//!   明示的にdbg->fatalされる特別扱い。way_writer.cc:98-154参照）。
//! - `topspeed` / `max_weight` / `axle_load` の妥当性検証: いずれも
//!   `obj.get_int(...)`で無条件に読み、範囲外や欠落値へのfatal/warningが無い
//!   （欠落時は999/999/9999にサイレントフォールバックするのみ。way_writer.cc:39-41）。
//!   vehicleのweight/speedチェックが見送られたのと同じ理由
//!   （「意図的な省略」と「入力ミス」を区別する根拠がmakeobjソース上に無い）。

use super::common::{KNOWN_WAYTYPES, check_image_ref};
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypeRequiredRule),
        Box::new(BaseImageRequiredRule),
        Box::new(ClipBelowRangeRule),
    ]
}

/// `check_building`/`check_vehicle`と対称的な薄いラッパー。
pub fn check_way(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext { dat, dat_dir };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
}

/// way_writer.cc:51 は get_waytype(obj.get("waytype")) を無条件に呼ぶ
/// （vehicleと同じく分岐なしで常に評価される）。get_waytype.cc:14-49はSTRICMPが
/// 既知13種のいずれにも一致しなければ dbg->fatal("get_waytype()","invalid
/// waytype \"%s\"\n", waytype) で落とす。tabfileobj_t::get()はNULLを返さず
/// 欠落キーには空文字列を返す（tabfile.cc:48-56）ため、waytype未指定も同じ
/// fatalパスに入る。
struct WaytypeRequiredRule;
impl Rule for WaytypeRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let waytype = ctx.dat.get("waytype").unwrap_or("").to_ascii_lowercase();
        if waytype.is_empty() {
            vec![Diagnostic::error(
                "missing-waytype",
                "obj=way では waytype が必須です（get_waytype()は空文字列もFATAL ERRORにします）",
            )]
        } else if !KNOWN_WAYTYPES.contains(&waytype.as_str()) {
            vec![Diagnostic::error(
                "unknown-waytype",
                format!("waytype={waytype} は不正な値です（FATAL ERRORになります）"),
            )]
        } else {
            vec![Diagnostic::info("waytype-ok", format!("waytype={waytype}"))]
        }
    }
}

/// way_writer.cc:84-96: まず `image[-][0]`（冬季season 0の直進画像）を読み、
/// 空なら「冬季画像なし」分岐に入って `image[-]`（season無し版）を読む。
/// これも空だった場合のみ
/// `dbg->fatal("way_writer_t::write_obj", "image with label %s missing", buf)`
/// になる（`image[-][0]`が非空なら「冬季画像あり」分岐に入り、この fatal 自体を
/// 通らない）。つまり `image[-]` と `image[-][0]` のどちらか一方でも定義されていれば
/// 良く、両方欠落したときのみ FATAL ERROR になる。
struct BaseImageRequiredRule;
impl Rule for BaseImageRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let no_season = ctx.dat.get("image[-]").unwrap_or("");
        let season0 = ctx.dat.get("image[-][0]").unwrap_or("");
        if no_season.is_empty() && season0.is_empty() {
            return vec![Diagnostic::error(
                "missing-base-image",
                "image[-] （直進画像）が未指定です。image[-][0]（冬季season 0版）も未指定のため、\
                 makeobjはFATAL ERRORになります（\"image with label image[-] missing\"）",
            )];
        }

        let mut diags = vec![Diagnostic::info(
            "base-image-ok",
            if !season0.is_empty() {
                "image[-][0] が定義されています（冬季画像あり分岐）".to_string()
            } else {
                "image[-] が定義されています（冬季画像なし分岐）".to_string()
            },
        )];
        // 冬季画像なし分岐ではimage[-]が実際に読まれる画像なので、存在確認とサイズ確認を行う
        // （image[-][0]分岐ではimage[-]は評価されない。way_writer.cc:88-96参照）。
        if season0.is_empty() && !no_season.is_empty() {
            check_image_ref(no_season, ctx.dat_dir, "image[-]", &mut diags);
        }
        diags
    }
}

// REJECTED: cursor/icon 未指定チェック。way_writer.cc:149-150/214-215は
// cursorskin_writer_t::write_obj 経由で cursor/icon をそのまま imagelist に渡すが、
// image_writer_t::write_obj は空文字列/"-" を「画像なし」として無条件に許容し
// fatal/warning を出さない（image_writer.cc:366, imagelist_writer.cc内でも
// count mismatch は発生しない）。building.rsのCursorIconRuleは「ビルドメニューに
// 表示されない」という実機観察に基づくが、wayのcursor/iconはツールバー上のカーソル・
// アイコンであり、buildingと同じUI表示保証がmakeobjソース上からは確認できない。
// 別UIでの表示有無を推測で断定しないため、このルールは追加しない。

/// tabfile.cc:201-212 get_int_clamped(key, def, min, max): 値がmin..max範囲外だと
/// `dbg->warning("tabfileobj_t::get_int_clamped()", "Value %d for key %s out of
/// range %d..%d, resetting to %d", ...)` を出して値をクランプする（FATALにはしない）。
/// way_writer.cc:42 は `obj.get_int_clamped("clip_below", 1, 0, 1)` と呼ぶため、
/// clip_below は 0 か 1 以外を指定すると警告付きで黙って 0 か 1 にクランプされる。
struct ClipBelowRangeRule;
impl Rule for ClipBelowRangeRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let Some(raw) = ctx.dat.get("clip_below") else {
            return Vec::new();
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        // tabfileobj_t::get_int() は strtol(value, NULL, 0) を使う（tabfile.cc:183-198）。
        // Rustのi64::from_str_radixほど厳密ではないが、10進の妥当な数値かどうかの
        // 判定にはstr::parseで十分近似できる。パース不能な値はstrtolが0を返すため、
        // その場合も範囲外(0..1に収まる)として扱われクランプは発生しない。
        let Ok(value) = trimmed.parse::<i64>() else {
            return Vec::new();
        };
        if !(0..=1).contains(&value) {
            vec![Diagnostic::warning(
                "clip-below-out-of-range",
                format!(
                    "clip_below={value} は範囲0..1外です。makeobjはFATALにはしませんが警告を出し、\
                     値を0か1にクランプします（tabfileobj_t::get_int_clamped()）"
                ),
            )]
        } else {
            Vec::new()
        }
    }
}
