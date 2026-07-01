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

use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// `good_writer.cc`にはmakeobj時点でfatal/warningになる分岐が一つも無いため、
/// 現時点でこのVecは空。obj=goodを登録すること自体の価値は、obj種別を問わず
/// 動作する`check_duplicate_keys`（`rules/mod.rs`経由でmain.rsから常時実行）を
/// good datにも適用できるようにする点にある。
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![]
}

/// `check_building`/`check_vehicle`/`check_way`と対称的な薄いラッパー。
pub fn check_good(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext { dat, dat_dir };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
}
