//! `obj=cursor`（マウスカーソルのスキン画像定義。`.dat`に実際に書く`obj=`の値は
//! `cursor`）の検証ルール。検証根拠は`rules/mod.rs`冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/skin_writer.cc` / `skin_writer.h` /
//! `imagelist_writer.cc` / `image_writer.cc` / `obj_writer.cc` / `dataobj/tabfile.cc`）を
//! 直接読んで確認した。OTRP側の個別diffはまだ行っていない（building以外のobj種別と
//! 同様）。
//!
//! ## `obj=`文字列について
//!
//! `cursorskin_writer_t::get_type_name()`（skin_writer.h:56-67）は`return "cursor";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ（menuと同一パターン）。
//! さらに実際の公開`.dat`ファイル（GitHub code search、`"obj=cursor" extension:dat`）でも
//! `Obj=cursor`が使われていることを確認した
//! （例: `simutrans/pak128:base/misc_GUI/mouse.dat`、
//! `simutrans/pak128:base/misc_GUI/builder_cursor.dat`、
//! `simutrans/pak128:base/misc_GUI/generaltools.dat`、
//! `aburch/simutrans-pak128.britain:gui/gui128/new_cursor.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/UI/192/Cursors.dat`）。
//! **注意**: `obj=cursor`（このトップレベルobj種別）と、building/way/bridge等
//! 多くのobj種別が持つ`cursor=`/`icon=`**フィールド**（ビルドメニューのカーソル画像
//! 指定）は全くの別概念であり、同じ単語を使うだけで意味的なつながりは無い。
//!
//! ## `skin_writer_t` / `cursorskin_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`と同じく、`cursorskin_writer_t`（skin_writer.h:56-67）も共通の
//! 基底クラス`skin_writer_t`（skin_writer.h:18-29）のサブクラスであり、
//! `get_type()`/`get_type_name()`の2つのオーバーライドのみを持つ（`write_obj`は
//! 一切オーバーライドしない）。よって`cursor`の実際の書き込みロジックは全て基底
//! `skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものであり、`menu`と
//! **完全に同一**である。共有アーキテクチャの詳細（`write_obj`のコード引用、
//! `obj.get`のNULL/空文字列挙動、`"-"`センチネル、name/copyright、
//! waytype/cursor/icon不在、`"> "`ズーム不可プレフィックス、count不一致警告の
//! 到達不能性など）は`menu.rs`冒頭のdoc comment参照（本モジュールでも`menu.rs`の
//! 調査結果を鵜呑みにせず独立にskin_writer.h/skin_writer.ccを読み直して確認済み）。
//!
//! cursor固有の追加確認事項:
//! - **`"> "`（ズーム不可フラグ）構文の実例確認**: 実際の公開`.dat`
//!   （`simutrans/pak128:base/misc_GUI/mouse.dat`の`Image[0]=> mouse.1.0`）で
//!   cursorスキンでもこの構文が使われていることを確認した（menuマイルストーンで
//!   `common::check_image_ref`側に既に実装済みの`strip_zoomable_prefix`がそのまま
//!   cursorにも適用される。cursor固有の追加対応は不要）。
//!
//! `cursorskin_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`と**挙動上完全に同一**であることを確認した（`get_type()`/
//! `get_type_name()`の返り値が異なるだけ）。よって本モジュールのルールは
//! `menu.rs`の`AllImagesRule`と同一のロジックを、obj_type文字列とコメントの
//! 言い回しのみ差し替えて採用する。REJECTEDの理由も全てmenu.rsと同一
//! （詳細はmenu.rs参照）。

use super::common;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::Rule;
use std::path::Path;

/// ルール実装本体は`menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6種別で
/// 共有される`common::AllImagesRule`（skin_writer_t::write_objそのもの、根拠は
/// 上記コメント参照）。
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![Box::new(common::AllImagesRule)]
}

/// `tests/cursor_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_cursor(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("cursor", dat, dat_dir)
}
