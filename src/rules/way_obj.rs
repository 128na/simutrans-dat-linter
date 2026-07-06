//! `obj=way-object`（架線柱・照明など、wayに付随して描画されるオブジェクト。
//! makeobjソース上の内部呼称は"way object"だが、`.dat`記述者が実際に書く値は
//! `obj=way-object`である。詳細は下記コメント参照）の検証ルール。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/way_obj_writer.cc` / `way_obj_writer.h` /
//! `get_waytype.cc` / `imagelist_writer.cc` / `image_writer.cc` / `skin_writer.cc` /
//! `obj_writer.cc` / `dataobj/tabfile.cc`）を直接読んで確認した。OTRP側の個別diffは
//! まだ行っていない（vehicle/way/good/bridge/tunnel/roadsign/crossingと同様）。
//!
//! ## `obj=`文字列について
//!
//! このプロジェクトの他のRustモジュール名・ファイル名は`way_obj`（スネークケース）で
//! 揃えているが、`.dat`に実際に書く`obj=`の値は**`way-object`**（ハイフン、
//! アンダースコアではない）である。根拠は`obj_writer_t::write`
//! （obj_writer.cc:39-59）が`obj.get("obj")`の文字列でそのまま
//! `writer_by_name->get(type)`（obj_writer.cc:44）を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （obj_writer.cc:31）である。`way_obj_writer_t::get_type_name()`
//! （way_obj_writer.h:31）は`return "way-object";`を返す
//! （他のwriterと比較: `crossing_writer.h`は`"crossing"`、`way_writer.h`は`"way"`、
//! `bridge_writer.h`は`"bridge"`、`tunnel_writer.h`は`"tunnel"`、
//! `roadsign_writer.h`は`"roadsign"`、`good_writer.h`は`"good"`、
//! `vehicle_writer.h`は`"vehicle"`、`building_writer.h`は`"building"`）。
//! さらに実際のpak128/pak192.comic/pak-nippon等の公開`.dat`ファイル
//! （GitHub code search、例: `simutrans/pak128:infrastructure/catenary_all/third_rail_80.dat`）
//! でも`obj=way-object`が使われていることを確認した。`tests/fmt.rs`の
//! `reorder_unsupported_obj_falls_back_to_preserve_order`テストが旧来
//! `"wayobj"`という文字列を「まだ未対応のobj種別」の例として使っていたのは、
//! この文字列が単に登録されていなかった（`RuleSet::for_obj_type`/`order_for`の
//! どちらにもマッチしない）ことを示すためのプレースホルダに過ぎず、
//! 正しい`obj=`文字列を表していたわけではない。本実装により`"wayobj"`は
//! 意味のある文字列ではなくなったため、同テストは別の未対応文字列
//! （`"groundobj"`）に更新した。
//!
//! ## `way_obj_writer_t::write_obj`（way_obj_writer.cc:22-122）の構造
//!
//! - `price`（`get_int64("cost", 100)`）・`maintenance`（`get_int64("maintenance", 100)`）・
//!   `topspeed`（`get_int("topspeed", 999)`）・`intro_year`/`intro_month`/
//!   `retire_year`/`retire_month`（いずれも`get_int`）は**全て**無条件フォールバックの
//!   みで読まれ、`tabfileobj_t::get_int_clamped()`は一切呼ばれていない
//!   （way_obj_writer.cc:32-40に出てくる数値フィールドはこれで全てである）。
//!   つまりbridgeの`ClampedRangeRule`に相当するルールはway-objectには存在しない
//!   （根拠不在のため実装しない。下記REJECTED参照）。
//! - `waytype`は`get_waytype(obj.get("waytype"))`（way_obj_writer.cc:42）を
//!   無条件に呼ぶ。way/bridge/tunnel/roadsign/crossingと全く同じ`get_waytype()`
//!   関数を経由するため、欠落・不正値は
//!   `dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`になる
//!   （get_waytype.cc:14-49、tabfileobj_t::get()はキー欠落時に空文字列を返すのみで
//!   NULLは返さない）。
//! - `own_waytype`も同様に`get_waytype(obj.get("own_waytype"))`
//!   （way_obj_writer.cc:43）を無条件に呼ぶ。way-objectは他のobj種別と異なり
//!   **waytypeフィールドが2つ**あり、`waytype`は「この線路付随物がどのwaytypeの
//!   way上に置けるか」、`own_waytype`は「この線路付随物自体が表す種別
//!   （架線なら`electrified_track`等）」を表す（way_obj_desc.hのコメント
//!   「Type of the object, only overheadlines_wt is currently used」参照）。
//!   crossingの`waytype[0]`/`waytype[1]`とはキー名・意味が異なるが、
//!   どちらも同じ`get_waytype()`を通るため欠落・不正値は同じFATALパスに入る。
//!   crossingと異なり、`waytype`と`own_waytype`が解決後の値として同一かどうかを
//!   判定するfatal分岐はway_obj_writer.cc全文に存在しない（下記REJECTED参照）。
//!
//! 画像は`ribi_codes`配列（way_obj_writer.cc:24-29、26要素:
//! `-`,`n`,`e`,`ne`,`s`,`ns`,`se`,`nse`,`w`,`nw`,`ew`,`new`,`sw`,`nsw`,`sew`,`nsew`,
//! `nse1`,`new1`,`nsw1`,`sew1`,`nsew1`,`nse2`,`new2`,`nsw2`,`sew2`,`nsew2`）を
//! 全走査する`frontimage[{ribi}]`/`backimage[{ribi}]`（26種×2 = 52キー、
//! way_obj_writer.cc:61-69）、坂道用の`frontimageup[{slope}]`/`backimageup[{slope}]`
//! （slope=3,6,9,12の4種×2 = 8キー、way_obj_writer.cc:76-84）、
//! 二重高さ坂道用の`frontimageup2[{slope}]`/`backimageup2[{slope}]`
//! （同4種×2 = 8キー、85-97、こちらは非空の場合のみappendされる点が上記2グループと
//! 異なる）、対角線用の`frontdiagonal[{ribi}]`/`backdiagonal[{ribi}]`
//! （ribi_codes[3..12]の4種×2 = 8キー、104-112）で構成される。
//!
//! これら全てのキーはwayのimage[-]（`way_writer.cc`）のような「欠落したら
//! FATAL」という明示的な分岐が無い。`front_list`/`back_list`への`append`は
//! （imageup2の2グループを除き）値が空文字列でも無条件に実行される
//! （way_obj_writer.cc:64-68,79-83,107-111には`if(!str.empty())`のような
//! ガードが無い）。よって`imagelist_writer_t::write_obj`（imagelist_writer.cc:14-35）
//! に渡される`keys`の件数（`get_count()`）は常に、実際に`image_writer_t::write_obj`
//! が呼ばれる回数（`count`）と一致する
//! （`image_writer_t::write_obj`自体は空文字列/`"-"`のキーでも早期returnせず
//! 最後まで実行され、呼び出し側ループのcountを必ずインクリメントする。
//! image_writer.cc:348-455参照）。imageup2の2グループも非空の場合のみappendする
//! ため、appendされた件数と書き込まれた件数は結果的に常に一致する。したがって
//! `count < keys.get_count()`警告（"Expected %i but found %i images"）は
//! way-objectでは発火しない（tunnel/crossingと同じ理由。下記REJECTED参照）。
//!
//! 個々の画像キーが実際に画像を指す場合（空文字列でない場合）は、
//! `image_writer_t::write_obj`（image_writer.cc:348-455）がファイルの存在・
//! サイズ（128の倍数か）を検証する。これはbuilding/way/bridge/tunnel/roadsign/
//! crossingと共有の`common::check_image_ref`でカバーする。
//!
//! `cursor`/`icon`はway_obj_writer.cc:116-119で`cursorkeys`リストに
//! **常に両方**（空文字列でも）appendされ、`cursorskin_writer_t::instance()->write_obj`
//! （実体は`skin_writer_t::write_obj(fp, parent, obj, imagekeys)`、
//! skin_writer.cc:40-51）に渡される。`imagekeys`は常に2件appendされるため
//! `imagelist_writer_t::write_obj`のcount不一致警告もここでは発火しない
//! （way/bridge/tunnel/roadsignのcursor/icon省略が見送られたのと同じ理由。
//! 下記REJECTED参照）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `cost`（`get_int64("cost", 100)`）・`maintenance`（`get_int64("maintenance", 100)`）・
//!   `topspeed`（`get_int("topspeed", 999)`）・`intro_year`/`intro_month`/
//!   `retire_year`/`retire_month`（いずれも`get_int`）の妥当性検証:
//!   way_obj_writer.cc全文を精読したが、これら7つの数値フィールドは全て
//!   無条件フォールバックのみで読まれており`get_int_clamped`は一度も呼ばれて
//!   いない（way_obj_writer.cc:32-40）。bridgeの`ClampedRangeRule`に相当する
//!   根拠がway-objectには無いため見送り（way/tunnel/roadsign/goodの同種フィールドが
//!   見送られたのと同じ理由）。
//! - `waytype`と`own_waytype`が解決後の値として同一（または特定の組み合わせ）の
//!   ときの妥当性検証: crossingの`IdenticalWaytypesRule`に相当するチェックを
//!   検討したが、`way_obj_writer.cc:42-43`はそれぞれ独立に`get_waytype()`を
//!   呼ぶだけで、2つの結果を比較するコードが無い（crossing_writer.cc:80-82の
//!   ような`dbg->fatal("Crossing", "Identical ways...")`に相当する分岐が
//!   way_obj_writer.cc全文に存在しない）。実際、pak128の実例
//!   （`third_rail_80.dat`: `waytype=track`, `own_waytype=electrified_track`）でも
//!   両者は意図的に異なる値を取る設計であり、そもそも一致させる使い方が普通ではない。
//!   makeobj側に検証根拠が無いため見送り。
//! - 画像未指定（空文字列/`"-"`）の警告: bridgeの`FrontImageWarningRule`が依拠する
//!   `dbg->warning(..., "No %s specified (might still work)", ...)`という分岐は
//!   `bridge_writer.cc`固有のコードであり、`way_obj_writer.cc`には対応する分岐が
//!   存在しない（61-69,76-84,104-112のループに`str.empty()`によるwarning呼び出しが
//!   無い）。wayの`image[-]`のような明示的なFATALも無い（下記参照）。よってbridge/way
//!   同等の"未指定検証"ルールはway-objectには追加しない。
//! - `image[-]`相当の「最低1枚必須」チェック: wayの`BaseImageRequiredRule`は
//!   `image[-]`/`image[-][0]`の両方欠落時のみ`dbg->fatal("way_writer_t::write_obj",
//!   "image with label %s missing", ...)`という明示的な分岐（way_writer.cc:84-96）に
//!   依拠しているが、way_obj_writer.cc全文にはこれに相当する分岐が無い
//!   （ribi_codes[0]="-"に対応する`frontimage[-]`/`backimage[-]`キーも他のribiと
//!   全く同列に扱われ、空でも単にappendされるだけ）。根拠不在のため見送り。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、way_obj_writerは
//!   `frontimage`/`backimage`/`frontimageup`/`backimageup`グループを常に全件
//!   （空文字列含む）append、`frontimageup2`/`backimageup2`グループを非空時のみ
//!   appendするため、いずれの場合もappend件数と書き込み件数（`image_writer_t::write_obj`
//!   の呼び出し回数）は常に一致し、この分岐に到達する実行経路が無い
//!   （tunnel/crossingの同種警告が見送られたのと同じ理由）。
//! - `cursor`/`icon`未指定検証: way/bridge/tunnel/roadsignと同じ理由
//!   （`cursorskin_writer_t`経由で空文字列を無条件許容しfatal/warningを出さない）。
//! - `own_waytype`が既知だが`way-object`として意味的に不自然な値
//!   （例: `own_waytype=air`）の妥当性検証: way_obj_desc.hのコメントは
//!   「only overheadlines_wt is currently used」と書いているが、これは
//!   「現状よく使われる値」を示すコメントであり、makeobj側にそれ以外の値を
//!   拒否する分岐は無い（get_waytype()は既知13種のいずれでも受理する）。
//!   crossingの「意味のある交差の組み合わせ」検証が見送られたのと同じ理由。

use super::common::{check_date_index_overflow_field, check_image_ref};
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// way_obj_writer.cc:24-29 の`ribi_codes`配列そのもの（26要素）。
const RIBI_CODES: &[&str] = &[
    "-", "n", "e", "ne", "s", "ns", "se", "nse", "w", "nw", "ew", "new", "sw", "nsw", "sew",
    "nsew", "nse1", "new1", "nsw1", "sew1", "nsew1", "nse2", "new2", "nsw2", "sew2", "nsew2",
];

/// way_obj_writer.cc:76,85 のslopeループ（3,6,9,12）そのもの。
const SLOPES: &[u8] = &[3, 6, 9, 12];

/// way_obj_writer.cc:104 の対角線ribiループ（ribi_codes[3..=12]、3刻み）に対応する
/// インデックス（3,6,9,12）。RIBI_CODES[3]="ne", [6]="se", [9]="sw", [12]="new"。
const DIAGONAL_RIBI_INDICES: &[usize] = &[3, 6, 9, 12];

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypeRequiredRule),
        Box::new(OwnWaytypeRequiredRule),
        Box::new(ImageRefRule),
        Box::new(DateIndexOverflowRule),
    ]
}

/// `tests/way_obj_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
/// `obj=`の実際の値は`way-object`（ハイフン区切り。モジュール名`way_obj`とは
/// 異なる点に注意）。
pub fn check_way_obj(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("way-object", dat, dat_dir)
}

/// way_obj_writer.cc:42 は get_waytype(obj.get("waytype")) を無条件に呼ぶ
/// （way/bridge/tunnel/roadsign/crossingと同じく分岐なしで常に評価される）。
/// get_waytype.cc:14-49はSTRICMPが既知13種のいずれにも一致しなければ
/// dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", waytype) で落とす。
/// tabfileobj_t::get()はNULLを返さず欠落キーには空文字列を返す（tabfile.cc:48-56）
/// ため、waytype未指定も同じfatalパスに入る。実際のチェックロジックは
/// `common::check_waytype_field`に集約されている（way/bridge/tunnel/roadsign/
/// vehicle/way-object/crossingで共有）。
struct WaytypeRequiredRule;
impl Rule for WaytypeRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        super::common::check_waytype_field(ctx.dat, "waytype", ctx.language)
    }
}

/// way_obj_writer.cc:43 は get_waytype(obj.get("own_waytype")) を無条件に呼ぶ。
/// way-objectはwaytypeフィールドが2つあり（`waytype`=乗る対象のway種別、
/// `own_waytype`=このway-object自身の種別）、どちらも同じ`get_waytype()`を経由する
/// ため、`own_waytype`の欠落・不正値も同じfatalパスに入る
/// （crossingのwaytype[0]/waytype[1]とキー名は異なるが同じ仕組み）。実際の
/// チェックロジックは`common::check_waytype_field`に集約されている
/// （keyに`"own_waytype"`を渡す点のみ`WaytypeRequiredRule`と異なる）。
struct OwnWaytypeRequiredRule;
impl Rule for OwnWaytypeRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        super::common::check_waytype_field(ctx.dat, "own_waytype", ctx.language)
    }
}

/// way_obj_writer.cc:61-114: frontimage[{ribi}]/backimage[{ribi}]（26種×2）、
/// frontimageup[{slope}]/backimageup[{slope}]（4種×2）、
/// frontimageup2[{slope}]/backimageup2[{slope}]（4種×2、非空のみ）、
/// frontdiagonal[{ribi}]/backdiagonal[{ribi}]（ribi_codes[3,6,9,12]の4種×2）の
/// 全キーについて、実際に画像を指す値（空文字列以外）のみ
/// common::check_image_refでファイル存在・サイズを検証する
/// （building/way/bridge/tunnel/roadsign/crossingと共有）。
struct ImageRefRule;
impl Rule for ImageRefRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        // frontimage[{ribi}] / backimage[{ribi}]: 26方向 × front/back。
        for ribi in RIBI_CODES {
            for prefix in ["frontimage", "backimage"] {
                let key = format!("{prefix}[{ribi}]");
                let value = dat.get(&key).unwrap_or("");
                if !value.is_empty() {
                    check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
                }
            }
        }

        // frontimageup[{slope}] / backimageup[{slope}]: slope=3,6,9,12 × front/back。
        for slope in SLOPES {
            for prefix in ["frontimageup", "backimageup"] {
                let key = format!("{prefix}[{slope}]");
                let value = dat.get(&key).unwrap_or("");
                if !value.is_empty() {
                    check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
                }
            }
        }

        // frontimageup2[{slope}] / backimageup2[{slope}]: 上と同じslope集合だが、
        // ソース側も非空のときだけappendする（way_obj_writer.cc:88-96）ため、
        // 意味的には他のグループと同じ「非空なら検証」で足りる。
        for slope in SLOPES {
            for prefix in ["frontimageup2", "backimageup2"] {
                let key = format!("{prefix}[{slope}]");
                let value = dat.get(&key).unwrap_or("");
                if !value.is_empty() {
                    check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
                }
            }
        }

        // frontdiagonal[{ribi}] / backdiagonal[{ribi}]: RIBI_CODESのインデックス
        // 3,6,9,12（"ne","se","sw","new"）× front/back。
        for &idx in DIAGONAL_RIBI_INDICES {
            let ribi = RIBI_CODES[idx];
            for prefix in ["frontdiagonal", "backdiagonal"] {
                let key = format!("{prefix}[{ribi}]");
                let value = dat.get(&key).unwrap_or("");
                if !value.is_empty() {
                    check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
                }
            }
        }

        // cursor / icon: way_obj_writer.cc:116-119で常に両方appendされるが、
        // 実際に画像を指す場合のみファイル検証の対象になる（他obj種別と同じ）。
        for key in ["cursor", "icon"] {
            let value = dat.get(key).unwrap_or("");
            if !value.is_empty() {
                check_image_ref(value, ctx.dat_dir, key, &mut diags, ctx.language);
            }
        }

        diags
    }
}

/// `way_obj_writer.cc:36-40`: intro_date/retire_dateがそれぞれ`year*12+month-1`で
/// 計算されuint16に無条件代入される。根拠・設計は
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
            1900,
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
