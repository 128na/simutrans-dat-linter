//! `obj=tree`（樹木の景観オブジェクト。`.dat`に実際に書く`obj=`の値は`tree`）の
//! 検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/tree_writer.cc` / `tree_writer.h` /
//! `get_climate.cc` / `imagelist2d_writer.cc` / `imagelist_writer.cc` /
//! `image_writer.cc` / `obj_writer.cc` / `dataobj/tabfile.cc`）を直接読んで確認した。
//! OTRP側の個別diffはまだ行っていない（good/bridge/tunnel/roadsign/crossing/
//! way-object/ground_objと同様）。
//!
//! ## `obj=`文字列について
//!
//! `tree_writer_t::get_type_name()`（`tree_writer.h:29`）は`return "tree";`を
//! そのまま返す。way-object（ファイル名`way_obj_writer.cc`だが実際の値は
//! ハイフン区切り`way-object`）やground_obj（ファイル名`groundobj_writer.cc`だが
//! 実際の値はアンダースコア区切り`ground_obj`）のような、ファイル名からの
//! 単純な類推が外れる前例があったため、今回も実際に`get_type_name()`の返り値を
//! 確認した。`tree`は`tree_writer.cc`というファイル名から素直に導ける文字列と
//! 一致していた。さらにGitHub code searchで実際の公開`.dat`ファイル
//! （例: `simutrans/pak128:landscape/trees/tree042.dat`、
//! `Cousjava/simutrans-paks:pak128/landscape/trees/palm010.dat`、
//! `Varkalandar/pak144.Excentrique:src/tree/hjm-trees.dat`）でも
//! `obj=tree`が使われていることを確認した。
//!
//! ## `tree_writer_t::write_obj`（tree_writer.cc:17-69）の構造
//!
//! - `climates`（`obj.get("climates")`）: `tabfileobj_t::get()`はキー欠落時にも
//!   **NULLではなく空文字列**を返す（tabfile.cc:48-56）。ソースの
//!   `if (climate_str) { allowed_climates = get_climate_bits(climate_str); }
//!   else { printf("WARNING: old syntax without climates!\n"); allowed_climates =
//!   all_but_arctic_climate; }`という分岐は、`climate_str`が常に非NULLの
//!   `const char*`であるため**常にtrue側に入り、else分岐（printf警告＋デフォルト
//!   climate）は実行時に到達しない**（groundobjの`climates`と全く同じ構造の
//!   デッドコード。climatesキーが未指定でも`get_climate_bits("")`が呼ばれるだけで、
//!   これはSTRICMPが何にも一致せず`uv16=0`のまま返るのみ）。よって
//!   「climates未指定で警告」というルールはmakeobj時点の実際の挙動と一致しないため
//!   実装しない（下記REJECTED参照）。なお、そもそもこのelse分岐は`dbg->warning`
//!   ではなく素の`printf`であり、他obj種別のログレベル制御下にも無い点も
//!   groundobjの`climates`警告と異なる。
//! - `seasons`（`get_int("seasons", 1)`）・`distributionweight`
//!   （`get_int("distributionweight", 3)`）は共に無条件フォールバックのみで読まれ、
//!   `get_int_clamped`は使われていない（tree_writer.cc:34-35）。bridgeの
//!   `ClampedRangeRule`に相当するルールはtreeには存在しない（下記REJECTED参照）。
//! - 画像（tree_writer.cc:37-55）: `age`（0..4の固定5段階）×`season`
//!   （0..number_of_seasons-1）の二重ループで`image[<age>][<season>]`を
//!   全て走査する。groundobjの`phase`（無制限ループ、season 0欠落で早期終了）や
//!   way-objectの`ribi`（26方向）と異なり、treeは**ageが常に5固定・early exitなし**
//!   というシンプルな構造で、`str.empty()`が一つでもtrueなら即座に
//!   `dbg->fatal( "Tree", "Missing %s!", buf)`でFATALになる（tree_writer.cc:49-52）。
//!   つまり「age 0..4 × season 0..number_of_seasons-1 の全組み合わせの画像が
//!   無条件に必須」というシンプルなルールになる。`obj.get(buf)`（文字列版、
//!   tabfile.cc:48-56）も欠落時に空文字列を返すため、`str.empty()`は
//!   「キー自体が無い」場合と「値が空文字列」の場合の両方を等しく検出する。
//! - 個々の画像キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc、`imagelist_writer_t`経由で
//!   `imagelist2d_writer_t::write_obj`から呼ばれる）がファイルの存在・サイズ
//!   （128の倍数か）を検証する。これはbuilding/way/bridge/tunnel/roadsign/
//!   crossing/way-object/ground_objと共有の`common::check_image_ref`でカバーする。
//! - `cursor`/`icon`フィールドへの言及がtree_writer.cc全文に一つも無く
//!   （`cursorskin_writer_t`も呼ばれない）、他のobj種別と異なりそもそも
//!   対象フィールドが存在しない（crossing/ground_objと同様のパターン。樹木は
//!   ビルドメニューから選択して建てるものではなく、マップ生成時に自動配置される
//!   scenery objectのため、cursor/iconという概念自体が無い）。
//! - `waytype`フィールドへの言及もtree_writer.cc全文に一つも無く、
//!   `get_waytype()`は一切呼ばれない（goodと同様、waytypeを持たない完全な
//!   scenery object）。よって`common::KNOWN_WAYTYPES`はtreeには適用されない。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `climates`未指定の警告: ソースコード上に
//!   `printf("WARNING: old syntax without climates!\n")`という分岐が存在するため
//!   一見警告ルールの根拠がありそうに見えるが、`tabfileobj_t::get()`
//!   （tabfile.cc:48-56）はキー欠落時にNULLではなく空文字列`""`を返す実装であるため、
//!   `if (climate_str)`（非NULLかどうかのチェック）は常にtrueとなり、この警告分岐は
//!   実行時に到達しない（groundobjの`climates`REJECTEDと全く同じ理由）。
//! - `seasons`/`distributionweight`の妥当性検証: いずれも`get_int`で無条件に読み、
//!   `get_int_clamped`は一度も呼ばれていない（tree_writer.cc:34-35）。bridgeの
//!   `ClampedRangeRule`に相当する根拠が無いため見送り（way/tunnel/roadsign/good/
//!   way-object/ground_objの同種フィールドが見送られたのと同じ理由）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: tree_writer.cc内の`keys.at(age).append(str)`
//!   はage×seasonの二重ループ全体で必ず`image_writer_t::write_obj`の呼び出し回数と
//!   同数のappendを行う（値が空文字列であってもappendされる前に`dbg->fatal`で
//!   即座にプロセスが終了するため、そもそもこのカウント不一致は発生しえない）。
//!   tunnel/crossing/way-object/groundobjの同種警告が見送られたのと同じ理由。
//! - `cursor`/`icon`未指定検証: crossing/ground_objと同じ理由（tree_writer.cc全文に
//!   `cursor`/`icon`への言及が一つも無く、`cursorskin_writer_t`も呼ばれない。他の
//!   obj種別と異なり、そもそも対象フィールドが存在しない）。
//! - `waytype`関連の検証: goodと同じ理由（tree_writer.cc全文に`waytype`への言及が
//!   一つも無く、`get_waytype()`が一切呼ばれない）。
//! - ageの段階数（5固定）を`.dat`側でカスタマイズできるかの検証: `for (unsigned int
//!   age = 0; age < 5; age++)`はソースコード上のハードコードされた定数であり、
//!   `.dat`側にage数を指定するキーは存在しない。従って「age数が不正」という
//!   ルールは成立しない（そもそも検証対象のキーが無い）。

use super::common::{NameAndCopyrightStringFieldRule, check_image_ref};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::t;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// tree_writer.cc:37-55の`for (age = 0; age < 5; age++)`のハードコード定数。
const AGE_COUNT: u32 = 5;

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(AgeSeasonImageRule),
        Box::new(NameAndCopyrightStringFieldRule),
    ]
}

/// `tests/tree_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_tree(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("tree", dat, dat_dir)
}

/// tree_writer.cc:37-55: `age`（0..4固定5段階）×`season`（0..number_of_seasons-1）の
/// 全組み合わせについて`image[<age>][<season>]`が必須。いずれか1つでも空文字列
/// （キー欠落含む）なら`dbg->fatal( "Tree", "Missing %s!", buf)`でFATALになる。
///
/// `number_of_seasons`は`get_int("seasons", 1)`（無条件フォールバック、既定値1）。
struct AgeSeasonImageRule;
impl Rule for AgeSeasonImageRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        let seasons: i64 = dat
            .get("seasons")
            .unwrap_or("")
            .trim()
            .parse()
            .unwrap_or(1)
            .max(1);

        for age in 0..AGE_COUNT {
            for season in 0..seasons {
                let key = format!("image[{age}][{season}]");
                let value = dat.get(&key).unwrap_or("");
                if value.is_empty() {
                    diags.push(Diagnostic::error(
                        DiagnosticCode::MissingAgeSeasonImage,
                        t!(ctx.language,
                            ja: "{key}: age {age} season {season} の画像が未指定です。\
                                 makeobjはFATAL ERRORになります（\"Missing {key}!\"）",
                            en: "{key}: image for age {age} season {season} is unspecified. \
                                 makeobj treats this as a FATAL ERROR (\"Missing {key}!\")",
                            key = key,
                            age = age,
                            season = season,
                        ),
                    ));
                } else {
                    check_image_ref(
                        value,
                        ctx.dat_dir,
                        &key,
                        &mut diags,
                        ctx.language,
                        ctx.tile_size,
                        dat.line_of(&key),
                    );
                }
            }
        }

        diags
    }
}
