//! `obj=good` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/good_writer.cc` / `good_writer.h` /
//! `obj_writer.cc` / `text_writer.cc` / `dataobj/tabfile.cc`）を直接読んで確認した。
//! OTRP側の個別diffはまだ行っていない（building以外のobj種別と同様）。
//!
//! `good_writer_t::write_obj`（good_writer.cc:15-31）は building/vehicle/way と異なり、
//! フィールド読み取りに分岐が一切無い最小の実装である:
//!
//! ```text
//! write_name_and_copyright(fp, node, obj);                          // name, copyright
//! text_writer_t::instance()->write_obj(fp, node, obj.get("metric")); // metric
//! node.write_version(fp, 4);
//! node.write_sint64(fp, obj.get_int64("value", 0));
//! node.write_uint8 (fp, obj.get_int("catg", 0));
//! node.write_uint16(fp, obj.get_int("speed_bonus", 0));
//! node.write_uint16(fp, obj.get_int("weight_per_unit", 100));
//! node.write_uint8 (fp, obj.get_int("mapcolor", 255));
//! ```
//!
//! `obj.get(...)`（`tabfileobj_t::get`, tabfile.cc:48-56）と `obj.get_string(...)`
//! （tabfile.cc:63-71）はキー欠落時に空文字列／`def`を返すだけで fatal/warning を
//! 出さない。`obj.get_int(...)` / `obj.get_int64(...)`（tabfile.cc:183-198,
//! 221-234）も同様に `def` へ静かにフォールバックするだけで、`get_int_clamped()`
//! （tabfile.cc:201-212、wayのclip_belowで使われている警告付きクランプ関数）は
//! 一切呼ばれていない。つまり value/catg/speed_bonus/weight_per_unit/mapcolor の
//! どれも「欠落」「範囲外」を検出してfatal/warningにする分岐がmakeobjソース上に
//! 存在しない。
//!
//! `obj_writer_t::write_name_and_copyright`（obj_writer.cc:62-70）と
//! `text_writer_t::write_obj`（text_writer.cc:12-23）も、`text`がNULLなら
//! 空文字列に置き換えるだけで fatal/warning を出さない。したがって `name` が
//! 未指定（空文字列）でも good_writer自体はエラーにしない
//! （building/vehicle/wayのwaytype必須チェックのような対称的なルールは作れない）。
//!
//! `good` は building/vehicle/way と異なり `waytype` を一切読まない
//! （good_writer.cc全文にwaytypeへの参照が無い）ため、`common::KNOWN_WAYTYPES`
//! （`get_waytype()`のFATALパターン）はgoodには適用されない。同様にcursor/icon・
//! 画像フィールドも一切無い（good_writer.cc/good_writer.hに`image`/`cursor`/`icon`
//! への言及なし）ため、`common::check_image_ref`も適用対象が無い。
//!
//! REJECTED（候補として検討したが根拠不十分のため実装しなかった）:
//! - `name` 未指定チェック: 空文字列のままtext_writerへ渡り、fatal/warning無しで
//!   無名のgoodオブジェクトが生成されるだけ（obj_writer.cc:62-70,
//!   text_writer.cc:12-23）。buildingのcursor/icon省略のような「ビルドメニューに
//!   表示されない」式の実機観察に基づく根拠が無く、単に「無名でも動く」だけなので
//!   見送り。
//! - `catg`（貨物カテゴリ）の範囲チェック: `goods_desc.cc`の`catg_names[32]`配列は
//!   `catg & 31`でアクセスするため8bit値の上位ビットが暗黙にマスクされるが、これは
//!   ゲーム実行時（goods_reader_t::read_node以降）の話であり、makeobj自体はfatal/
//!   warning無しで`get_int("catg", 0)`の生値をuint8として書き込むだけ
//!   （good_writer.cc:25）。makeobj時点の検証根拠が無いため見送り。
//! - `value` / `speed_bonus` / `weight_per_unit` / `mapcolor` の妥当性検証:
//!   いずれも`get_int`/`get_int64`で無条件に読み、欠落時は0/0/100/255への
//!   サイレントフォールバックのみ（get_int_clampedではない）。vehicleの
//!   weight/speedチェックやwayのtopspeed/max_weight/axle_loadチェックが
//!   見送られたのと同じ理由（fatal/warning分岐がmakeobjソース上に無い）。
//! - `metric`（単位文字列）の未指定チェック: `text_writer_t::write_obj`は空文字列を
//!   無条件に許容し、fatal/warningを出さない（text_writer.cc:12-23）。
//! - freightとしてgoodを参照するvehicle側（`freight=`）や、freightimagetype経由の
//!   xref解決: このマイルストーンは`good` `.dat`自身の内部妥当性検証のみが対象であり、
//!   他obj種別からの被参照解決はスコープ外（vehicleのfreight xref未解決と同じ理由、
//!   `xref_writer_t::write_obj`はこの参照を検証せずゲーム読み込み時まで遅延する）。

use super::common::{NameAndCopyrightStringFieldRule, check_narrow_int_overflow_field};
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// `good_writer.cc`自体にmakeobj時点でfatal/warningになる分岐は一つも無いが、
/// name/copyright（`NameAndCopyrightStringFieldRule`、全obj種別共通）と、
/// `catg`/`speed_bonus`/`mapcolor`（`NarrowIntOverflowRule`、下記docコメント参照）は
/// このobj種別にも適用できる。
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(NameAndCopyrightStringFieldRule),
        Box::new(NarrowIntOverflowRule),
    ]
}

/// `good_writer.cc:25,26,28`: `catg`（uint8）・`speed_bonus`（uint16）・
/// `mapcolor`（uint8）はいずれも`obj.get_int(key, def)`（範囲チェック無しの
/// 無条件フォールバック）で読まれた後、対応する`node.write_uint8`/`write_uint16`へ
/// 無条件に代入される。`weight_per_unit`（uint16、line 27）も同じ`get_int`経由だが、
/// デフォルト100・実務上の値域が狭く、範囲外を指定する動機が乏しいため対象外とした
/// （`catg`/`mapcolor`はuint8という特に狭い型で、255を超える値を書きやすい実務上の
/// リスクがあるためこちらを優先した）。根拠・設計は
/// `common::check_narrow_int_overflow_field`のdocコメント参照
/// （`date-index-overflow`と同種の静的解析ルール）。
struct NarrowIntOverflowRule;
impl Rule for NarrowIntOverflowRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "catg",
            0,
            8,
            false,
            ctx.language,
        ));
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "speed_bonus",
            0,
            16,
            false,
            ctx.language,
        ));
        diags.extend(check_narrow_int_overflow_field(
            dat,
            "mapcolor",
            255,
            8,
            false,
            ctx.language,
        ));
        diags
    }
}

/// `tests/good_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_good(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("good", dat, dat_dir)
}
