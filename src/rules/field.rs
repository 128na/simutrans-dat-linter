//! `obj=field`（`obj=factory`の`fields=`フィールドが名前で参照する、収穫段階
//! （作物の生育フェーズ）ごとのスキン画像定義。`.dat`に実際に書く`obj=`の値は
//! `field`。この`obj=field`（トップレベルobj種別）と、`obj=factory`が持つ
//! `fields=`/`max_fields=`/`min_fields=`/`start_fields=`**フィールド**（工場が
//! どの`obj=field`を何個まで生成するかの指定。`src/rules/factory.rs`が対象）は
//! 全くの別概念であり、同じ単語を使うだけで意味的なつながりは無い。事前合意の
//! 14種計画には含まれておらず、soundマイルストーンでの`descriptor/writer/`配下の
//! 機械的な棚卸しで発見された7種の新規obj種別のうち6件目に対応した1件）の
//! 検証ルール。検証根拠は`rules/mod.rs`冒頭コメント参照。
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
//! `field_writer_t::get_type_name()`（skin_writer.h:91-101）は`return "field";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ（menu/cursor/symbol/
//! smokeと同一パターン）。さらに実際の公開`.dat`ファイル（GitHub code search、
//! `repos/simutrans/pak128`内`corn_farm.dat`を直接fetchして確認）でも
//! `obj=field`が使われていることを確認した。`corn_farm.dat`は1ファイル内に
//! `Obj=factory`ブロックと`obj=field`ブロックの**2つ**を`----`区切り線で
//! 連結する構成になっており、factory側の`fields=corn_field`が名前でこの
//! `obj=field`ブロック（`name=corn_field`）を参照する:
//!
//! ```text
//! Obj=factory
//! ...
//! max_fields=80
//! min_fields=10
//! production_per_field=5
//! fields=corn_field
//! ...
//! ----------------------------------
//! obj=field
//! name=corn_field
//! copyright=Sarlock
//!
//! Image[0]=corn_farm.4.3
//! Image[1]=corn_farm.4.0
//! Image[2]=corn_farm.4.1
//! Image[3]=corn_farm.4.2
//! Image[4]=corn_farm.4.4
//! ```
//!
//! 同様に`repos/simutrans/pak128:factories/fields_agriculture.dat`は
//! `obj=field`ブロックのみを複数（`cotton_field`/`grain_field`等）連結した
//! ファイルで、いずれも`name`/`copyright`/`image[0..4]`（収穫段階5枚、うち
//! 1枚は積雪版）という構成だった。これは`skin_writer_t::write_obj`が
//! 生成する構造そのもの（`image[i]`の1次元・無制限走査 + name/copyright）と
//! 完全に一致しており、`field`固有の追加フィールド（生育段階数や日数等）は
//! `.dat`側に一切存在しないことも実例から確認できた。
//!
//! ## `skin_writer_t` / `field_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/
//! `smoke_writer_t`と同じく、`field_writer_t`（skin_writer.h:91-101）も
//! 共通の基底クラス`skin_writer_t`（skin_writer.h:18-29）のサブクラスであり、
//! `get_type()`/`get_type_name()`の2つのオーバーライドのみを持つ（`write_obj`は
//! 一切オーバーライドしない）。よって`field`の実際の書き込みロジックは全て
//! 基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものであり、
//! `menu`/`cursor`/`symbol`/`smoke`と**完全に同一**である。「field」という
//! 名前が作物の生育段階の日数・タイミング処理（`.dat`側での段階数指定や
//! 生育速度パラメータ等）を連想させる可能性を考慮し、本モジュールでも
//! smokeマイルストーンと同様に他4種の結論を鵜呑みにせずskin_writer.h/
//! skin_writer.ccを実際に読み直して独立に確認したが、そのような特別な処理は
//! `write_obj`側に一切無く、`menu`/`cursor`/`symbol`/`smoke`と**挙動上完全に
//! 同一**であることを確認した（詳細は下記コード引用）。実例
//! （`fields_agriculture.dat`のコメント「5 images, of which one is snow」）
//! からも、生育段階数は`.dat`記述側で単に`image[0..4]`を書く枚数として
//! 表現されるだけで、段階数・タイミングを指定する専用フィールドは存在しない
//! ことがわかる（生育の進行速度自体は`obj=factory`側の`production_per_field`
//! やゲーム実行時ロジックの領分であり、`obj=field`自体は単なる画像リスト）:
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
//!   走査全体が終了する（menu/cursor/symbol/smokeと全く同じ、単一の1次元配列の
//!   走査）。
//! - `"-"`（画像なしセンチネル、image_writer.cc:343の仕様コメント参照）は
//!   空文字列ではないため、`image[N]=-`と書けばそのNでの走査は継続する
//!   （menu/cursor/symbol/smokeと同じ扱い）。
//! - `name`/`copyright`は`write_name_and_copyright`（obj_writer.cc:62-70）経由で
//!   `text_writer_t::write_obj`（text_writer.cc:12-23）に渡るが、どちらも空文字列を
//!   無条件に許容しfatal/warningを出さない（menu/cursor/symbol/smokeと同じ）。
//! - `field_writer_t`は`waytype`/`cursor`/`icon`のいずれも読まない
//!   （`skin_writer_t::write_obj`/`skin_writer.h`全文にこれらへの言及なし）。
//!   作物の生育段階を構成するスキン画像（収穫フェーズフレーム列）であり、
//!   ビルドメニューに「載る」対象（building等）ではないため、cursor/iconという
//!   （フィールドとしての）概念自体が無い（crossing/ground_obj/tree/citycar/
//!   pedestrian/menu/cursor/symbol/smokeと同じパターン）。
//! - 個々の`image[i]`キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-514、`imagelist_writer_t`
//!   経由）がファイルの存在・サイズ（128の倍数か）を検証する。これは他の全obj種別と
//!   共有の`common::check_image_ref`でカバーする。
//!   **`"> "`（ズーム不可フラグ）構文について**: この処理はobj種別中立の共通ロジック
//!   （image_writer.cc:356-364）であり、menuマイルストーンで`common::check_image_ref`
//!   側に`strip_zoomable_prefix_and_trim`として既に実装済みのため、field固有の
//!   追加対応は不要（cursor/symbol/smokeと同様）。実際の公開`.dat`のfield系
//!   ファイル（`corn_farm.dat`/`fields_agriculture.dat`）に`>`プレフィックス構文を
//!   使う実例は見つからなかったが、`image_writer_t::write_obj`側の処理はobj種別を
//!   一切区別しないため、書けば同じ挙動になる前提でそのまま再利用する。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告について検討したが、menu/cursor/
//!   symbol/smokeと全く同じ理由（`keys`は空文字列でない値のみでappendされ、
//!   `image_writer_t::write_obj`は空文字列/`"-"`に対して早期returnせず最後まで
//!   実行してcountをインクリメントする）で到達する実行経路が無い。
//!
//! ## menu/cursor/symbol/smokeとの比較結論
//!
//! `field_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/
//! `smoke_writer_t`と**挙動上完全に同一**であることを確認した（`get_type()`/
//! `get_type_name()`の返り値が異なるだけ）。「field」という名前から作物の
//! 生育段階の日数・タイミング処理を連想したが、そのような処理は`obj=factory`側の
//! フィールド（`production_per_field`等）とゲーム実行時ロジックに存在し、
//! makeobjの`obj=field`書き込み処理自体には一切含まれないことを確認した。
//! よって本モジュールのルールは`menu.rs`/`cursor.rs`/`symbol.rs`/`smoke.rs`の
//! `AllImagesRule`と同一のロジックを、obj_type文字列とコメントの言い回しのみ
//! 差し替えて採用する。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった。menu/cursor/symbol/smokeと同一の理由）:
//! - `name`/`copyright`未指定チェック: good/sound/ground/menu/cursor/symbol/smokeと
//!   全く同じ理由（`obj_writer_t::write_name_and_copyright`とtext_writer_t::write_objは
//!   空文字列を無条件許容し、fatal/warningを出さない）。
//! - 画像0枚（`image[0]`未指定）の警告: `keys`が空のままループが1回で終了するだけで、
//!   fatal/warningの分岐は無い（menu/cursor/symbol/smokeと同種の判断）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、fieldの`keys`は空文字列を
//!   含まない状態で構築され、`image_writer_t::write_obj`が空文字列に対して
//!   countをスキップすることも無いため、`count < keys.get_count()`に到達する
//!   実行経路が無い（menu/cursor/symbol/smokeと同じ理由）。
//! - `image[i]`の途中欠落（"歯抜け"）自体の検出: これはmakeobjの実際の仕様どおりの
//!   動作であり（`str.empty()`で即座に走査打ち切り）、fatal/warningのソース側分岐も
//!   無い（menu/cursor/symbol/smokeと同じ理由）。
//! - `waytype`/`cursor`/`icon`（フィールドとしての）関連の検証: good/tree/menu/cursor/
//!   symbol/smokeと同じ理由（skin_writer.h/skin_writer.cc全文にこれらのフィールドへの
//!   言及が一つも無い）。
//! - 生育段階数・生育速度等の妥当性検証（画像枚数と`obj=factory`側の
//!   `production_per_field`等との整合性）: これらは`obj=factory`側のフィールドで
//!   あり、`skin_writer.cc`/`skin_writer.h`には一切登場しない（`obj=factory`側の
//!   `fields`/`max_fields`/`min_fields`/`start_fields`フィールドのfatal/warning
//!   分岐の有無は`src/rules/factory.rs`のREJECTEDコメント参照。本モジュールの
//!   スコープ外）。`obj=field`側から見て、参照元の`obj=factory`が何個・どの名前で
//!   この`obj=field`を参照しているかを検証するには複数`.dat`ファイルを横断する
//!   解析が必要であり、これは1ファイル単位の`lint`ではなく`couplings`サブコマンドの
//!   ようなスコープになる（現時点では対象外）。

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

/// `tests/field_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_field(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("field", dat, dat_dir)
}
