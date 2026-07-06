//! `obj=tunnel` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/tunnel_writer.cc` / `tunnel_writer.h` /
//! `get_waytype.cc` / `imagelist_writer.cc` / `skin_writer.cc` / `image_writer.cc` /
//! `dataobj/tabfile.cc`）を直接読んで確認した。OTRP側の個別diffはまだ行っていない
//! （vehicle/way/good/bridgeと同様）。
//!
//! `tunnel_writer_t::write_obj`（tunnel_writer.cc:20-125）はbridgeと構造は似ているが、
//! 中身はより単純である:
//!
//! - `topspeed`（`get_int`）・`cost`/`maintenance`（`get_int64`）・`axle_load`
//!   （`get_int`）・`intro_year`/`intro_month`/`retire_year`/`retire_month`
//!   （いずれも`get_int`）は**全て**無条件フォールバックのみで読まれ、
//!   `tabfileobj_t::get_int_clamped()`は一切呼ばれていない（tunnel_writer.cc:22-33に
//!   出てくる数値フィールドはこの7つで全てである）。つまりbridgeの
//!   `ClampedRangeRule`に相当するルールはtunnelには存在しない
//!   （根拠不在のため実装しない。下記REJECTED参照）。
//! - `waytype`は`get_waytype(obj.get("waytype"))`（tunnel_writer.cc:25）を
//!   無条件に呼ぶ。bridge/way/vehicleと全く同じ`get_waytype()`関数を経由するため、
//!   欠落・不正値は`dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`
//!   になる（get_waytype.cc:14-49、tabfileobj_t::get()はキー欠落時に空文字列を
//!   返すのみでNULLは返さない）。
//!
//! 画像はbridgeの`front{name}[{index}]`形式と異なり、`{front|back}image[{方向}{幅}][{season}]`
//! という2階建てのキー形式を使う（tunnel_writer.cc:36-98）:
//!
//! - 季節数`number_of_seasons`は`frontimage[n][1]`（`indices[0]="n"`,
//!   `add[1]="l"`ではなく空文字列の"n"のみ）が非空かどうかで判定する
//!   （tunnel_writer.cc:41-47。bridgeの`backimage[ns][0]`とはキー名も判定対象も異なる）。
//! - 幅（`number_portals`）は`frontimage[nl][0]`（無ければ短縮形`frontimage[nl]`）
//!   が非空かどうかで判定し、非空なら4方向×4幅（n/s/e/w × 無印/l/r/m）の
//!   broad portal、空なら4方向×1幅（無印のみ）のnarrow portalになる
//!   （tunnel_writer.cc:49-60）。
//! - 実際に走査されるキーは `season=0..=number_of_seasons` × `pos∈{front,back}` ×
//!   `j<number_portals`（0..1または0..4） × `i<4`（n/s/e/w）の全組み合わせで、
//!   `{front|back}image[{indices[i]}{add[j]}][{season}]`を読み、season==0かつ空なら
//!   短縮形`{front|back}image[{indices[i]}{add[j]}]`にフォールバックする
//!   （tunnel_writer.cc:84-97）。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:14-35）は
//!   `count < keys.get_count()`のときのみ`dbg->warning(..., "Expected %i but
//!   found %i images (might be correct)!\n", ...)`を出すが、tunnel_writerは
//!   backkeys/frontkeysへ必ず`number_portals*4`件を１対１でappendする
//!   （空文字列でもappendする、tunnel_writer.cc:95）ため、`count`は常に
//!   `keys.get_count()`と一致し、この警告分岐は実際には発火しない
//!   （bridgeの`front{name}[...]`のような`value.size() <= 2`警告自体が
//!   tunnel_writerには存在しない。下記REJECTED参照）。
//! - 個々の画像キーが実際に画像を指す場合（空文字列でも"-"でもない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-439）がファイルの存在・
//!   サイズ（128の倍数か）を検証する。これはbuilding/way/bridgeと共有の
//!   `common::check_image_ref`でカバーする。
//!
//! `cursor`/`icon`は`cursorskin_writer_t::instance()->write_obj`
//! （tunnel_writer.cc:107、season==0のときのみ）経由で、`skin_writer_t::write_obj`
//! （skin_writer.cc:40-51）の`write_name_and_copyright` + `imagelist_writer_t::write_obj`
//! を呼ぶだけであり、cursor/iconが空文字列でもfatal/warningを出さない
//! （way/bridgeのcursor/icon省略が見送られたのと同じ理由。下記REJECTED参照）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `topspeed`（`get_int("topspeed", 999)`）・`cost`/`maintenance`
//!   （`get_int64`）・`axle_load`（`get_int("axle_load", 9999)`）・
//!   `intro_year`/`intro_month`/`retire_year`/`retire_month`（いずれも`get_int`）の
//!   妥当性検証: tunnel_writer.cc全文を精読したが、これら7つの数値フィールドは
//!   全て無条件フォールバックのみで読まれており`get_int_clamped`は一度も
//!   呼ばれていない（tunnel_writer.cc:22-33）。bridgeの`ClampedRangeRule`に
//!   相当する根拠がtunnelには無いため見送り（wayのtopspeed等・goodのvalue等が
//!   見送られたのと同じ理由）。
//! - 画像未指定（空文字列/"-"）の警告: bridgeの`FrontImageWarningRule`が依拠する
//!   `dbg->warning(..., "No %s specified (might still work)", ...)`という
//!   分岐は`bridge_writer.cc`固有のコードであり、`tunnel_writer.cc`には
//!   対応する分岐が存在しない（tunnel_writer.cc:84-98のループには
//!   `str.empty()`によるwarning呼び出しが無い。空文字列はそのまま
//!   `frontkeys`/`backkeys`にappendされ、後段の`imagelist_writer_t::write_obj`も
//!   count不一致にならないため警告に至らない）。よってbridge同等の
//!   "未指定警告"ルールはtunnelには追加しない。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: 上記の通り、tunnel_writerは
//!   `number_portals*4`件を必ず1対1でappendするため、count（実際にwriteされた
//!   枚数）とkeys.get_count()（append件数）は常に一致し、この分岐に到達する
//!   実行経路が無い（image_writer_t::write_obj自体は空/-キーでもcountを
//!   インクリメントする。image_writer.cc:348-439内に早期returnやcountを
//!   飛ばす分岐が無いことを確認済み）。
//! - `cursor`/`icon`未指定チェック: `cursorskin_writer_t`は`skin_writer_t`の
//!   `write_obj(fp, parent, obj, imagekeys)`オーバーロードをそのまま使い、
//!   `write_name_and_copyright` + `imagelist_writer_t::write_obj`のみで
//!   fatal/warningを出さない（skin_writer.cc:40-51）。way/bridgeのcursor/icon
//!   省略チェックが見送られたのと全く同じ理由。
//! - `way=`（地下ウェイオブジェクトへの参照）の実在性検証: `xref_writer_t::write_obj`
//!   （tunnel_writer.cc:113、fatal引数=false）は参照を検証せずゲーム読み込み時まで
//!   遅延する。goodのfreight参照・vehicleのconstraint参照が見送られたのと同じ
//!   理由（ディレクトリ横断のレジストリが無い現状のスコープ外）。

use super::common::{check_date_index_overflow_field, check_image_ref};
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// tunnel_writer.cc:36-37 の`indices`/`add`配列そのもの。
const DIRECTIONS: &[&str] = &["n", "s", "e", "w"];
const WIDTHS: &[&str] = &["", "l", "r", "m"];

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypeRequiredRule),
        Box::new(ImageRefRule),
        Box::new(DateIndexOverflowRule),
    ]
}

/// `tests/tunnel_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_tunnel(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("tunnel", dat, dat_dir)
}

/// tunnel_writer.cc:25 は get_waytype(obj.get("waytype")) を無条件に呼ぶ
/// （bridge/way/vehicleと同じく分岐なしで常に評価される）。get_waytype.cc:14-49は
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

/// tunnel_writer.cc:41-47: `frontimage[n][1]`が非空なら`number_of_seasons=1`
/// （冬季画像あり）、空なら0（無季節扱いで`season=0`のみ走査）。
fn number_of_seasons(dat: &DatFile) -> u8 {
    let snow_probe = dat.get("frontimage[n][1]").unwrap_or("");
    if snow_probe.is_empty() { 0 } else { 1 }
}

/// tunnel_writer.cc:49-60: `frontimage[nl][0]`（無ければ短縮形`frontimage[nl]`）が
/// 非空なら4幅（broad portal）、空なら1幅（narrow portal、無印のみ）。
fn number_portals(dat: &DatFile) -> u8 {
    let with_season = dat.get("frontimage[nl][0]").unwrap_or("");
    let probe = if !with_season.is_empty() {
        with_season
    } else {
        dat.get("frontimage[nl]").unwrap_or("")
    };
    if probe.is_empty() { 1 } else { 4 }
}

/// tunnel_writer.cc:84-98: season=0..=number_of_seasons、pos∈{front,back}、
/// j<number_portals、i<4（n/s/e/w）の全組み合わせについて
/// `{front|back}image[{indices[i]}{add[j]}][{season}]`を読み、season==0かつ空なら
/// 短縮形`{front|back}image[{indices[i]}{add[j]}]`にフォールバックする。
/// 実際に画像を指す値（空文字列や"-"以外）についてのみ、common::check_image_refで
/// ファイル存在・サイズを検証する（building/way/bridgeと同じ仕組み）。
struct ImageRefRule;
impl Rule for ImageRefRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        let seasons = number_of_seasons(dat);
        let portals = number_portals(dat);

        for season in 0..=seasons {
            for pos in ["front", "back"] {
                for j in 0..portals {
                    for dir in DIRECTIONS {
                        let width = WIDTHS[j as usize];
                        let key_with_season = format!("{pos}image[{dir}{width}][{season}]");
                        let mut value = dat.get(&key_with_season).unwrap_or("");
                        let key_used = if value.is_empty() && season == 0 {
                            let short_key = format!("{pos}image[{dir}{width}]");
                            value = dat.get(&short_key).unwrap_or("");
                            short_key
                        } else {
                            key_with_season
                        };

                        if value.is_empty() {
                            continue;
                        }
                        // "-"（画像なしセンチネル）の判定は`check_image_ref`
                        // （src/rules/common.rs）側に一元化されている。以前はここに
                        // `value == "-"`ガードを個別追加していたが、第8弾で共通化した
                        // ため不要（`check_image_ref`冒頭のdocコメント参照）。
                        check_image_ref(value, ctx.dat_dir, &key_used, &mut diags, ctx.language);
                    }
                }
            }
        }

        diags
    }
}

/// `tunnel_writer.cc:29-33`: intro_date/retire_dateがそれぞれ`year*12+month-1`で
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
