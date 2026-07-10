//! `obj=citycar`（プレイヤー非所有の私有車。街に自動で出現する乗用車）の検証ルール。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/citycar_writer.cc` / `citycar_writer.h` /
//! `citycar_desc.h` / `imagelist_writer.cc` / `image_writer.cc` / `obj_writer.cc` /
//! `dataobj/tabfile.cc`）を直接読んで確認した。OTRP側の個別diffはまだ行っていない
//! （building以外のobj種別と同様）。
//!
//! ## `obj=`文字列について
//!
//! `citycar_writer_t::get_type_name()`（`citycar_writer.h:32`）は`return "citycar";`を
//! そのまま返す。way-object・ground_objのようなファイル名からの単純な類推が外れる
//! 前例があったため、今回も実際に`get_type_name()`の返り値を確認した上で、
//! `citycar_writer.cc`というファイル名から素直に導ける文字列と一致することを
//! 確認している。さらにGitHub code searchで実際の公開`.dat`ファイル
//! （例: `simutrans/pak128:citycars/tk_car_2.dat`、
//! `aburch/simutrans-pak128.britain:citycars/peel-p50.dat`、
//! `jamespetts/pakbritain-experimental:citycars/reliant-robin.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/vehicles/road/_citycar.dat`）でも
//! `obj=citycar`が使われていることを確認した。
//!
//! ## `citycar_writer_t::write_obj`（citycar_writer.cc:15-56）の構造
//!
//! `citycar_desc_t`（citycar_desc.h）のdocコメントの通り、citycarは
//! 「Private city cars, not player owned. They automatically appear in cities.」
//! であり、`obj=vehicle`（プレイヤーが編成する車両）とは根本的に異なるオブジェクトである。
//! `citycar_writer.cc`全文は`obj=vehicle`の`vehicle_writer.cc`（166行超）と比べて
//! 56行しかなく、対応する概念（waytype・engine_type・freight・constraint系）が
//! ことごとく存在しない:
//!
//! - `distributionweight`（`get_int("distributionweight", 1)`）・`intro_year`/
//!   `intro_month`/`retire_year`/`retire_month`（`get_int`の無条件フォールバック、
//!   `DEFAULT_INTRO_YEAR`/`DEFAULT_RETIRE_YEAR`等の定数へフォールバック）・
//!   `speed`（`get_int("speed", 80)`、内部的に`*16`されるがdat記述者からは
//!   単純な整数値）は全て無条件フォールバックのみで読まれ、`get_int_clamped`は
//!   一切使われていない（citycar_writer.cc:19-30）。bridgeの`ClampedRangeRule`に
//!   相当するルールはcitycarには存在しない（下記REJECTED参照）。
//! - `waytype`フィールドへの言及がcitycar_writer.cc全文に一つも無く、
//!   `get_waytype()`は一切呼ばれない（goodやtreeと同様、waytypeを持たない
//!   完全なobj種別）。citycarは道路上を走る私有車だが、waytypeは常に暗黙的に
//!   道路として扱われるためdat上で指定する概念自体が無い。よって
//!   `common::KNOWN_WAYTYPES`はcitycarには適用されない。
//! - `engine_type`・`freight`・`freightimage[...]`・`freightimagetype[...]`・
//!   `constraint[prev]`/`constraint[next]`への言及もcitycar_writer.cc全文に
//!   一つも無い。citycarはプレイヤーが編成する列車と異なり単体で走る車であり、
//!   連結制約という概念自体が存在しないため、vehicleの`couplings`サブコマンドの
//!   ような連結解析はcitycarには適用されない。
//! - 画像（citycar_writer.cc:38-51）: `dir_codes = {"s","w","sw","se","n","e","ne","nw"}`
//!   の固定8方向について、`for (i = 0; i < 8; i++)`という**無条件の**ループで
//!   `image[<dir>]`を読み、値が空文字列であってもそのまま`keys`に`append`する
//!   （vehicleの`emptyimage[dir]`のような「最初の欠落で走査終了」という早期終了ロジックは
//!   citycarには存在しない。常に8方向全てをキーとして`imagelist_writer_t::write_obj`に
//!   渡す）。よって「8方向の一部だけ定義されている」という状態を検出するvehicleの
//!   `incomplete-8-direction-images`に相当するfatal分岐はcitycarには存在しない
//!   （下記REJECTED参照）。
//! - 個々の`image[<dir>]`キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc、`imagelist_writer_t`経由）が
//!   ファイルの存在・サイズ（128の倍数か）を検証する。これはbuilding/vehicle/way/
//!   bridge/tunnel/roadsign/crossing/way-object/ground_obj/treeと共有の
//!   `common::check_image_ref`でカバーする。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:24-26）の
//!   `count < keys.get_count()`という不一致警告（`dbg->warning(...,"Expected %i but
//!   found %i images (might be correct)!")`）分岐について検討したが、citycarの
//!   `keys`は常にちょうど8要素（無条件ループでappendされるため）であり、
//!   `image_writer_t::write_obj`は空文字列/`"-"`に対して早期returnせず最後まで
//!   実行してcountをインクリメントする（image_writer.cc:366,443-453、
//!   groundobj/treeで確認済みの構造と同じ）。よって`count`は常に`keys.get_count()`
//!   （=8）に到達し、この警告分岐に到達する実行経路が無い（下記REJECTED参照）。
//!   なお、非空の画像キーが不正（画像ファイルが開けない等）な場合は
//!   `image_writer_t::write_obj`が`obj_pak_exception_t`を`throw`し、
//!   これは`makeobj.cc`の`try`ブロックまで伝播してプロセス全体をエラー終了させる
//!   （個別オブジェクトの警告ではなく致命的な失敗）。この経路は既存の
//!   `common::check_image_ref`が検出する「画像ファイルが見つからない／サイズが
//!   128の倍数でない」と同じ実体である。
//! - `cursor`/`icon`フィールドへの言及がcitycar_writer.cc全文に一つも無く
//!   （`cursorskin_writer_t`も呼ばれない）、他のobj種別と異なりそもそも
//!   対象フィールドが存在しない（crossing/ground_obj/treeと同様のパターン。
//!   citycarはビルドメニューから選択して建てるものではなく、街が自動生成する
//!   NPC的な車両のため、cursor/iconという概念自体が無い）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `distributionweight`/`intro_year`/`intro_month`/`retire_year`/`retire_month`/
//!   `speed`の妥当性検証: いずれも`get_int`で無条件に読み、`get_int_clamped`は
//!   一度も呼ばれていない（citycar_writer.cc:19-30）。bridgeの`ClampedRangeRule`に
//!   相当する根拠が無いため見送り（way/tunnel/roadsign/good/way-object/groundobj/
//!   treeの同種フィールドが見送られたのと同じ理由）。
//! - 8方向`image[<dir>]`の一部欠落検証（vehicleの`incomplete-8-direction-images`
//!   相当）: vehicleの`emptyimage[dir]`は「最初に欠落した方向でループを打ち切る」
//!   走査ロジック（vehicle_writer.cc:202-218）を持ち、4方向以上7方向以下で
//!   止まっている状態をFATALにする分岐がある。しかしcitycarの
//!   `for (i = 0; i < 8; i++)`ループ（citycar_writer.cc:38-46）にはそのような
//!   早期終了・不完全性チェックが存在せず、常に8方向全てを（空文字列であっても）
//!   無条件に`keys`へappendするだけである。よって「8方向のうち一部だけ定義されている」
//!   という状態を検出しても、makeobj自体は何もfatal/warningを出さないため、
//!   vehicleと対称的なルールは成立しない。
//! - `image[<dir>]`が1つも定義されていない（8方向すべて空文字列）場合の警告:
//!   `imagelist_writer_t::write_obj`は空リストであっても`dbg->warning`/`dbg->fatal`
//!   を出さず、単に`count=0`のimagelistノードを書き込むだけ（groundobjの
//!   「画像0枚も許容される」パターンと同じ）。way-objectの`BaseImageRequiredRule`が
//!   依拠するような明示的なFATAL分岐（wayの`image[-]`のような特別扱い）は
//!   citycar_writer.cc上に存在しないため見送り。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、citycarの`keys`は
//!   常にちょうど8要素であり、`image_writer_t::write_obj`が空文字列に対して
//!   countをスキップすることも無いため、`count < keys.get_count()`に到達する
//!   実行経路が無い（tunnel/crossing/way-object/groundobj/treeの同種警告が
//!   見送られたのと同じ理由）。
//! - `name`未指定チェック: goodと同じ理由（`text_writer_t::write_obj`は空文字列を
//!   無条件に許容し、fatal/warningを出さない。obj_writer.cc:62-70,
//!   text_writer.cc:12-23）。
//! - `waytype`/`engine_type`/`freight`関連の検証: goodやtreeと同じ理由
//!   （citycar_writer.cc全文にこれらのフィールドへの言及が一つも無い）。
//! - `constraint[prev]`/`constraint[next]`の連結制約解析（`couplings`サブコマンド
//!   相当）: citycarはプレイヤーが編成する概念を持たず、citycar_writer.cc全文に
//!   constraint系フィールドへの言及が無い。vehicleの`couplings`サブコマンドが
//!   対象とする「有限な編成として絶対に成立しない車両」という問題設定自体が
//!   citycarには存在しない。
//! - `cursor`/`icon`未指定検証: crossing/ground_obj/treeと同じ理由
//!   （citycar_writer.cc全文に`cursor`/`icon`への言及が一つも無く、
//!   `cursorskin_writer_t`も呼ばれない。他obj種別と異なり、そもそも対象フィールドが
//!   存在しない）。

use super::common::{
    DIR_CODES, NameAndCopyrightStringFieldRule, check_date_index_overflow_field, check_image_ref,
    check_narrow_int_overflow_field,
};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::t;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(DirectionImageRefRule),
        Box::new(DateIndexOverflowRule),
        Box::new(NameAndCopyrightStringFieldRule),
        Box::new(DistributionWeightNarrowIntRule),
    ]
}

/// `tests/citycar_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_citycar(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("citycar", dat, dat_dir)
}

/// citycar_writer.cc:38-46: 8方向全てについて`image[<dir>]`を無条件に読む
/// （欠落時は空文字列としてそのまま`keys`にappendされ、fatal/warningにはならない）。
/// このルールは「欠落」自体は検出せず、非空の画像キーが実際に画像ファイルを指す
/// 場合の存在確認・サイズ確認（`common::check_image_ref`、他obj種別と共有）のみ行う。
struct DirectionImageRefRule;
impl Rule for DirectionImageRefRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        for dir in DIR_CODES {
            let key = format!("image[{dir}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                diags.push(Diagnostic::debug(
                    DiagnosticCode::ImageOmitted,
                    t!(ctx.language,
                        ja: "{key} が未指定です。makeobjはこれをFATALにしません\
                             （citycarの8方向画像は個別に省略可能です）",
                        en: "{key} is unspecified. makeobj does not treat this as FATAL \
                             (each of citycar's 8 image directions can be omitted individually)",
                        key = key,
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

        diags
    }
}

/// `citycar_writer.cc:21-27`: intro_date/retire_dateがそれぞれ`year*12+month-1`で
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

/// `citycar_writer.cc:19`: `distributionweight`（`uint16`）は
/// `obj.get_int("distributionweight", 1)`（範囲チェック無しの無条件フォールバック）で
/// 読まれた後、`node.write_uint16`へ無条件に代入される。根拠・設計は
/// `common::check_narrow_int_overflow_field`のdocコメント参照
/// （`DateIndexOverflowRule`と同種の静的解析ルール）。
struct DistributionWeightNarrowIntRule;
impl Rule for DistributionWeightNarrowIntRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        check_narrow_int_overflow_field(ctx.dat, "distributionweight", 1, 16, false, ctx.language)
    }
}
