//! `obj=pedestrian`（プレイヤー非所有のNPC歩行者。街路上に自動で出現する）の検証ルール。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/pedestrian_writer.cc` / `pedestrian_writer.h` /
//! `descriptor/pedestrian_desc.h` / `imagelist_writer.cc` / `imagelist2d_writer.cc` /
//! `image_writer.cc` / `obj_writer.cc` / `dataobj/tabfile.cc`）を直接読んで確認した。
//! OTRP側の個別diffはまだ行っていない（citycar以降の全obj種別と同様）。
//!
//! ## `obj=`文字列について
//!
//! `pedestrian_writer_t::get_type_name()`（`pedestrian_writer.h:27`）は
//! `return "pedestrian";`をそのまま返す。way-object・ground_objのようなファイル名
//! からの単純な類推が外れる前例があったため、今回も実際に`get_type_name()`の
//! 返り値を確認した上で、`pedestrian_writer.cc`というファイル名から素直に導ける
//! 文字列と一致することを確認している。さらにGitHub code searchで実際の公開
//! `.dat`ファイル（例: `simutrans/pak128:base/pedestrians/Pedestrians.dat`、
//! `aburch/simutrans-pak128.britain:pedestrians/Pedestrians.dat`、
//! `jamespetts/pakbritain-experimental:pedestrians/Pedestrians.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/48/landscape/pedestrians/pedestrian.dat`、
//! `wa-st/pak-nippon:dat/pedestrian/pedestrian.np.dat`）でも`obj=pedestrian`が
//! 使われていることを確認した（`Obj=pedestrian`と大文字小文字は混在するが、
//! `tabfile_t`はキーを小文字化して読むため同じ値として扱われる）。
//!
//! ## `pedestrian_writer_t::write_obj`（pedestrian_writer.cc:15-90）の構造
//!
//! citycarと同じく「Private, not player owned. Automatically appear」なNPC的
//! obj種別であり、`obj=vehicle`のような概念（waytype・engine_type・freight・
//! constraint系）はことごとく存在しない。pedestrian_writer.cc全文は90行で、
//! citycar_writer.cc（56行）よりやや長いが、これは後述する「アニメーション画像」
//! 分岐が追加されているためである。
//!
//! - `distributionweight`（`get_int("distributionweight", 1)`）・`offset`
//!   （`get_int("offset", 20)`）・`intro_year`/`intro_month`/`retire_year`/
//!   `retire_month`（`get_int`の無条件フォールバック、`DEFAULT_RETIRE_YEAR`等の
//!   定数へフォールバック）は全て無条件フォールバックのみで読まれ、
//!   `get_int_clamped`は一切使われていない（pedestrian_writer.cc:23,71-79）。
//!   bridgeの`ClampedRangeRule`に相当するルールはpedestrianには存在しない
//!   （下記REJECTED参照）。
//! - `waytype`フィールドへの言及がpedestrian_writer.cc全文に一つも無く、
//!   `get_waytype()`は一切呼ばれない（citycar・good・treeと同様）。よって
//!   `common::KNOWN_WAYTYPES`はpedestrianには適用されない。
//! - `engine_type`・`freight`・`freightimage[...]`・`freightimagetype[...]`・
//!   `constraint[prev]`/`constraint[next]`への言及もpedestrian_writer.cc全文に
//!   一つも無い（citycarと同様、連結制約という概念自体が存在しない）。
//! - 画像（pedestrian_writer.cc:33-69）: citycarと異なり、**アニメーション画像
//!   の有無で2つの排他的な分岐**を持つ。まず`dir_codes = {"s","w","sw","se",
//!   "n","e","ne","nw"}`の固定8方向全てについて`image[<dir>][0]`（フレーム0）
//!   が非空かどうかを調べ（pedestrian_writer.cc:36-40）、1つでも非空なら
//!   `is_animated`が真になる:
//!   - **`is_animated`が偽（全方向で`image[<dir>][0]`が空。静止画像、pak128の
//!     実例はすべてこちら）**: `image[<dir>]`（フレーム添字なし）を8方向全てに
//!     ついて無条件に読む（pedestrian_writer.cc:55-59）。citycarの
//!     `image[<dir>]`ループと**全く同じ構造**（早期終了なし、常に8キー全てを
//!     `keys`にappend）。
//!   - **`is_animated`が真（いずれかの方向で`image[<dir>][0]`が非空）**:
//!     8方向それぞれについて`image[<dir>][0]`, `image[<dir>][1]`, ... と
//!     フレーム番号0から499まで走査し、最初に空文字列になったフレームで
//!     その方向の走査を打ち切る（pedestrian_writer.cc:44-53、`break`。
//!     goto/fatalではなくループの`break`のみ）。**ある方向が
//!     `image[<dir>][0]`から空文字列の場合、その方向は0フレームのまま
//!     （空のフレームリスト）で次の方向に進むだけであり、fatal/warningは
//!     一切出ない**（groundobjの`speed!=0`分岐のような「8方向全て必須」
//!     FATALはpedestrianには存在しない）。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告（`dbg->warning(...,"Expected %i
//!   but found %i images (might be correct)!")`）分岐について検討したが、
//!   citycarと同じ理由（`image_writer_t::write_obj`は空文字列/`"-"`に対して
//!   早期returnせず最後まで実行してcountをインクリメントする、
//!   image_writer.cc:366,443-453）でこの警告分岐に到達する実行経路が無い
//!   （静止分岐は`keys`が常にちょうど8要素、アニメーション分岐は各方向の
//!   `imagelist_writer_t::write_obj`が`imagelist2d_writer_t::write_obj`
//!   経由で個別に呼ばれ、`keys_animated.at(i)`の要素数と実際に書き込む
//!   枚数が常に一致する。下記REJECTED参照）。
//! - `steps_per_frame`（`is_animated ? max(obj.get_int("steps_per_frame", 1),
//!   1) : 0`、pedestrian_writer.cc:62）は`tabfileobj_t::get_int_clamped()`
//!   ではなく、C++標準の`max()`関数によるインライン下限クランプであり、
//!   **`dbg->warning`等のメッセージ出力を一切伴わない**（このプロジェクトが
//!   これまで`ClampedRangeRule`として扱ってきた`get_int_clamped`とは異なる
//!   コードパス）。加えて`is_animated`が偽の場合はこの式自体が評価されず
//!   `steps_per_frame=0`に固定される。下記REJECTED参照。
//! - 個々の`image[<dir>]`または`image[<dir>][<frame>]`キーが実際に画像を指す
//!   場合（空文字列でない場合）は、`image_writer_t::write_obj`
//!   （image_writer.cc、`imagelist_writer_t`/`imagelist2d_writer_t`経由）が
//!   ファイルの存在・サイズ（128の倍数か）を検証する。これは
//!   building/way/bridge/tunnel/roadsign/crossing/way-object/ground_obj/tree/
//!   citycarと共有の`common::check_image_ref`でカバーする。
//! - `cursor`/`icon`フィールドへの言及がpedestrian_writer.cc全文に一つも無く
//!   （`cursorskin_writer_t`も呼ばれない）、citycar/crossing/ground_obj/tree
//!   と同様、そもそも対象フィールドが存在しない（歩行者はビルドメニューから
//!   選択して建てるものではなく、街が自動生成するNPCのため）。
//! - `name`未指定チェック: goodやcitycarと同じ理由（`write_name_and_copyright`
//!   経由の`text_writer_t::write_obj`は空文字列を無条件に許容し、
//!   fatal/warningを出さない。obj_writer.cc:62-70, text_writer.cc:12-23）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `distributionweight`/`offset`/`intro_year`/`intro_month`/`retire_year`/
//!   `retire_month`の妥当性検証: いずれも`get_int`で無条件に読み、
//!   `get_int_clamped`は一度も呼ばれていない（pedestrian_writer.cc:23,71-79）。
//!   bridgeの`ClampedRangeRule`に相当する根拠が無いため見送り（citycar/way/
//!   tunnel/roadsign/good/way-object/groundobj/treeの同種フィールドが
//!   見送られたのと同じ理由）。
//! - `steps_per_frame`が0または負の値を指定した場合の警告:
//!   `max(obj.get_int("steps_per_frame", 1), 1)`はC++標準の`max()`による
//!   インラインの下限クランプであり、`tabfileobj_t::get_int_clamped()`が
//!   内部で呼ぶ`dbg->warning(...)`のようなメッセージ出力を一切伴わない。
//!   このプロジェクトの`ClampedRangeRule`は「`get_int_clamped`呼び出しである
//!   （＝dbg->warningという観測可能な根拠がある）」ことを一貫した採用基準に
//!   してきた（bridge.rs冒頭コメント参照）が、`max()`によるサイレントな
//!   クランプはその基準を満たさない、根拠の弱いパターンである。またこの
//!   フィールドは`is_animated`が真の場合にしか評価されず、pak128の実例は
//!   全て静止画像（`is_animated`が偽）でありこのフィールド自体を使っていない
//!   ため、実務上の影響も限定的と判断し見送った。
//! - 8方向`image[<dir>]`（静止分岐）の一部欠落検証（vehicleの
//!   `incomplete-8-direction-images`相当）: citycarと全く同じ理由。
//!   `for (i = 0; i < 8; i++)`ループ（pedestrian_writer.cc:55-59）には
//!   早期終了・不完全性チェックが存在せず、常に8方向全てを（空文字列で
//!   あっても）無条件に`keys`へappendするだけである。
//! - アニメーション分岐（`is_animated`が真）で、ある方向だけ`image[<dir>][0]`
//!   が空（＝その方向のみ0フレーム）という「方向間の不整合」の検証:
//!   `for (i = 0; i < 8; i++) { for (j = 0; j<500; j++) { ...; if (str.empty())
//!   break; } }`（pedestrian_writer.cc:42-54）は方向ごとに独立した
//!   走査・`break`であり、他方向のフレーム数と比較したり、0フレームの方向を
//!   fatal/warningにしたりする分岐が存在しない。実行時の見た目は不自然
//!   （ある方向だけアニメーションしない歩行者）になり得るが、makeobj時点では
//!   検出不能。
//! - `image[<dir>]`（静止分岐）または`image[<dir>][<frame>]`
//!   （アニメーション分岐）が1つも定義されていない場合の警告:
//!   `imagelist_writer_t::write_obj`/`imagelist2d_writer_t::write_obj`は
//!   空リストであっても`dbg->warning`/`dbg->fatal`を出さない
//!   （citycar・groundobjの「画像0枚も許容される」パターンと同じ）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、静止分岐の`keys`は
//!   常にちょうど8要素、アニメーション分岐の各方向`keys_animated.at(i)`も
//!   実際に呼ばれる`image_writer_t::write_obj`の回数と常に一致するため、
//!   `count < keys.get_count()`に到達する実行経路が無い（citycar/tunnel/
//!   crossing/way-object/groundobj/treeの同種警告が見送られたのと同じ理由）。
//! - `waytype`/`engine_type`/`freight`関連の検証: goodやtree・citycarと同じ理由
//!   （pedestrian_writer.cc全文にこれらのフィールドへの言及が一つも無い）。
//! - `constraint[prev]`/`constraint[next]`の連結制約解析（`couplings`
//!   サブコマンド相当）: citycarと同じ理由（プレイヤーが編成する概念を持たず、
//!   pedestrian_writer.cc全文にconstraint系フィールドへの言及が無い）。
//! - `cursor`/`icon`未指定検証: crossing/ground_obj/tree/citycarと同じ理由
//!   （pedestrian_writer.cc全文に`cursor`/`icon`への言及が一つも無く、
//!   `cursorskin_writer_t`も呼ばれない。他obj種別と異なり、そもそも
//!   対象フィールドが存在しない）。

use super::common::{DIR_CODES, check_date_index_overflow_field, check_image_ref};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::{Language, t};
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// アニメーションフレームの上限（pedestrian_writer.cc:44の`for (uint16 j = 0;
/// j<500; j++)`そのもの）。
const MAX_ANIMATION_FRAMES: u32 = 500;

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(DirectionImageRefRule),
        Box::new(DateIndexOverflowRule),
    ]
}

/// `tests/pedestrian_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_pedestrian(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("pedestrian", dat, dat_dir)
}

/// pedestrian_writer.cc:33-69: まず8方向全てについて`image[<dir>][0]`が非空か
/// 調べ（is_animated判定）、1つでも非空なら全体をアニメーション分岐として扱う。
///
/// - 静止分岐（is_animated偽）: `image[<dir>]`を8方向全て無条件に読む
///   （citycarと同じ構造。欠落自体はfatal/warningにならない）。
/// - アニメーション分岐（is_animated真）: `image[<dir>][<frame>]`を
///   frame=0から最初の空文字列まで方向ごとに走査する（欠落・方向間の
///   フレーム数不一致もfatal/warningにならない）。
///
/// このルールは「欠落」自体は検出せず、非空の画像キーが実際に画像ファイルを
/// 指す場合の存在確認・サイズ確認（`common::check_image_ref`、他obj種別と共有）
/// のみ行う。
struct DirectionImageRefRule;
impl Rule for DirectionImageRefRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        // pedestrian_writer.cc:34-40: is_animated判定（8方向いずれかで
        // image[<dir>][0]が非空か）。
        let is_animated = DIR_CODES.iter().any(|dir| {
            let key = format!("image[{dir}][0]");
            !dat.get(&key).unwrap_or("").is_empty()
        });

        if is_animated {
            check_animated_images(dat, ctx.dat_dir, &mut diags, ctx.language);
        } else {
            check_static_images(dat, ctx.dat_dir, &mut diags, ctx.language);
        }

        diags
    }
}

/// 静止分岐（is_animated偽）: pedestrian_writer.cc:55-59。citycarの
/// `DirectionImageRefRule`と全く同じ構造。
fn check_static_images(dat: &DatFile, dat_dir: &Path, diags: &mut Vec<Diagnostic>, lang: Language) {
    for dir in DIR_CODES {
        let key = format!("image[{dir}]");
        let value = dat.get(&key).unwrap_or("");
        if value.is_empty() {
            diags.push(Diagnostic::debug(
                DiagnosticCode::ImageOmitted,
                t!(lang,
                    ja: "{key} が未指定です。makeobjはこれをFATALにしません\
                         （pedestrianの8方向静止画像は個別に省略可能です）",
                    en: "{key} is unspecified. makeobj does not treat this as FATAL \
                         (each of pedestrian's 8 static-image directions can be omitted individually)",
                    key = key,
                ),
            ));
        } else {
            check_image_ref(value, dat_dir, &key, diags, lang);
        }
    }
}

/// アニメーション分岐（is_animated真）: pedestrian_writer.cc:42-54。方向ごとに
/// frame=0から最初の空文字列（またはMAX_ANIMATION_FRAMES）まで走査する。
fn check_animated_images(
    dat: &DatFile,
    dat_dir: &Path,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    for dir in DIR_CODES {
        let mut frame = 0u32;
        while frame < MAX_ANIMATION_FRAMES {
            let key = format!("image[{dir}][{frame}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                if frame == 0 {
                    diags.push(Diagnostic::debug(
                        DiagnosticCode::ImageOmitted,
                        t!(lang,
                            ja: "{key} が未指定です。makeobjはこれをFATALにしません\
                                 （このpedestrianはアニメーション画像を使用しており、\
                                 他方向でimage[<dir>][0]が定義されているため\
                                 アニメーション分岐に入りますが、{dir}方向は\
                                 0フレームのまま許容されます）",
                            en: "{key} is unspecified. makeobj does not treat this as FATAL \
                                 (this pedestrian uses animated images, since image[<dir>][0] is \
                                 defined for another direction, entering the animation branch, \
                                 but direction {dir} is allowed to remain at 0 frames)",
                            key = key,
                            dir = dir,
                        ),
                    ));
                }
                break;
            }
            check_image_ref(value, dat_dir, &key, diags, lang);
            frame += 1;
        }
    }
}

/// `pedestrian_writer.cc:73-79`: intro_date/retire_dateがそれぞれ計算されuint16に
/// 無条件代入される。intro_yearのみ他のobj種別と異なり`DEFAULT_INTRO_YEAR`
/// （1900）ではなく`1`がデフォルト値である点に注意（`obj.get_int("intro_year", 1)`、
/// コメント「no DEFAULT_INTRO_DATE here」の通り）。根拠・設計は
/// `common::check_date_index_overflow_field`のdocコメント参照
/// （`PowerGearMismatchRule`と同種の静的解析ルール）。
struct DateIndexOverflowRule;
impl Rule for DateIndexOverflowRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        diags.extend(check_date_index_overflow_field(
            dat,
            "intro_year",
            1,
            Some("intro_month"),
            ctx.language,
        ));
        diags.extend(check_date_index_overflow_field(
            dat,
            "retire_year",
            2999,
            Some("retire_month"),
            ctx.language,
        ));
        diags
    }
}
