//! `obj=symbol`（ビルドメニューの各種シンボルアイコン・ロゴ等、UIスキン画像定義。
//! `.dat`に実際に書く`obj=`の値は`symbol`）の検証ルール。検証根拠は
//! `rules/mod.rs`冒頭コメント参照。
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
//! `symbolskin_writer_t::get_type_name()`（skin_writer.h:73）は`return "symbol";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ（menu/cursorと同一
//! パターン）。さらに実際の公開`.dat`ファイル（GitHub code search、
//! `"obj=symbol" extension:dat`）でも`Obj=symbol`が使われていることを確認した
//! （例: `simutrans/pak128:base/misc_GUI/builder_symbol.dat`（`Obj=symbol` /
//! `Image[0]=builder_symbol.1.0`）、`simutrans/pak128:base/special/BigLogo.dat`、
//! `aburch/simutrans:themes.src/pak64german/files/sim-symbols.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/UI/64/symbol.dat`、
//! `VictorErik/Pak128.Sweden-Ex:Base/misc_GUI_64/symbols-64.dat`）。
//!
//! ## `skin_writer_t` / `symbolskin_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`と同じく、`symbolskin_writer_t`
//! （skin_writer.h:62-74）も共通の基底クラス`skin_writer_t`（skin_writer.h:18-29）の
//! サブクラスであり、`get_type()`/`get_type_name()`の2つのオーバーライドのみを持つ
//! （`write_obj`は一切オーバーライドしない）。よって`symbol`の実際の書き込みロジックは
//! 全て基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものであり、
//! `menu`/`cursor`と**完全に同一**である。共有アーキテクチャの詳細（`write_obj`の
//! コード引用、`obj.get`のNULL/空文字列挙動、`"-"`センチネル、name/copyright、
//! waytype/cursor/icon不在、`"> "`ズーム不可プレフィックス、count不一致警告の
//! 到達不能性など）は`menu.rs`冒頭のdoc comment参照（本モジュールでも`menu.rs`/
//! `cursor.rs`の調査結果を鵜呑みにせず独立にskin_writer.h/skin_writer.ccを
//! 読み直して確認済み）。
//!
//! symbol固有の追加確認事項:
//! - **`"> "`（ズーム不可フラグ）構文について**: 実際の公開`.dat`
//!   （`VictorErik/Pak128.Sweden-Ex:Base/misc_GUI_64/symbols-64.dat`等、symbol系
//!   スキンでも`>`プレフィックス構文が使われる実例が確認できるgui系ディレクトリに
//!   配置されている）ことから、symbolスキンでもこの構文が使われうる前提で
//!   `common::check_image_ref`側の`strip_zoomable_prefix_and_trim`をそのまま
//!   再利用する（symbol固有の追加対応は不要、cursorと同様）。
//!
//! `symbolskin_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`と**挙動上完全に同一**であることを
//! 確認した（`get_type()`/`get_type_name()`の返り値が異なるだけ）。よって
//! 本モジュールのルールは`menu.rs`/`cursor.rs`の`AllImagesRule`と同一のロジックを、
//! obj_type文字列とコメントの言い回しのみ差し替えて採用する。REJECTEDの理由も
//! 全てmenu.rs/cursor.rsと同一（詳細はmenu.rs参照）。

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

/// `tests/symbol_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_symbol(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("symbol", dat, dat_dir)
}
