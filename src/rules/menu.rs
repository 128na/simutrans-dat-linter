//! `obj=menu`（ビルドメニューのウィンドウ枠・ボタン等、UIスキン画像定義。
//! `.dat`に実際に書く`obj=`の値は`menu`）の検証ルール。検証根拠は
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
//! `menuskin_writer_t::get_type_name()`（skin_writer.h:43）は`return "menu";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ。さらに実際の公開
//! `.dat`ファイル（GitHub code search、`"obj=menu" extension:dat`）でも
//! `Obj=menu`が使われていることを確認した
//! （例: `aburch/simutrans-pak128.britain:gui/gui64/skins-64.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/UI/32/menu.dat`、
//! `VictorErik/Pak128.Sweden-Ex:Base/misc_GUI_64/new_menus.dat`、
//! `aburch/simutrans:themes.src/flat/flat-skin.dat`）。
//!
//! ## `skin_writer_t` / `menuskin_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6つのobj種別は全て共通の
//! 基底クラス`skin_writer_t`（skin_writer.h:18-29）のサブクラスであり、
//! `menuskin_writer_t`（skin_writer.h:32-44）は`get_type()`/`get_type_name()`の
//! 2つのオーバーライドのみを持つ（`write_obj`は一切オーバーライドしない）。
//! よって`menu`の実際の書き込みロジックは全て基底`skin_writer_t::write_obj`
//! （skin_writer.cc:18-51）そのものである:
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
//!   走査全体が終了する（groundのslope内phase走査・citycarの固定8方向走査とは
//!   異なり、menuは**添字が連続していないと途中で切れる**単一の1次元配列である点が
//!   特徴的。例えば`image[0]`〜`image[9]`はあるが`image[10]`が欠落していれば、
//!   `image[11]`以降が実際に書かれていても一切読まれない）。
//! - `"-"`（画像なしセンチネル、image_writer.cc:343の仕様コメント参照）は
//!   空文字列ではないため、`image[N]=-`と書けばそのNでの走査は継続する
//!   （groundのslope内phase走査・way-objectのribi/slope走査などと同じ扱い）。
//! - `name`/`copyright`は`write_name_and_copyright`（obj_writer.cc:62-70）経由で
//!   `text_writer_t::write_obj`（text_writer.cc:12-23）に渡るが、どちらも空文字列を
//!   無条件に許容しfatal/warningを出さない。good/sound/groundと全く同じ。
//! - `menuskin_writer_t`は`waytype`/`cursor`/`icon`のいずれも読まない
//!   （`skin_writer_t::write_obj`/`skin_writer.h`全文にこれらへの言及なし）。
//!   ビルドメニュー自体を構成するUIスキン画像であり、ビルドメニューに「載る」対象
//!   （building等）ではないため、cursor/iconという概念自体が無い
//!   （crossing/ground_obj/tree/citycar/pedestrianと同じパターン）。
//! - 個々の`image[i]`キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-514、`imagelist_writer_t`
//!   経由）がファイルの存在・サイズ（128の倍数か）を検証する。これは他の全obj種別と
//!   共有の`common::check_image_ref`でカバーする。
//!   **menu固有の追加発見**: `image_writer_t::write_obj`の構文仕様コメント
//!   （image_writer.cc:342-347）どおり、値の先頭に`"> "`（`>`+空白、または`>`
//!   単体）を書くと「ズーム不可」フラグとして`'>'`の1文字が剥がされる
//!   （`an_imagekey[0]=='>'`分岐、image_writer.cc:356-364）。実際の公開`.dat`
//!   （`aburch/simutrans-pak128.britain:gui/gui64/skins-64.dat`の
//!   `Image[0]=> skins.0.4`等）でこの構文が使われていることを確認した。
//!   この処理はobj種別中立の`image_writer_t::write_obj`内の分岐であり
//!   menu以外の画像キーにも理論上適用されるが、実例が集中しているのはmenu/symbol/
//!   cursor系のスキン`.dat`であり、本マイルストーンで発覚したため
//!   `common::check_image_ref`側（全obj種別共有）に`strip_zoomable_prefix`として
//!   修正を反映した（先頭`'>'`を剥がし`trim`してから解決する。修正前は
//!   `"> skins.0.4"`をそのままファイル名として解決しようとし、実在するファイルを
//!   「見つからない」と誤検知していた）。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告（`dbg->warning(...,"Expected %i but
//!   found %i images (might be correct)!")`）について検討したが、menuの`keys`は
//!   `skin_writer_t::write_obj`のオーバーロード1で構築される時点で、既に
//!   「空文字列でない値のみ」がappendされている（`str.empty()`ならその時点で
//!   走査を打ち切り、appendしない）。かつ`image_writer_t::write_obj`は空文字列/
//!   `"-"`に対して早期returnせず最後まで実行してcountをインクリメントする
//!   （image_writer.cc:366,443-453、groundobj/tree/citycar等で確認済みの構造と
//!   同じ）ため、`count`は常に`keys.get_count()`に到達し、この警告分岐に到達する
//!   実行経路が無い（他の多くのobj種別と同じ結論）。
//!
//! ## 他5種別（cursor/symbol/smoke/field/misc）との関係
//!
//! `menuskin_writer_t`と同じく`cursorskin_writer_t`/`symbolskin_writer_t`/
//! `smoke_writer_t`/`field_writer_t`/`miscimages_writer_t`（skin_writer.h:47-122）も
//! いずれも`skin_writer_t`のサブクラスで、`get_type()`/`get_type_name()`の
//! オーバーライドのみを持ち`write_obj`は一切オーバーライドしない。つまり
//! 6種別とも**全く同一の`skin_writer_t::write_obj`ロジック**（image[i]の1次元
//! 無制限走査 + name/copyright）を共有しており、`get_type_name()`が返す文字列
//! （"menu"/"cursor"/"symbol"/"smoke"/"field"/"misc"）が違うだけである。
//! したがって将来この5種別を実装する際は、本モジュールの`AllImagesRule`と
//! ほぼ同一のロジックをそのまま流用できる見込みが高い（obj_type文字列と
//! REJECTED節の言い回しを差し替えるだけで済む可能性が高い）。ただし念のため、
//! 実装時は本コメントに頼らず各サブクラスのヘッダ定義を再確認すること
//! （このプロジェクトの一貫した方針: 「共有基底クラスだから同じはず」という
//! 推測だけでは実装しない）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `name`/`copyright`未指定チェック: good/sound/groundと全く同じ理由
//!   （`obj_writer_t::write_name_and_copyright`とtext_writer_t::write_objは
//!   空文字列を無条件許容し、fatal/warningを出さない）。
//! - 画像0枚（`image[0]`未指定）の警告: `keys`が空のままループが1回で終了するだけで、
//!   fatal/warningの分岐は無い（good/sound/groundの「無名でも動く」見送りと同種の
//!   判断）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、menuの`keys`は空文字列を
//!   含まない状態で構築され、`image_writer_t::write_obj`が空文字列に対して
//!   countをスキップすることも無いため、`count < keys.get_count()`に到達する
//!   実行経路が無い（citycar/groundobj/tree等の同種警告が見送られたのと同じ理由）。
//! - `image[i]`の途中欠落（"歯抜け"）自体の検出: これはmakeobjの実際の仕様どおりの
//!   動作であり（`str.empty()`で即座に走査打ち切り）、fatal/warningのソース側分岐も
//!   無い。groundの「途中のslopeで欠落すると以降が打ち切られる」REJECTED理由と
//!   同種。
//! - `waytype`/`cursor`/`icon`関連の検証: goodやtreeと同じ理由
//!   （skin_writer.h/skin_writer.cc全文にこれらのフィールドへの言及が一つも無い）。

use super::common::check_image_ref;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![Box::new(AllImagesRule)]
}

/// `check_ground`/`check_citycar`と対称的な薄いラッパー。
pub fn check_menu(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext { dat, dat_dir };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
}

/// skin_writer.cc:21-35: `image[0]`, `image[1]`, ... と1次元・無制限に走査し、
/// 最初に欠落した（空文字列の）添字で走査全体を終了する（`"-"`センチネルは
/// 空文字列ではないため走査を止めない）。実際に画像を指す値（空文字列でも
/// `"-"`でもない値）についてのみ、`common::check_image_ref`でファイル存在・
/// サイズ（128の倍数か）を検証する（他の全obj種別と共有のパターン）。
struct AllImagesRule;
impl Rule for AllImagesRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        let mut i = 0u32;
        loop {
            let key = format!("image[{i}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                // skin_writer.cc:28-30: キー欠落（空文字列）で走査終了。
                break;
            }
            if value != "-" {
                check_image_ref(value, ctx.dat_dir, &key, &mut diags);
            }
            i += 1;
            // 安全弁: dat構文異常でiが際限なく増え続ける事態を避ける
            // （makeobj自身は無限ループ`for (;;i++)`だが、実用上十分大きい上限で
            // 打ち切る。ground/groundobjのphase安全弁と同じ考え方）。
            if i > 4096 {
                break;
            }
        }

        diags
    }
}
