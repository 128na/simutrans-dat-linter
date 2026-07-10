//! `obj=ground`（水面・傾斜地・気候別テクスチャなど、地面タイルそのものの画像定義。
//! `.dat`に実際に書く`obj=`の値は`ground`）の検証ルール。検証根拠は
//! `rules/mod.rs`冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/ground_writer.cc` / `ground_writer.h` /
//! `descriptor/ground_desc.h` / `imagelist2d_writer.cc` / `imagelist_writer.cc` /
//! `image_writer.cc` / `obj_writer.cc` / `text_writer.cc` / `dataobj/tabfile.cc`）を
//! 直接読んで確認した。OTRP側の個別diffはまだ行っていない（building以外のobj種別と
//! 同様）。
//!
//! ## `obj=`文字列について
//!
//! このRustモジュール名は`ground`（既存の`groundobj`モジュール、`.dat`上の
//! `obj=ground_obj`とは全くの別物）。`ground_writer_t::get_type_name()`
//! （ground_writer.h:29）は`return "ground";`を返す。根拠は`obj_writer_t::write`
//! （obj_writer.cc:39-59）が`obj.get("obj")`の文字列でそのまま
//! `writer_by_name->get(type)`（obj_writer.cc:44）を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （obj_writer.cc:31、`register_writer(true)`経由）である点は他のobj種別と同じ。
//! さらに実際の公開`.dat`ファイル（GitHub code search、`extension:dat
//! "obj=ground"`）でも`Obj=ground`が使われていることを確認した
//! （例: `simutrans/pak128:landscape/grounds/slope.dat`
//! （`Obj=ground` / `Image[<slope>][<phase>]=`形式）、
//! `simutrans/pak128:landscape/grounds/texture-climate.dat`
//! （気候別テクスチャも`Image[<slope>][0]=`の位置引数のみで表現され、
//! `climates=`のような名前付きフィールドは存在しない）、
//! `Flemmbrav/Pak192.Comic:pakset/landscape/ground/shore.dat`）。
//! `obj=ground_obj`（別のobj種別、`rules/groundobj.rs`参照）や`obj=groundobj`
//! （どのwriterにも存在しない文字列）とは検索結果を目視で区別して除外した。
//!
//! ## `ground_writer_t::write_obj`（ground_writer.cc:15-45）の構造
//!
//! `good`/`sound`と同様、フィールド読み取りに分岐が一切無い最小の実装だが、
//! 唯一の実データが「128通りのslope×可変数のphaseからなる2次元画像リスト」である
//! 点が特異である:
//!
//! ```text
//! write_name_and_copyright(fp, node, obj);        // name, copyright
//! for (int slope = 0; slope < 128; slope++) {
//!     for (int phase = 0; ; phase++) {
//!         sprintf(buf, "image[%d][%d]", slope, phase);
//!         std::string str = obj.get(buf);
//!         if (str.empty()) break;                 // このslopeのphase走査を終了
//!         keys.at(slope).append(str);
//!     }
//!     if (keys.at(slope).empty()) break;           // slope走査全体を終了
//! }
//! imagelist2d_writer_t::instance()->write_obj(fp, node, keys);
//! ```
//!
//! - `obj.get(...)`（`tabfileobj_t::get`, tabfile.cc:48-56）はキー欠落時に
//!   **NULLではなく空文字列**を返す。`str.empty()`は「キーが実際に存在しないか、
//!   値として空文字列が書かれている」場合のみtrueになる。**`"-"`（画像なし
//!   センチネル、image_writer.cc:343の仕様コメント参照）は空文字列ではない**ため、
//!   `image[N][0]=-`と書けばそのslopeのphase走査は継続する（実例:
//!   `simutrans/pak128:landscape/grounds/slope.dat`の`Image[0][0]=-`）。
//! - `slope`は`0..127`の128通りに固定（C++の`for`ループの上限）。`image[128][0]`
//!   のような範囲外キーは単に走査されず無視されるだけで、fatal/warningの分岐は
//!   無い（下記REJECTED参照）。
//! - `phase`は無制限（`for (int phase = 0; ; phase++)`）。あるslopeの
//!   `image[<slope>][0]`が欠落（空文字列）なら、そのslopeも含めそれ以降の
//!   slopeは一切走査されずループ全体が終了する（`keys.at(slope).empty()`が
//!   trueになるため）。つまり「途中のslopeでphase0が抜けると、それ以降の
//!   すべてのslopeの画像が無視される」という位置依存の罠があるが、これ自体は
//!   FATALにはならない（画像0枚の`ground`もmakeobj時点ではエラーにならない）。
//! - `climates`/`waytype`/`cursor`/`icon`等の名前付きフィールドへの言及は
//!   ground_writer.cc全文に一つも無い。`ground_desc.h`のコメント（"Images of all
//!   possible surface tiles: slopes, climates, transitions, etc."）が示す通り、
//!   気候・傾斜の意味づけは全てslope番号の**位置**で決まる実行時ロジック
//!   （`ground_desc_t::get_climate_tile`等、ground_desc.cc側）であり、.dat記述側は
//!   純粆な位置引数の画像リストを書くだけである。よって`common::KNOWN_WAYTYPES`・
//!   `common::check_waytype_field`はどちらも適用対象が無い。
//! - 個々の画像キーが実際に画像を指す場合（空文字列でも`"-"`でもない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-514、`imagelist_writer_t`
//!   経由で`imagelist2d_writer_t::write_obj`から呼ばれる）がファイルの存在・
//!   サイズ（128の倍数か）を検証する。これは他の全obj種別と共有の
//!   `common::check_image_ref`でカバーする。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `name`/`copyright`未指定チェック: good/soundと全く同じ理由
//!   （`obj_writer_t::write_name_and_copyright`とtext_writer_t::write_objは
//!   空文字列を無条件許容し、fatal/warningを出さない）。
//! - 画像0枚（`image[0][0]`未指定）の警告: groundobjの固定物分岐と同様、
//!   `keys.at(0).empty()`が即trueになりループが1回で終了するだけで、fatal/warning
//!   の分岐は無い。ground_objでは「no-images」をinfoとして出しているが、これは
//!   groundobj側にwaytype省略という別の非対称ルールとセットで実装した経緯があり、
//!   groundにはwaytype概念自体が無いため対称的な理由付けが弱く、makeobj時点の
//!   fatal/warning根拠も無いことから見送った（good/soundの「無名でも動く」見送りと
//!   同種の判断）。
//! - `image[<slope>][<phase>]`のslope値が128以上（範囲外）の場合の検出:
//!   `for (int slope = 0; slope < 128; slope++)`はCの固定回数ループであり、
//!   `image[128][0]`のようなキーは単に一度も`sprintf`で生成されず、
//!   `obj.get()`が呼ばれることも無い。書いても無視されるだけで、makeobj側に
//!   fatal/warning/クランプの分岐は存在しない。「範囲外キーが黙って無視される」
//!   という状態はこのプロジェクトが過去に採用してきた「サイレントに間違った
//!   動作をする経路」の対象範囲に含めるか判断が分かれるが、他のobj種別
//!   （way_objのribi/slope走査、pedestrianの方向走査等）でも同種の「未知の
//!   キーは単に読まれないだけ」というケースは軒並みREJECTEDとしてきており
//!   （goodのcatgクランプ見送り等と同系統）、本プロジェクトの「fatal/warning/
//!   観測可能な誤動作のいずれかへの直接トレース」という基準を満たさないため
//!   見送った。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: `image_writer_t::write_obj`
//!   （image_writer.cc:348-514）は空文字列/`"-"`を含むどんな入力に対しても
//!   例外を投げるか正常終了してcountをインクリメントするかのいずれかであり、
//!   `imagelist_writer_t::write_obj`のループ内で`count`が`keys.get_count()`を
//!   下回ったまま関数を抜ける実行経路が無い（groundobj/tunnel/crossing/
//!   way-objectの同種警告が見送られたのと同じ理由）。
//! - 途中のslopeで`image[<slope>][0]`が欠落すると、それ以降の全slopeが
//!   無条件に走査打ち切りになる「位置依存の罠」自体の警告: これはmakeobjの
//!   実際の仕様どおりの動作であり、fatal/warningのソース側分岐も無い。
//!   pak128実例（`slope.dat`）でも`Image[0][0]=-`という「空文字列ではない
//!   ダミー値」を使うことでこの罠を回避する書き方が実際に使われており、
//!   これを「間違い」として検出するとむしろ正しい記述を誤検知することになる
//!   ため見送った。

use super::common::{NameAndCopyrightStringFieldRule, check_image_ref};
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// ground_writer.cc:23の`for (int slope = 0; slope < 128; slope++)`そのもの。
const MAX_SLOPES: u32 = 128;

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(SlopeImageRefRule),
        Box::new(NameAndCopyrightStringFieldRule),
    ]
}

/// `tests/ground_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_ground(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("ground", dat, dat_dir)
}

/// ground_writer.cc:21-41: slope=0..127、phase=0,1,2,...の`image[<slope>][<phase>]`を
/// 走査する。あるslopeの`image[<slope>][0]`が欠落（空文字列）なら、そのslope以降は
/// 一切走査されない（`str.empty()`は「キー欠落」のみを指し、`"-"`は含まれない点に
/// 注意。tabfileobj_t::get()はキー欠落時にNULLではなく空文字列を返すため）。
/// 実際に画像を指す値（空文字列でも`"-"`でもない値）についてのみ、
/// `common::check_image_ref`でファイル存在・サイズ（128の倍数か）を検証する
/// （他の全obj種別と共有のパターン）。
struct SlopeImageRefRule;
impl Rule for SlopeImageRefRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        for slope in 0..MAX_SLOPES {
            let mut phase = 0u32;
            loop {
                let key = format!("image[{slope}][{phase}]");
                let value = dat.get(&key).unwrap_or("");
                if value.is_empty() {
                    // ground_writer.cc:32-34: キー欠落（空文字列）でphase走査終了。
                    break;
                }
                // "-"（画像なしセンチネル）の判定は`check_image_ref`
                // （src/rules/common.rs）側に一元化されている。以前はここに
                // `value != "-"`ガードを個別追加していたが、第8弾で共通化した
                // ため不要（`check_image_ref`冒頭のdocコメント参照）。
                check_image_ref(
                    value,
                    ctx.dat_dir,
                    &key,
                    &mut diags,
                    ctx.language,
                    ctx.tile_size,
                    dat.line_of(&key),
                );
                phase += 1;
                // 安全弁: dat構文異常でphaseが際限なく増え続ける事態を避ける
                // （makeobj自身は無限ループ`for (;;phase++)`だが、実用上十分大きい
                // 上限で打ち切る。groundobjのphase安全弁と同じ考え方）。
                if phase > 4096 {
                    break;
                }
            }
            if phase == 0 {
                // ground_writer.cc:38-40: keys.at(slope).empty() -> slope走査全体を
                // 終了。このslope以降はimage[<slope>][0]すら読まれない。
                break;
            }
        }

        diags
    }
}
