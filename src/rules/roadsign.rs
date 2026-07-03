//! `obj=roadsign` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/roadsign_writer.cc` / `roadsign_writer.h` /
//! `roadsign_desc.h` / `get_waytype.cc` / `imagelist_writer.cc` / `image_writer.cc` /
//! `dataobj/tabfile.cc`）を直接読んで確認した。OTRP側の個別diffはまだ行っていない
//! （vehicle/way/good/bridge/tunnelと同様）。
//!
//! `roadsign_writer_t::write_obj`（roadsign_writer.cc:81-162）は他のobj種別と構造が
//! 大きく異なる:
//!
//! - `min_speed`（`get_int`）・`offset_left`（`get_int`）・`cost`（`get_int64`）・
//!   `maintenance`（`get_int64`）・`is_signal`/`free_route`/`is_presignal`/
//!   `is_prioritysignal`/`is_longblocksignal`/`single_way`/`is_private`/
//!   `no_foreground`/`end_of_choose`（いずれも`get_int`でフラグ判定に使うのみ）・
//!   `intro_year`/`intro_month`/`retire_year`/`retire_month`（いずれも`get_int`）は
//!   **全て**無条件フォールバックのみで読まれ、`tabfileobj_t::get_int_clamped()`は
//!   一切呼ばれていない（roadsign_writer.cc:83-132に出てくる数値フィールドは
//!   これで全てである）。つまりbridgeの`ClampedRangeRule`に相当するルールは
//!   roadsignには存在しない（根拠不在のため実装しない。下記REJECTED参照）。
//! - `waytype`は`get_waytype(obj.get("waytype"))`（roadsign_writer.cc:87）を
//!   無条件に呼ぶ。vehicle/way/bridge/tunnelと全く同じ`get_waytype()`関数を経由する
//!   ため、欠落・不正値は`dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`
//!   になる（get_waytype.cc:14-49、tabfileobj_t::get()はキー欠落時に空文字列を
//!   返すのみでNULLは返さない）。
//!
//! 画像はbridge/tunnelとは全く異なる形式で、**2種類の排他的な構文**を持つ
//! （roadsign_writer.cc:139-148で分岐、どちらを使うかは`image[0]`の有無で決まる）。
//!
//! **numbered構文**（`image[0]`が非空のとき、`parse_images_numbered`
//! roadsign_writer.cc:62-78が使われる）: `image[0]`, `image[1]`, ... と
//! 連番で走査し、最初に空のキーに当たった時点（インデックス`i`）でループを
//! 止める。このとき`i % 4 != 0`なら
//! `dbg->fatal("roadsign_writer", "image count is %d but must be multiple of 4!", i)`
//! になる（`i % 4 == 0`、つまり0/4/8/...枚集まった時点での終了はfatalにならない）。
//!
//! **2D構文**（`image[0]`が空のとき、`parse_images_2d`
//! roadsign_writer.cc:22-59が使われる。roadsignの標準的な書式）:
//! 方向セットは`flags`（`is_private`→`PRIVATE_ROAD`）と`image[ne][0]`の有無で
//! 決まる3択（roadsign_writer.cc:27-40）:
//!
//! - `is_private=1`（`PRIVATE_ROAD`フラグ）: 方向=`["ns","ew"]`（dir_cnt=2）
//! - `image[ne][0]`が非空（信号機ではなく、trafficlightと推定）:
//!   方向=`["n","s","w","e","nw","se","sw","ne"]`（dir_cnt=8）
//! - それ以外（通常の道路標識・鉄道信号）: 方向=`["n","s","w","e"]`（dir_cnt=4）
//!
//! `state=0..8`×`idx=0..dir_cnt`の二重ループで`image[{方向}][{state}]`を走査し、
//! 空のキーに当たったとき:
//!
//! - `idx==0`（そのstateの最初の方向）かつ`state > (dir_cnt==2 ? 1 : 0)`
//!   （dir_cnt=2の私有地標識ならstate>=2、それ以外はstate>=1）ならループを
//!   打ち切るだけ（"以降のstateは無い"とみなす、fatalにならない）。
//! - それ以外（＝最初のstate(0)、または私有地標識のstate 0/1が丸ごと欠けている、
//!   または`idx>0`つまりその行の途中で空になった＝直進方向は有るのに他方向が
//!   無い）は`dbg->fatal("roadsign_writer", "%s is missing!", buf)`。
//!
//! つまり: state=0の全方向（dir_cnt個）は必須。私有地標識の場合はstate=1の全方向も
//! 必須。それ以降のstateは「そのstateの最初の方向のキーが有るか無いか」で
//! 有効/終了を判定するが、途中の方向だけ欠けているとfatalになる。
//!
//! 個々の画像キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-439）がファイルの存在・
//!   サイズ（128の倍数か）を検証する。これはbuilding/way/bridge/tunnelと共有の
//!   `common::check_image_ref`でカバーする（roadsignの画像キーには`"-"`による
//!   明示スキップの慣習は無いが、空文字列チェックのみ共通化する）。
//!
//! `cursor`/`icon`は他のobj種別と異なり、**どちらか一方でも非空なら**
//! `cursorskin_writer_t::instance()->write_obj`を呼ぶ（roadsign_writer.cc:154-158、
//! `if (*c || *i)`）。呼ばれた場合の内部処理はway/bridge/tunnelと同じ
//! `skin_writer_t::write_obj`経由でfatal/warningを出さない。呼ばれない場合
//! （両方空）も単に何もしないだけで、fatal/warningにはならない。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `min_speed`（`get_int("min_speed", 0)`）・`offset_left`
//!   （`get_int("offset_left", 14)`）・`cost`（`get_int64`）・`maintenance`
//!   （`get_int64`）・`intro_year`/`intro_month`/`retire_year`/`retire_month`
//!   （いずれも`get_int`）の妥当性検証: roadsign_writer.cc全文を精読したが、
//!   これらの数値フィールドは全て無条件フォールバックのみで読まれており
//!   `get_int_clamped`は一度も呼ばれていない（roadsign_writer.cc:83-86,126-132）。
//!   bridgeの`ClampedRangeRule`に相当する根拠がroadsignには無いため見送り
//!   （way/tunnel/goodの同種フィールドが見送られたのと同じ理由）。
//! - `is_signal`/`free_route`/`is_presignal`/`is_prioritysignal`/
//!   `is_longblocksignal`/`single_way`/`is_private`/`no_foreground`/
//!   `end_of_choose`（フラグ系）の相互排他性検証（例: `is_signal=1`と
//!   `is_presignal=1`を同時指定した場合の挙動）: roadsign_writer.cc:90-113の
//!   if-elseチェーンは`is_signal`を最優先で判定し、以降の分岐は単に無視される
//!   だけ（dbg->fatal/warningは無い）。makeobjコメント自身も
//!   "this causes unused entries to give a warning that they are ignored"
//!   （roadsign_writer.cc:114）と書いているが、これは`tabfileobj_t`の未使用キー
//!   検出機構（全obj種別に共通する一般的な仕組みで、roadsign固有のfatal/warning
//!   分岐ではない）を指しているに過ぎず、read側の`tabfileobj_t`実装
//!   （本ツールのスコープ外）に依存するため見送り。
//! - numbered構文と2D構文の同時使用検証（両方の形式のキーが混在する`.dat`）:
//!   `image[0]`が非空なら2D構文のキー（`image[n][0]`等）は単に無視されて
//!   読まれないだけで、fatal/warningの分岐が無い（roadsign_writer.cc:139-148の
//!   if-elseは片方のみを評価する）。「意図しない混在」を検出する価値はありそうだが、
//!   makeobj自体がエラーにしないため、根拠不在の理由により見送り。
//! - `waytype`が`road`/`track`以外（例: `water`）のときの`image[ne][0]`分岐判定
//!   への影響検証: `parse_images_2d`の方向セット選択は`flags`と`image[ne][0]`の
//!   有無のみで決まり、`waytype`の値そのものはこの分岐に関与しない
//!   （roadsign_writer.cc:27-40参照）。よってwaytype値と画像形式の対応関係を
//!   検証する根拠はmakeobjソース上に無い。
//! - `cursor`/`icon`未指定チェック: 他のobj種別（way/bridge/tunnel）と異なり
//!   roadsignは`*c || *i`の条件分岐すらあるが、どちらのケースも
//!   （呼ばれる/呼ばれない）fatal/warningを出さない（`cursorskin_writer_t`は
//!   `skin_writer_t::write_obj`経由でname/copyrightとimagelistを書くだけ、
//!   skin_writer.cc:40-51）。way/bridge/tunnelのcursor/icon省略が見送られたのと
//!   同じ理由。

use super::common::check_image_ref;
use crate::diagnostics::Diagnostic;
use crate::i18n::t;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// roadsign_writer.cc:19 の`private_sign_directions`配列そのもの。
const PRIVATE_DIRECTIONS: &[&str] = &["ns", "ew"];
/// roadsign_writer.cc:20 の`traffic_light_directions`配列そのもの。
const TRAFFIC_LIGHT_DIRECTIONS: &[&str] = &["n", "s", "w", "e", "nw", "se", "sw", "ne"];
/// roadsign_writer.cc:21 の`general_sign_directions`配列そのもの。
const GENERAL_DIRECTIONS: &[&str] = &["n", "s", "w", "e"];

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![Box::new(WaytypeRequiredRule), Box::new(ImageRule)]
}

/// `check_bridge`/`check_tunnel`と対称的な薄いラッパー。
pub fn check_roadsign(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext {
        dat,
        dat_dir,
        language: crate::i18n::Language::default(),
    };
    all().iter().flat_map(|r| r.check(&ctx)).collect()
}

/// roadsign_writer.cc:87 は get_waytype(obj.get("waytype")) を無条件に呼ぶ
/// （vehicle/way/bridge/tunnelと同じく分岐なしで常に評価される）。get_waytype.cc:14-49は
/// STRICMPが既知13種のいずれにも一致しなければ dbg->fatal("get_waytype()","invalid
/// waytype \"%s\"\n", waytype) で落とす。tabfileobj_t::get()はNULLを返さず
/// 欠落キーには空文字列を返す（tabfile.cc:48-56）ため、waytype未指定も同じ
/// fatalパスに入る。実際のチェックロジックは`common::check_waytype_field`に
/// 集約されている（way/bridge/tunnel/roadsign/vehicle/way-object/crossingで共有）。
struct WaytypeRequiredRule;
impl Rule for WaytypeRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        super::common::check_waytype_field(ctx.dat, "waytype", ctx.language)
    }
}

/// roadsign_writer.cc:90-113: is_signal→SIGN_SIGNAL(+CHOOSE_SIGN)、
/// is_presignal→SIGN_PRE_SIGNAL、is_prioritysignal→SIGN_PRIORITY_SIGNAL、
/// is_longblocksignal→SIGN_LONGBLOCK_SIGNAL、それ以外はis_private等の組み合わせで
/// PRIVATE_ROAD等が立つ。ここでは`is_private`の有無だけがparse_images_2dの
/// 方向セット選択（PRIVATE_ROAD分岐）に影響するため、それだけを再現すれば良い。
fn is_private_road(dat: &DatFile) -> bool {
    // roadsign_writer.cc:90-113: is_signal/is_presignal/is_prioritysignal/
    // is_longblocksignalのいずれかが立っているとelseブロック（is_private等の判定）
    // 自体に入らないため、PRIVATE_ROADフラグは絶対に立たない。
    let is_signal_family = [
        "is_signal",
        "is_presignal",
        "is_prioritysignal",
        "is_longblocksignal",
    ]
    .iter()
    .any(|k| dat.get(k).unwrap_or("0").trim().parse::<i64>().unwrap_or(0) > 0);
    if is_signal_family {
        return false;
    }
    dat.get("is_private")
        .unwrap_or("0")
        .trim()
        .parse::<i64>()
        .unwrap_or(0)
        > 0
}

/// roadsign_writer.cc:139-148,22-78: `image[0]`の有無でnumbered構文/2D構文の
/// どちらが使われるかが決まる。
struct ImageRule;
impl Rule for ImageRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let image0 = dat.get("image[0]").unwrap_or("");
        if !image0.is_empty() {
            check_numbered(ctx)
        } else {
            check_2d(ctx)
        }
    }
}

/// roadsign_writer.cc:62-78 `parse_images_numbered`をそのまま再現する。
/// `image[0]`, `image[1]`, ... と連番で走査し、最初に空のキーに当たった時点の
/// インデックス`i`が4の倍数でなければ
/// `dbg->fatal("roadsign_writer", "image count is %d but must be multiple of 4!", i)`。
fn check_numbered(ctx: &RuleContext) -> Vec<Diagnostic> {
    let dat = ctx.dat;
    let mut diags = Vec::new();
    for i in 0..32 {
        let key = format!("image[{i}]");
        let value = dat.get(&key).unwrap_or("");
        if value.is_empty() {
            if i % 4 != 0 {
                diags.push(Diagnostic::error(
                    "roadsign-image-count-not-multiple-of-4",
                    t!(ctx.language,
                        ja: "image[{i}] が未指定です。numbered構文（image[0]あり）では \
                             画像枚数は4の倍数である必要があります（\"image count is {i} but \
                             must be multiple of 4!\"、roadsign_writerはFATAL ERRORにします）",
                        en: "image[{i}] is unspecified. In the numbered syntax (image[0] present), \
                             the image count must be a multiple of 4 (\"image count is {i} but \
                             must be multiple of 4!\", roadsign_writer treats this as a FATAL ERROR)",
                        i = i,
                    ),
                ));
            }
            break;
        }
        check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
    }
    diags
}

/// roadsign_writer.cc:22-59 `parse_images_2d`をそのまま再現する。
fn check_2d(ctx: &RuleContext) -> Vec<Diagnostic> {
    let dat = ctx.dat;
    let mut diags = Vec::new();

    let private_road = is_private_road(dat);
    let (directions, threshold): (&[&str], u8) = if private_road {
        (PRIVATE_DIRECTIONS, 1)
    } else if !dat.get("image[ne][0]").unwrap_or("").is_empty() {
        (TRAFFIC_LIGHT_DIRECTIONS, 0)
    } else {
        (GENERAL_DIRECTIONS, 0)
    };

    'state_loop: for state in 0u8..8 {
        for (idx, dir) in directions.iter().enumerate() {
            let key = format!("image[{dir}][{state}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                if idx == 0 && state > threshold {
                    // roadsign_writer.cc:48-51: それ以降のstateは無いとみなして
                    // 走査を打ち切るだけ。fatalにはならない。
                    break 'state_loop;
                }
                diags.push(Diagnostic::error(
                    "roadsign-image-missing",
                    t!(ctx.language,
                        ja: "{key} が未指定です。roadsign_writerは2D構文（image[0]なし）でこのキーを\
                             必須として扱います（\"{key} is missing!\"、FATAL ERRORになります）",
                        en: "{key} is unspecified. roadsign_writer treats this key as required in \
                             the 2D syntax (no image[0]) (\"{key} is missing!\", this becomes a \
                             FATAL ERROR)",
                        key = key,
                    ),
                ));
            } else {
                check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
            }
        }
    }

    diags
}
