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
//! **完全に同一**である。本モジュールでは`menu.rs`の調査結果を鵜呑みにせず、
//! 上記の通りskin_writer.h/skin_writer.ccを実際に読み直して独立に確認した:
//!
//! ```text
//! // skin_writer_t::write_obj(fp, parent, obj)  [オーバーロード1、実際にobj_writer.cc経由で呼ばれる方]
//! for (int i = 0; ; i++) {
//!     sprintf(buf, "image[%d]", i);
//!     std::string str = obj.get(buf);
//!     if (str.empty()) break;                 // 画像走査を終了
//!     keys.append(str);
//! }
//! write_obj(fp, parent, obj, keys);            // オーバーロード2へ委譲
//!
//! // skin_writer_t::write_obj(fp, parent, obj, imagekeys)  [オーバーロード2]
//! write_name_and_copyright(fp, node, obj);     // name, copyright
//! imagelist_writer_t::instance()->write_obj(fp, node, imagekeys);
//! node.check_and_write_header(fp);
//! ```
//!
//! - `obj.get(...)`（`tabfileobj_t::get`, tabfile.cc:48-56）はキー欠落時に
//!   **NULLではなく空文字列**を返す。`str.empty()`は「キーが実際に存在しないか、
//!   値として空文字列が書かれている」場合のみtrueになる。`i`は`0`始まりで
//!   無制限（`for (int i = 0; ; i++)`）に走査され、最初に欠落した`image[i]`で
//!   走査全体が終了する（menuと全く同じ、単一の1次元配列の走査）。
//! - `"-"`（画像なしセンチネル、image_writer.cc:343の仕様コメント参照）は
//!   空文字列ではないため、`image[N]=-`と書けばそのNでの走査は継続する
//!   （menuと同じ扱い）。
//! - `name`/`copyright`は`write_name_and_copyright`（obj_writer.cc:62-70）経由で
//!   `text_writer_t::write_obj`（text_writer.cc:12-23）に渡るが、どちらも空文字列を
//!   無条件に許容しfatal/warningを出さない（menuと同じ）。
//! - `cursorskin_writer_t`は`waytype`/`cursor`/`icon`のいずれも読まない
//!   （`skin_writer_t::write_obj`/`skin_writer.h`全文にこれらへの言及なし）。
//!   マウスカーソル自体を構成するスキン画像であり、ビルドメニューに「載る」対象
//!   （building等）ではないため、cursor/iconという（フィールドとしての）概念自体が
//!   無い（crossing/ground_obj/tree/citycar/pedestrian/menuと同じパターン）。
//! - 個々の`image[i]`キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-514、`imagelist_writer_t`
//!   経由）がファイルの存在・サイズ（128の倍数か）を検証する。これは他の全obj種別と
//!   共有の`common::check_image_ref`でカバーする。
//!   **`"> "`（ズーム不可フラグ）構文の実例確認**: 実際の公開`.dat`
//!   （`simutrans/pak128:base/misc_GUI/mouse.dat`の`Image[0]=> mouse.1.0`）で
//!   cursorスキンでもこの構文が使われていることを確認した（menuマイルストーンで
//!   `common::check_image_ref`側に既に実装済みの`strip_zoomable_prefix`がそのまま
//!   cursorにも適用される。cursor固有の追加対応は不要）。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告について検討したが、menuと全く同じ
//!   理由（`keys`は空文字列でない値のみでappendされ、`image_writer_t::write_obj`は
//!   空文字列/`"-"`に対して早期returnせず最後まで実行してcountをインクリメントする）
//!   で到達する実行経路が無い。
//!
//! ## menuとの比較結論
//!
//! `cursorskin_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`と**挙動上完全に同一**であることを確認した（`get_type()`/
//! `get_type_name()`の返り値が異なるだけ）。よって本モジュールのルールは
//! `menu.rs`の`AllImagesRule`と同一のロジックを、obj_type文字列とコメントの
//! 言い回しのみ差し替えて採用する。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった。menuと同一の理由）:
//! - `name`/`copyright`未指定チェック: good/sound/ground/menuと全く同じ理由
//!   （`obj_writer_t::write_name_and_copyright`とtext_writer_t::write_objは
//!   空文字列を無条件許容し、fatal/warningを出さない）。
//! - 画像0枚（`image[0]`未指定）の警告: `keys`が空のままループが1回で終了するだけで、
//!   fatal/warningの分岐は無い（menuと同種の判断）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、cursorの`keys`は空文字列を
//!   含まない状態で構築され、`image_writer_t::write_obj`が空文字列に対して
//!   countをスキップすることも無いため、`count < keys.get_count()`に到達する
//!   実行経路が無い（menuと同じ理由）。
//! - `image[i]`の途中欠落（"歯抜け"）自体の検出: これはmakeobjの実際の仕様どおりの
//!   動作であり（`str.empty()`で即座に走査打ち切り）、fatal/warningのソース側分岐も
//!   無い（menuと同じ理由）。
//! - `waytype`/`cursor`/`icon`（フィールドとしての）関連の検証: good/tree/menuと
//!   同じ理由（skin_writer.h/skin_writer.cc全文にこれらのフィールドへの言及が
//!   一つも無い）。

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
