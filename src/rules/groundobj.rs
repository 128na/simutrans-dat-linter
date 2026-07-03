//! `obj=ground_obj`（岩・廃墟・草むらなどの地面装飾オブジェクト。`.dat`に実際に書く
//! `obj=`の値は`ground_obj`。詳細は下記コラム参照）の検証ルール。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/groundobj_writer.cc` / `groundobj_writer.h` /
//! `get_waytype.cc` / `get_climate.cc` / `imagelist2d_writer.cc` / `imagelist_writer.cc` /
//! `image_writer.cc` / `obj_writer.cc` / `dataobj/tabfile.cc`）を直接読んで確認した。
//! OTRP側の個別diffはまだ行っていない（good/bridge/tunnel/roadsign/crossing/way-objectと同様）。
//!
//! ## `obj=`文字列について
//!
//! このプロジェクトの他のRustモジュール名・ファイル名は`groundobj`（スネークケース、
//! アンダースコアなし）で揃えているが、`.dat`に実際に書く`obj=`の値は
//! **`ground_obj`**（アンダースコア区切り）である。`groundobj_writer.cc`という
//! ファイル名や、way-objectの前例（ファイル名`way_obj_writer.cc`だが実際の値は
//! ハイフン区切り`way-object`）から安易に類推すると`"groundobj"`と誤りやすいため、
//! 実際に`get_type_name()`を確認した。根拠は`obj_writer_t::write`
//! （obj_writer.cc:39-59）が`obj.get("obj")`の文字列でそのまま
//! `writer_by_name->get(type)`（obj_writer.cc:44）を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （obj_writer.cc:31）である。`groundobj_writer_t::get_type_name()`
//! （groundobj_writer.h:29）は`return "ground_obj";`を返す
//! （他のwriterと比較: `way_obj_writer.h`は`"way-object"`、`crossing_writer.h`は
//! `"crossing"`、`good_writer.h`は`"good"`）。さらに実際の公開`.dat`ファイル
//! （GitHub code search、例: `simutrans/pak128:landscape/groundobj_static/cactus000_0.dat`、
//! `Cousjava/simutrans-paks:pak128/landscape/groundobj_static/cactus002_0.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/landscape/ground_objects/memes.dat`、
//! `Varkalandar/pak144.Excentrique:src/decoration/hjm-deco.dat`、
//! `Varkalandar/pak72.Elegance:src/deco/random_fields.dat`）でも`obj=ground_obj`が
//! 使われていることを確認した。`obj=groundobj`（アンダースコアなし）のGitHub code
//! search結果は0件だった。`tests/fmt.rs`の
//! `reorder_unsupported_obj_falls_back_to_preserve_order`テストが旧来
//! `"groundobj"`という文字列を「まだ未対応のobj種別」の例として使っていたのは、
//! この文字列が単に登録されていなかったことを示すプレースホルダに過ぎず、
//! 正しい`obj=`文字列を表していたわけではない。本実装により`"groundobj"`は
//! 意味のある文字列ではなくなったため、同テストは別の未対応文字列に更新した。
//!
//! ## `groundobj_writer_t::write_obj`（groundobj_writer.cc:17-115）の構造
//!
//! - `climates`（`obj.get("climates")`）: `tabfileobj_t::get()`はキー欠落時にも
//!   **NULLではなく空文字列**を返す（tabfile.cc:48-56）。ソースの
//!   `if (climate_str) { ... } else { dbg->warning(...,"No climates (using
//!   default)!"); ... }`という分岐は、`climate_str`が常に非NULLのポインタである
//!   ため**常にtrue側に入り、warning分岐は到達しない**（climatesキーが未指定でも
//!   `get_climate_bits("")`が呼ばれるだけで、これはSTRICMPが何にも一致せず
//!   `uv16=0`のまま返るのみでfatal/warningを出さない）。よって「climates未指定で
//!   警告」というルールはmakeobj時点の実際の挙動と一致しないため実装しない
//!   （下記REJECTED参照）。
//! - `seasons`（`get_int("seasons", 1)`）・`distributionweight`
//!   （`get_int("distributionweight", 3)`）・`cost`（`get_int64("cost", 0)`）・
//!   `speed`（`get_int("speed", 0)`）・`trees_on_top`
//!   （`get_int("trees_on_top", 1) != 0`）は全て無条件フォールバックのみで読まれ、
//!   `get_int_clamped`は一切使われていない（groundobj_writer.cc:34-46に出てくる
//!   数値フィールドはこれで全てである）。bridgeの`ClampedRangeRule`に相当する
//!   ルールはgroundobjには存在しない（下記REJECTED参照）。
//! - `waytype`（groundobj_writer.cc:49-50）は他のobj種別と**明確に異なる**分岐を持つ:
//!   ```text
//!   char const* const waytype_txt = obj.get("waytype");
//!   waytype_t   const waytype     = waytype_txt && waytype_txt[0] != '\0'
//!                                    ? get_waytype(waytype_txt) : ignore_wt;
//!   ```
//!   `obj.get()`はNULLを返さない（常に非NULLの`const char*`）ため、
//!   `waytype_txt && ...`の左辺は常にtrueで、実質的な条件は
//!   `waytype_txt[0] != '\0'`（＝非空文字列かどうか）のみである。
//!   **waytypeが未指定（空文字列）の場合は`get_waytype()`自体が呼ばれず、
//!   `ignore_wt`にサイレントフォールバックする（FATALにならない）**。
//!   これはbuilding/vehicle/way/bridge/tunnel/roadsign/crossing/way-objectの
//!   いずれとも異なる（それらは全て`get_waytype(obj.get("waytype"))`を無条件に
//!   呼ぶため欠落時もFATALになる）。一方、waytypeが**非空だが不正な値**の場合は
//!   従来通り`get_waytype()`内の
//!   `dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`
//!   （get_waytype.cc:14-49）でFATAL ERRORになる。つまりgroundobjでは
//!   「waytype未指定」は許容されるが「waytype に既知13種以外の値」はFATALになる、
//!   という非対称なルールになる。
//! - 画像（groundobj_writer.cc:52-100）は`speed`の値で2つの排他的な分岐に分かれる:
//!   - **`speed == 0`（固定物、岩・草むら等）**: `image[<phase>][<season>]`を
//!     `phase=0,1,2,...`と無制限に走査する。各phaseについて`season=0`から
//!     `number_of_seasons-1`まで走査し、`image[<phase>][0]`が空文字列なら
//!     `goto finish_images`でphaseループ全体を終了する（**FATALにならない**。
//!     つまりphase=0のseason=0すら無くても、画像0枚のground_objはmakeobj時点では
//!     エラーにならない。way-objectの`frontimage[-]`が省略可能なのと同様の
//!     「最低1枚必須」チェックが存在しないパターン）。しかし`image[<phase>][0]`が
//!     **非空なのに**season>0の`image[<phase>][<season>]`が空文字列の場合は
//!     `dbg->fatal("groundobj_writer_t","Season image for season %i missing!",
//!     seasons)`でFATALになる（groundobj_writer.cc:71）。つまり「あるphaseの
//!     season 0画像を定義したら、そのphaseの残り全季節分の画像も揃えないと
//!     FATAL」というルールになる。
//!   - **`speed != 0`（移動物、鳥・羊等）**: 8方向
//!     （`dir_codes = {"s","w","sw","se","n","e","ne","nw"}`、
//!     groundobj_writer.cc:80-82）全てについて、`season=0`から
//!     `number_of_seasons-1`まで`image[<dir>][<season>]`を走査し、
//!     **season 0を含むいずれかが空文字列でも即FATAL**
//!     （`dbg->fatal("groundobj_writer_t","Season image for season %i missing
//!     (expected %s)!", seasons, buf)`、groundobj_writer.cc:94）。
//!     固定物分岐と異なりseason 0の省略による早期終了は無く、8方向×seasons全てが
//!     必須になる。
//! - 個々の画像キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-455、`imagelist_writer_t`
//!   経由で`imagelist2d_writer_t::write_obj`から呼ばれる）がファイルの存在・
//!   サイズ（128の倍数か）を検証する。これはbuilding/way/bridge/tunnel/roadsign/
//!   crossing/way-objectと共有の`common::check_image_ref`でカバーする。
//! - `cursor`/`icon`フィールドへの言及がgroundobj_writer.cc全文に一つも無く
//!   （`cursorskin_writer_t`も呼ばれない）、他のobj種別と異なりそもそも
//!   対象フィールドが存在しない（crossingと同様のパターン）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `climates`未指定の警告: ソースコード上に
//!   `dbg->warning(obj_writer_t::last_name, "No climates (using default)!")`という
//!   分岐が存在するため一見警告ルールの根拠がありそうに見えるが、
//!   `tabfileobj_t::get()`（tabfile.cc:48-56）はキー欠落時にNULLではなく
//!   空文字列`""`を返す実装であるため、`if (climate_str)`（非NULLかどうかの
//!   チェック）は常にtrueとなり、この警告分岐は実行時に到達しない
//!   （climatesが実際に未指定でも`get_climate_bits("")`が呼ばれ、
//!   `climate_bits`は単に0のまま返るだけでfatal/warningを出さない）。
//!   ソースにwarning文字列が存在すること自体は、それが実行時に到達可能かどうかの
//!   根拠にはならないため、実際の到達可能性を確認した上で見送った。
//! - `seasons`/`distributionweight`/`cost`/`speed`/`trees_on_top`の妥当性検証:
//!   いずれも`get_int`/`get_int64`で無条件に読み、`get_int_clamped`は
//!   一度も呼ばれていない（groundobj_writer.cc:34-46）。bridgeの
//!   `ClampedRangeRule`に相当する根拠が無いため見送り
//!   （way/tunnel/roadsign/good/way-objectの同種フィールドが見送られたのと同じ理由）。
//! - `waytype`が既知だが意味的に不自然な値（例: 固定物に`waytype=air`）の
//!   妥当性検証: groundobj_desc.hのコメント「meaningful air for birds, water for
//!   fish, does not matter for everything else」は用途の説明に過ぎず、makeobj側に
//!   speedとwaytypeの組み合わせを拒否する分岐は無い（get_waytype()は空文字列を
//!   `ignore_wt`にフォールバックさせ、既知13種のいずれでも受理する）。
//!   way-objectの`own_waytype`が既知だが不自然な値のケースが見送られたのと同じ理由。
//! - `trees_on_top`が0/1以外の任意の整数値を取る場合の検証:
//!   `get_int("trees_on_top", 1) != 0`というC++の比較式はどんな整数値でも
//!   0以外なら単にtrueとして扱われるだけで、fatal/warningの分岐が無い。
//! - `image[<phase>][<season>]`のphase方向（何phaseまで許容されるか）の上限検証:
//!   `for (unsigned int phase = 0; 1; phase++)`は無限ループであり、
//!   `image[<phase>][0]`が空文字列になった時点で終了するだけで、makeobj側に
//!   phase数の妥当性チェック（多すぎる/少なすぎる）は存在しない。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: groundobj_writer.cc内の`keys.append()`は
//!   固定物分岐・移動物分岐のいずれも、実際に`image_writer_t::write_obj`を
//!   呼び出す回数と常に同数のappendを行う（値が空文字列であってもappendされ、
//!   image_writer_t::write_obj側は空文字列/"-"を早期returnせず最後まで実行して
//!   countをインクリメントする。image_writer.cc:366,443-453参照）ため、
//!   `count < keys.get_count()`に到達する実行経路が無い
//!   （tunnel/crossing/way-objectの同種警告が見送られたのと同じ理由）。
//! - `cursor`/`icon`未指定検証: crossingと同じ理由
//!   （groundobj_writer.cc全文に`cursor`/`icon`への言及が一つも無く、
//!   `cursorskin_writer_t`も呼ばれない。他obj種別と異なり、そもそも対象フィールドが
//!   存在しない）。

use super::common::{KNOWN_WAYTYPES, check_image_ref};
use crate::diagnostics::Diagnostic;
use crate::i18n::{Language, t};
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// groundobj_writer.cc:80-82 の`dir_codes`配列そのもの（移動物分岐で使う8方向）。
const DIR_CODES: &[&str] = &["s", "w", "sw", "se", "n", "e", "ne", "nw"];

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypeIfPresentValidRule),
        Box::new(SeasonImageRule),
    ]
}

/// `check_way`/`check_way_obj`と対称的な薄いラッパー。
pub fn check_groundobj(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext {
        dat,
        dat_dir,
        language: crate::i18n::Language::default(),
    };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
}

/// groundobj_writer.cc:49-50: `waytype_txt && waytype_txt[0] != '\0' ?
/// get_waytype(waytype_txt) : ignore_wt`。`tabfileobj_t::get()`は欠落キーに対し
/// NULLではなく空文字列を返す（tabfile.cc:48-56）ため、実質的な条件は
/// 「waytypeが非空文字列かどうか」のみ。**waytype未指定は`ignore_wt`にサイレント
/// フォールバックしFATALにならない**（他の全obj種別と異なる非対称な挙動）。
/// waytypeが非空だが既知13種のいずれにも一致しない場合のみ、get_waytype()内の
/// dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)でFATAL ERRORになる。
struct WaytypeIfPresentValidRule;
impl Rule for WaytypeIfPresentValidRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let waytype = ctx.dat.get("waytype").unwrap_or("").to_ascii_lowercase();
        if waytype.is_empty() {
            vec![Diagnostic::info(
                "waytype-omitted",
                t!(ctx.language,
                    ja: "obj=ground_obj では waytype は省略可能です（省略時は ignore_wt にフォールバックし、\
                         FATAL ERRORにはなりません。他のobj種別と異なりwaytype必須ではありません）",
                    en: "waytype is optional for obj=ground_obj (omitting it falls back to \
                         ignore_wt and does not cause a FATAL ERROR. Unlike other obj types, \
                         waytype is not required)",
                ),
            )]
        } else if !KNOWN_WAYTYPES.contains(&waytype.as_str()) {
            vec![Diagnostic::error(
                "unknown-waytype",
                t!(ctx.language,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            )]
        } else {
            vec![Diagnostic::info("waytype-ok", format!("waytype={waytype}"))]
        }
    }
}

/// groundobj_writer.cc:52-100: `speed`（`get_int("speed", 0)`）の値で
/// 固定物分岐（speed==0）と移動物分岐（speed!=0）に分かれる。
///
/// - 固定物分岐: `image[<phase>][<season>]`をphase=0,1,2,...と走査。各phaseの
///   season 0が空文字列ならそのphaseで走査終了（FATALにならない。画像0枚も許容）。
///   season 0が非空なのに後続season（1..number_of_seasons-1）が空文字列だと
///   FATAL（"Season image for season %i missing!"）。
/// - 移動物分岐: 8方向（DIR_CODES）× season 0..number_of_seasons-1が全て必須。
///   season 0を含むいずれかが空文字列でも即FATAL
///   （"Season image for season %i missing (expected %s)!"）。
///
/// `number_of_seasons`は`get_int("seasons", 1)`（無条件フォールバック、既定値1）。
struct SeasonImageRule;
impl Rule for SeasonImageRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        let speed: i64 = dat.get("speed").unwrap_or("").trim().parse().unwrap_or(0);
        let seasons: i64 = dat
            .get("seasons")
            .unwrap_or("")
            .trim()
            .parse()
            .unwrap_or(1)
            .max(1);

        if speed == 0 {
            check_fixed_images(dat, ctx.dat_dir, seasons, &mut diags, ctx.language);
        } else {
            check_moving_images(dat, ctx.dat_dir, seasons, &mut diags, ctx.language);
        }

        diags
    }
}

/// 固定物分岐（speed==0）: phase=0,1,2,...と走査し、`image[<phase>][0]`が空文字列に
/// なった時点で走査終了。それより前のphaseで、season 0が非空なのに後続seasonが
/// 空文字列だとFATAL相当のerrorを出す。
fn check_fixed_images(
    dat: &DatFile,
    dat_dir: &Path,
    seasons: i64,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    let mut phase = 0u32;
    loop {
        let season0_key = format!("image[{phase}][0]");
        let season0 = dat.get(&season0_key).unwrap_or("");
        if season0.is_empty() {
            // groundobj_writer.cc:66-69: goto finish_images。このphase以降は走査しない。
            break;
        }
        check_image_ref(season0, dat_dir, &season0_key, diags, lang);

        for season in 1..seasons {
            let key = format!("image[{phase}][{season}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                diags.push(Diagnostic::error(
                    "missing-season-image",
                    t!(lang,
                        ja: "{key}: phase {phase} の season 0 画像は定義されていますが、\
                             season {season} の画像が未指定です。makeobjはFATAL ERRORになります\
                             （\"Season image for season {season} missing!\"）",
                        en: "{key}: phase {phase} has a season 0 image defined, but season \
                             {season} is missing. makeobj treats this as a FATAL ERROR \
                             (\"Season image for season {season} missing!\")",
                        key = key,
                        phase = phase,
                        season = season,
                    ),
                ));
            } else {
                check_image_ref(value, dat_dir, &key, diags, lang);
            }
        }

        phase += 1;
        // 安全弁: dat構文異常でphaseが際限なく増え続ける事態を避ける
        // （makeobj自身は無限ループ`for (;;phase++)`だが、通常のground_objは
        // 数phase程度であるため、実用上十分大きい上限で打ち切る）。
        if phase > 4096 {
            break;
        }
    }

    if phase == 0 {
        diags.push(Diagnostic::info(
            "no-images",
            t!(lang,
                ja: "image[0][0] が未指定です。makeobjはこれをFATALにしません（画像0枚のground_objも\
                     許容されますが、ゲーム内では何も描画されません）",
                en: "image[0][0] is unspecified. makeobj does not treat this as FATAL (a \
                     ground_obj with zero images is allowed, but nothing renders in-game)",
            ),
        ));
    }
}

/// 移動物分岐（speed!=0）: DIR_CODESの8方向 × season 0..seasons-1が全て必須。
/// いずれか1つでも空文字列ならFATAL相当のerrorを出す。
fn check_moving_images(
    dat: &DatFile,
    dat_dir: &Path,
    seasons: i64,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    for dir in DIR_CODES {
        for season in 0..seasons {
            let key = format!("image[{dir}][{season}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                diags.push(Diagnostic::error(
                    "missing-season-image",
                    t!(lang,
                        ja: "{key}: speed!=0 (移動物) の ground_obj では8方向 x seasons分の画像が\
                             全て必須です。makeobjはFATAL ERRORになります\
                             （\"Season image for season {season} missing (expected {key})!\"）",
                        en: "{key}: for ground_obj with speed!=0 (moving objects), images for all \
                             8 directions x seasons are required. makeobj treats this as a FATAL \
                             ERROR (\"Season image for season {season} missing (expected {key})!\")",
                        key = key,
                        season = season,
                    ),
                ));
            } else {
                check_image_ref(value, dat_dir, &key, diags, lang);
            }
        }
    }
}
