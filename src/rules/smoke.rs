//! `obj=smoke`（車両・工場が排出する煙エフェクトのスキン画像定義。`.dat`に実際に書く
//! `obj=`の値は`smoke`。この`obj=smoke`（トップレベルobj種別）と、`obj=factory`が
//! 持つ`smoketile=`/`smokeoffset=`/`smoke=`**フィールド**（工場のどのタイル・座標から
//! この煙obj参照を発生させるかの指定）は全くの別概念であり、同じ単語を使うだけで
//! 意味的なつながりは無い。事前合意の14種計画には含まれておらず、soundマイルストーンでの
//! `descriptor/writer/`配下の機械的な棚卸しで発見された7種の新規obj種別のうち5件目に
//! 対応した1件）の検証ルール。検証根拠は`rules/mod.rs`冒頭コメント参照。
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
//! `smoke_writer_t::get_type_name()`（skin_writer.h:77-89）は`return "smoke";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ（menu/cursor/symbolと
//! 同一パターン）。さらに実際の公開`.dat`ファイル（GitHub code search、
//! `"obj=smoke" extension:dat`）でも`Obj=smoke`が使われていることを確認した
//! （例: `simutrans/pak128:base/smokes/Smokes.dat`（`Obj=smoke` /
//! `Name=Diesel` / `Image[0]=misc-smoke-128.0.0`など5フェーズの連番画像）、
//! `aburch/simutrans-pak128.britain:smokes/smoke.dat`、
//! `VictorErik/Pak128.Sweden-Ex:Base/Smokes/smokes.dat`、
//! `Cousjava/simutrans-paks:pak128.German/smoke/Steam.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/vehicles/smoke/smoke_vehicles.dat`）。
//! **注意**: `obj=smoke`（このトップレベルobj種別）と、`obj=factory`が持つ
//! `smoketile[N]=`/`smokeoffset[N]=`/`smoke=`**フィールド**（工場のどのタイル・座標
//! から煙を発生させ、この`obj=smoke`をどう参照するかの指定。`src/rules/factory.rs`の
//! `SmokeOffsetRule`が検証対象）は全くの別概念であり、同じ単語を使うだけで意味的な
//! つながりは無い（`factory_writer.cc`は`smoketile[N]`/`smokeoffset[N]`を読むだけで、
//! `skin_writer.cc`/`skin_writer.h`とは無関係）。
//!
//! ## `skin_writer_t` / `smoke_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`と同じく、
//! `smoke_writer_t`（skin_writer.h:77-89）も共通の基底クラス`skin_writer_t`
//! （skin_writer.h:18-29）のサブクラスであり、`get_type()`/`get_type_name()`の2つの
//! オーバーライドのみを持つ（`write_obj`は一切オーバーライドしない）。よって`smoke`の
//! 実際の書き込みロジックは全て基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）
//! そのものであり、`menu`/`cursor`/`symbol`と**完全に同一**である。「smoke」という
//! 名前が視覚エフェクトのタイミング（アニメーション周期・フェード等）に関する特別な
//! 処理を示唆する可能性を考慮し、本モジュールでは`menu.rs`/`cursor.rs`/`symbol.rs`の
//! 調査結果を鵜呑みにせず、上記の通りskin_writer.h/skin_writer.ccを実際に読み直して
//! 独立に確認した。`smoke_writer_t`のクラス定義（skin_writer.h:77-89）には
//! `get_type()`/`get_type_name()`以外のメンバー関数は一切無く、`write_obj`の
//! オーバーライドは無い。`skin_writer.cc`全文（18-52行、ファイル末尾）にも
//! `smoke_writer_t`固有の分岐は存在しない（クラス名で分岐する`switch`/`if`文も
//! 皆無）:
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
//! 「煙のアニメーション周期・フェードタイミング」に相当する処理（`smokeuplift`/
//! `smokelifetime`等）は`.dat`のこの`obj=smoke`側ではなく、`obj=factory`側の
//! フィールド（`factory_writer.cc`が読む`smokeuplift`/`smokelifetime`、
//! `FACTORY_NAMED_POST_BUILDING`に既に含まれる）およびゲーム実行時の
//! `smoke_desc_t`（本ツールの対象外、makeobjではなくゲームエンジン側のロジック）に
//! あり、`obj=smoke`自体は単に連番画像（アニメーションフレーム列）を保持する
//! スキン画像リストに過ぎないことが確認できた。
//!
//! - `obj.get(...)`（`tabfileobj_t::get`, tabfile.cc:48-56）はキー欠落時に
//!   **NULLではなく空文字列**を返す。`str.empty()`は「キーが実際に存在しないか、
//!   値として空文字列が書かれている」場合のみtrueになる。`i`は`0`始まりで
//!   無制限（`for (int i = 0; ; i++)`）に走査され、最初に欠落した`image[i]`で
//!   走査全体が終了する（menu/cursor/symbolと全く同じ、単一の1次元配列の走査）。
//! - `"-"`（画像なしセンチネル、image_writer.cc:343の仕様コメント参照）は
//!   空文字列ではないため、`image[N]=-`と書けばそのNでの走査は継続する
//!   （menu/cursor/symbolと同じ扱い）。
//! - `name`/`copyright`は`write_name_and_copyright`（obj_writer.cc:62-70）経由で
//!   `text_writer_t::write_obj`（text_writer.cc:12-23）に渡るが、どちらも空文字列を
//!   無条件に許容しfatal/warningを出さない（menu/cursor/symbolと同じ）。
//! - `smoke_writer_t`は`waytype`/`cursor`/`icon`のいずれも読まない
//!   （`skin_writer_t::write_obj`/`skin_writer.h`全文にこれらへの言及なし）。
//!   煙エフェクト自体を構成するスキン画像（アニメーションフレーム列）であり、
//!   ビルドメニューに「載る」対象（building等）ではないため、cursor/iconという
//!   （フィールドとしての）概念自体が無い（crossing/ground_obj/tree/citycar/
//!   pedestrian/menu/cursor/symbolと同じパターン）。
//! - 個々の`image[i]`キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-514、`imagelist_writer_t`
//!   経由）がファイルの存在・サイズ（128の倍数か）を検証する。これは他の全obj種別と
//!   共有の`common::check_image_ref`でカバーする。
//!   **`"> "`（ズーム不可フラグ）構文について**: この処理はobj種別中立の共通ロジック
//!   （image_writer.cc:356-364）であり、menuマイルストーンで`common::check_image_ref`
//!   側に`strip_zoomable_prefix_and_trim`として既に実装済みのため、smoke固有の
//!   追加対応は不要（cursor/symbolと同様）。実際の公開`.dat`のsmoke系ファイルに
//!   `>`プレフィックス構文を使う実例はGitHub code search（`"obj=smoke" "Image[0]=>"
//!   extension:dat`）では見つからなかったが、`image_writer_t::write_obj`側の処理は
//!   obj種別を一切区別しないため、書けば同じ挙動になる前提でそのまま再利用する。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告について検討したが、menu/cursor/symbolと
//!   全く同じ理由（`keys`は空文字列でない値のみでappendされ、`image_writer_t::write_obj`
//!   は空文字列/`"-"`に対して早期returnせず最後まで実行してcountをインクリメントする）
//!   で到達する実行経路が無い。
//!
//! ## menu/cursor/symbolとの比較結論
//!
//! `smoke_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`と**挙動上完全に
//! 同一**であることを確認した（`get_type()`/`get_type_name()`の返り値が異なるだけ）。
//! 「smoke」という名前から視覚エフェクトのタイミング処理を連想したが、そのような
//! 処理は`obj=factory`側のフィールドとゲーム実行時ロジックに存在し、makeobjの
//! `obj=smoke`書き込み処理自体には一切含まれないことを確認した。よって本モジュールの
//! ルールは`menu.rs`/`cursor.rs`/`symbol.rs`の`AllImagesRule`と同一のロジックを、
//! obj_type文字列とコメントの言い回しのみ差し替えて採用する。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった。menu/cursor/symbolと同一の理由）:
//! - `name`/`copyright`未指定チェック: good/sound/ground/menu/cursor/symbolと全く
//!   同じ理由（`obj_writer_t::write_name_and_copyright`とtext_writer_t::write_objは
//!   空文字列を無条件許容し、fatal/warningを出さない）。
//! - 画像0枚（`image[0]`未指定）の警告: `keys`が空のままループが1回で終了するだけで、
//!   fatal/warningの分岐は無い（menu/cursor/symbolと同種の判断）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、smokeの`keys`は空文字列を
//!   含まない状態で構築され、`image_writer_t::write_obj`が空文字列に対して
//!   countをスキップすることも無いため、`count < keys.get_count()`に到達する
//!   実行経路が無い（menu/cursor/symbolと同じ理由）。
//! - `image[i]`の途中欠落（"歯抜け"）自体の検出: これはmakeobjの実際の仕様どおりの
//!   動作であり（`str.empty()`で即座に走査打ち切り）、fatal/warningのソース側分岐も
//!   無い（menu/cursor/symbolと同じ理由）。
//! - `waytype`/`cursor`/`icon`（フィールドとしての）関連の検証: good/tree/menu/cursor/
//!   symbolと同じ理由（skin_writer.h/skin_writer.cc全文にこれらのフィールドへの言及が
//!   一つも無い）。
//! - アニメーション周期・フェード等の煙エフェクトのタイミング検証（`smokeuplift`/
//!   `smokelifetime`の妥当性等）: これらは`obj=factory`側のフィールドであり
//!   `skin_writer.cc`/`skin_writer.h`には一切登場しない（`obj=factory`の
//!   `FACTORY_NAMED_POST_BUILDING`で既に順序定義済みだが、fatal/warning分岐の有無は
//!   `src/rules/factory.rs`のREJECTEDコメント参照。本モジュールのスコープ外）。

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

/// `tests/smoke_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_smoke(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("smoke", dat, dat_dir)
}
