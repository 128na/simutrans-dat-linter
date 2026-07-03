//! `obj=misc`（ゲームがまだ「本物のオブジェクト」として統合していない、雑多なUI用画像を
//! 保持するスキン画像定義。`skin_writer.h`のクラス名コメント「Used for images needed by
//! the game but not yet integrated as real objects」の通り）で検出する主な項目。
//! 事前合意の14種計画には含まれておらず、soundマイルストーンでの`descriptor/writer/`配下の
//! 機械的な棚卸しで発見された7種の新規obj種別のうち7件目（最後）に対応した1件。
//! 検証根拠は`rules/mod.rs`冒頭コメント参照。
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
//! `miscimages_writer_t::get_type_name()`（skin_writer.h:106-118）は`return "misc";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`obj_writer_t::register_writer(true)`、obj_writer.cc:24-36の`if (main_obj)`分岐
//! 経由）である点は他のobj種別と同じ（menu/cursor/symbol/smoke/fieldと同一パターン）。
//! さらに実際の公開`.dat`ファイルを直接fetchして確認した（GitHub code searchは
//! 部分一致ノイズが多いため、有望なパス名を直接あたった）:
//! - `simutrans/pak128:base/misc_GUI/construction.dat`
//!   （`Obj=misc` / `Name=Construction` / `copyright=MHz` /
//!   `Image[0]=construction.1.0` 〜 `Image[9]=construction.1.9`の10枚構成）
//! - `aburch/simutrans-pak128.britain:gui/gui128/misc_images-128.dat`
//!   （`Obj=misc` / `Name=Construction` / `copyright=James` /
//!   `Image[0]` 〜 `Image[6]`の7枚構成）
//!
//! いずれも`skin_writer_t::write_obj`が生成する構造（`image[i]`の1次元・無制限走査 +
//! name/copyright）と完全に一致しており、`misc`固有の追加フィールドは`.dat`側に
//! 一切存在しないことも実例から確認できた。
//!
//! ## `skin_writer_t` / `miscimages_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/`smoke_writer_t`/
//! `field_writer_t`と同じく、`miscimages_writer_t`（skin_writer.h:106-118）も
//! 共通の基底クラス`skin_writer_t`（skin_writer.h:18-29）のサブクラスであり、
//! `get_type()`/`get_type_name()`の2つのオーバーライドのみを持つ（`write_obj`は
//! 一切オーバーライドしない）。よって`misc`の実際の書き込みロジックは全て
//! 基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものであり、
//! `menu`/`cursor`/`symbol`/`smoke`/`field`と**完全に同一**である。
//! `miscimages_writer_t`の直前コメント（skin_writer.h:104-106、
//! 「Used for images needed by the game but not yet integrated as real objects」）は
//! 「まだ本物のオブジェクトとして統合されていない画像用」という意味論上の説明であり、
//! 特別な検証処理・専用フィールドを示唆するものではないかを確認する必要があった。
//! 他5種と同様に本モジュールでも結論を鵜呑みにせずskin_writer.h/skin_writer.ccを
//! 実際に読み直して独立に確認したが、そのような特別な処理は`write_obj`側に一切無く、
//! `miscimages_writer_t`のクラス定義（skin_writer.h:106-118）には`get_type()`/
//! `get_type_name()`以外のメンバー関数は一切無い。`skin_writer.cc`全文
//! （18-52行、ファイル末尾）にも`miscimages_writer_t`固有の分岐は存在しない
//! （クラス名で分岐する`switch`/`if`文も皆無）:
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
//!   走査全体が終了する（menu/cursor/symbol/smoke/fieldと全く同じ、単一の1次元配列の
//!   走査）。
//! - `"-"`（画像なしセンチネル、image_writer.cc:343の仕様コメント参照）は
//!   空文字列ではないため、`image[N]=-`と書けばそのNでの走査は継続する
//!   （menu/cursor/symbol/smoke/fieldと同じ扱い）。
//! - `name`/`copyright`は`write_name_and_copyright`（obj_writer.cc:62-70）経由で
//!   `text_writer_t::write_obj`（text_writer.cc:12-23）に渡るが、どちらも空文字列を
//!   無条件に許容しfatal/warningを出さない（menu/cursor/symbol/smoke/fieldと同じ）。
//! - `miscimages_writer_t`は`waytype`/`cursor`/`icon`のいずれも読まない
//!   （`skin_writer_t::write_obj`/`skin_writer.h`全文にこれらへの言及なし）。
//!   ビルドメニューに「載る」対象（building等）ではなく、パークセット全体で
//!   使う雑多なUI画像素材の連番リストであるため、cursor/iconという
//!   （フィールドとしての）概念自体が無い（crossing/ground_obj/tree/citycar/
//!   pedestrian/menu/cursor/symbol/smoke/fieldと同じパターン）。
//! - 個々の`image[i]`キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-514、`imagelist_writer_t`
//!   経由）がファイルの存在・サイズ（128の倍数か）を検証する。これは他の全obj種別と
//!   共有の`common::check_image_ref`でカバーする。
//!   **`"> "`（ズーム不可フラグ）構文について**: この処理はobj種別中立の共通ロジック
//!   （image_writer.cc:356-364）であり、menuマイルストーンで`common::check_image_ref`
//!   側に`strip_zoomable_prefix_and_trim`として既に実装済みのため、misc固有の
//!   追加対応は不要（cursor/symbol/smoke/fieldと同様）。実際の公開`.dat`の
//!   `construction.dat`系ファイルに`>`プレフィックス構文を使う実例は見つからなかったが、
//!   `image_writer_t::write_obj`側の処理はobj種別を一切区別しないため、書けば同じ挙動に
//!   なる前提でそのまま再利用する。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告について検討したが、menu/cursor/
//!   symbol/smoke/fieldと全く同じ理由（`keys`は空文字列でない値のみでappendされ、
//!   `image_writer_t::write_obj`は空文字列/`"-"`に対して早期returnせず最後まで
//!   実行してcountをインクリメントする）で到達する実行経路が無い。
//!
//! ## menu/cursor/symbol/smoke/fieldとの比較結論
//!
//! `miscimages_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/`smoke_writer_t`/
//! `field_writer_t`と**挙動上完全に同一**であることを確認した（`get_type()`/
//! `get_type_name()`の返り値が異なるだけ）。「まだ本物のオブジェクトとして統合されて
//! いない画像用」というクラス直前コメントの説明は、`obj=misc`という種別の
//! **意味論的な位置づけ**（パークセット全体でメタ的に使う未整理画像素材）を示す
//! だけであり、makeobjの`obj=misc`書き込み処理自体には特別な検証・専用フィールドは
//! 一切含まれないことを確認した。よって本モジュールのルールは`menu.rs`/`cursor.rs`/
//! `symbol.rs`/`smoke.rs`/`field.rs`の`AllImagesRule`と同一のロジックを、obj_type
//! 文字列とコメントの言い回しのみ差し替えて採用する。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった。menu/cursor/symbol/smoke/fieldと同一の理由）:
//! - `name`/`copyright`未指定チェック: good/sound/ground/menu/cursor/symbol/smoke/
//!   fieldと全く同じ理由（`obj_writer_t::write_name_and_copyright`と
//!   `text_writer_t::write_obj`は空文字列を無条件許容し、fatal/warningを出さない）。
//! - 画像0枚（`image[0]`未指定）の警告: `keys`が空のままループが1回で終了するだけで、
//!   fatal/warningの分岐は無い（menu/cursor/symbol/smoke/fieldと同種の判断）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、miscの`keys`は空文字列を
//!   含まない状態で構築され、`image_writer_t::write_obj`が空文字列に対して
//!   countをスキップすることも無いため、`count < keys.get_count()`に到達する
//!   実行経路が無い（menu/cursor/symbol/smoke/fieldと同じ理由）。
//! - `image[i]`の途中欠落（"歯抜け"）自体の検出: これはmakeobjの実際の仕様どおりの
//!   動作であり（`str.empty()`で即座に走査打ち切り）、fatal/warningのソース側分岐も
//!   無い（menu/cursor/symbol/smoke/fieldと同じ理由）。
//! - `waytype`/`cursor`/`icon`（フィールドとしての）関連の検証: good/tree/menu/cursor/
//!   symbol/smoke/fieldと同じ理由（skin_writer.h/skin_writer.cc全文にこれらの
//!   フィールドへの言及が一つも無い）。
//! - `image[i]`の枚数・用途（例: `construction.dat`のコメントが示す「0=city
//!   building, 1=tourist attraction...」というインデックスごとの意味）の妥当性検証:
//!   これはゲーム実行時ロジック（`skin_desc_t`側でインデックスの意味を解釈する）の
//!   領分であり、`skin_writer.cc`/`skin_writer.h`（makeobj側）には一切登場しない
//!   （本ツールのスコープ外）。

use super::common;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// ルール実装本体は`menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6種別で
/// 共有される`common::AllImagesRule`（skin_writer_t::write_objそのもの、根拠は
/// 上記コメント参照）。
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![Box::new(common::AllImagesRule)]
}

/// `check_menu`/`check_cursor`/`check_symbol`/`check_smoke`/`check_field`と対称的な
/// 薄いラッパー。
pub fn check_misc(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext {
        dat,
        dat_dir,
        language: crate::i18n::Language::default(),
    };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
}
