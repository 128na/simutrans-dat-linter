//! `obj=crossing`（2つのwayが交差する踏切/交差点）の検証ルール。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/crossing_writer.cc` / `crossing_writer.h` /
//! `get_waytype.cc` / `imagelist_writer.cc` / `xref_writer.cc` / `obj_writer.cc` /
//! `dataobj/tabfile.cc`）を直接読んで確認した。OTRP側の個別diffはまだ行っていない
//! （vehicle/way/good/bridge/tunnel/roadsignと同様）。
//!
//! `crossing_writer_t::write_obj`（crossing_writer.cc:47-159）は他のobj種別と異なり、
//! **waytypeが2つ**（`waytype[0]`/`waytype[1]`）ある。この2つは交差する2本のwayを表す
//! （crossing_writer.cc:77「waytypes, waytype 2 will be on top」）:
//!
//! - `waytype[0]`/`waytype[1]`はそれぞれ`get_waytype(obj.get("waytype[N]"))`
//!   （crossing_writer.cc:78-79）を無条件に呼ぶ。vehicle/way/bridge/tunnel/roadsignと
//!   全く同じ`get_waytype()`関数を経由するため、欠落・不正値は
//!   `dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`になる
//!   （get_waytype.cc:14-49、tabfileobj_t::get()はキー欠落時に空文字列を返すのみで
//!   NULLは返さない）。
//! - 2つが**解決後の値として同一**の場合（例: `waytype[0]=road`と`waytype[1]=road`、
//!   あるいは`waytype[0]=schiene_tram`と`waytype[1]=tram_track`のように別名でも
//!   同じ`waytype_t`列挙値に解決されるケースも含む。get_waytype.cc:36-39で
//!   `schiene_tram`と`tram_track`はどちらも`tram_wt`に解決される既知の別名ペアである）は
//!   `dbg->fatal("Crossing", "Identical ways (%s) cannot cross (check waytypes)!", ...)`
//!   になる（crossing_writer.cc:80-82）。文字列としての一致ではなく解決後の列挙値の
//!   一致で判定される点に注意（get_waytype()実装を素朴に再現するため、本ツールも
//!   文字列->列挙値マッピングを再現してから比較する）。
//! - `speed[0]`/`speed[1]`（いずれも`get_int(..., 0)`、crossing_writer.cc:87-88）は
//!   **どちらか一方でも0**（未指定を含む。get_intのデフォルト値も0）なら
//!   `dbg->fatal("Crossing", "A maxspeed MUST be given for both ways!")`になる
//!   （crossing_writer.cc:90-92）。bridgeやtunnelの数値フィールドと異なり、
//!   `get_int_clamped`ではなく素の`get_int`だが、0のときに明示的なfatal分岐が
//!   存在する点で他のobj種別のフィールドとは扱いが異なる。
//! - `sound`・`animation_time_open`/`animation_time_closed`・`intro_year`/
//!   `intro_month`/`retire_year`/`retire_month`（crossing_writer.cc:52-69,97-117）は
//!   全て無条件フォールバック（`atoi`または`get_int`）のみで、`get_int_clamped`は
//!   一切呼ばれていない（crossing_writer.cc全文で確認）。bridgeの`ClampedRangeRule`に
//!   相当するルールはcrossingには存在しない（下記REJECTED参照）。
//!
//! 画像は`make_list`ヘルパー（crossing_writer.cc:24-36）が`{key}[{i}]`形式で
//! `i=0,1,2,...`と連番走査し、最初に空のキーに当たった時点で止める（roadsignの
//! numbered構文と同じ「連番総なめ」方式だが、4の倍数チェックのような制約は無い）。
//! 8種類のリスト（`openimage[ns]`/`openimage[ew]`/`front_openimage[ns]`/
//! `front_openimage[ew]`/`closedimage[ns]`/`closedimage[ew]`/
//! `front_closedimage[ns]`/`front_closedimage[ew]`）全てがこの走査方式を使う
//! （crossing_writer.cc:130,140,147,153-154）。
//!
//! - `openimage[ns]`と`openimage[ew]`は**両方とも最低1枚（インデックス0）が必須**。
//!   どちらかが空リスト（`openimage[ns][0]`または`openimage[ew][0]`が未指定）だと
//!   `dbg->fatal("Crossing", "Missing images (at least one openimage! (but %i and %i
//!   found)!)", ...)`になる（crossing_writer.cc:132-135、コメント「// these must
//!   exists!」）。
//! - `front_openimage[ns/ew]`・`closedimage[ns/ew]`・`front_closedimage[ns/ew]`は
//!   空リストでも`write_list`が`xref_writer_t::write_obj(..., "", false)`
//!   （空プレースホルダ、fatal引数=false）を書くだけで、fatal/warningは出ない
//!   （crossing_writer.cc:38-45,142-144,149-150,155-156。コメント「// the following
//!   lists are optional」「// closed crossings ...」に対応する必須チェックが無い）。
//! - 個々の画像キーが実際に画像を指す場合（空文字列でない場合）は、
//!   `image_writer_t::write_obj`（image_writer.cc:348-439）がファイルの存在・
//!   サイズ（128の倍数か）を検証する。これはbuilding/way/bridge/tunnel/roadsignと
//!   共有の`common::check_image_ref`でカバーする。
//! - `imagelist_writer_t::write_obj`（imagelist_writer.cc:14-35）の
//!   `count < keys.get_count()`警告は、`make_list`が非空値だけを`list.append()`する
//!   （crossing_writer.cc:24-36の`if (str.empty()) break;`より前にappendは無い）ため
//!   `count`は常に`keys.get_count()`と一致し、この警告分岐は実際には発火しない
//!   （tunnelと同じ理由）。
//!
//! crossingには`cursor`/`icon`フィールドへの言及がcrossing_writer.cc全文に一つも
//! 無い（building/way/bridge/tunnel/roadsignと異なり、`cursorskin_writer_t`を
//! 一切呼ばない）。よってcursor/icon関連のルールはそもそも対象外。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `sound`（crossing_writer.cc:52-69）の妥当性検証: 数値なら`atoi`でsound_idとして
//!   使われ、非数値（先頭が'0'でない非数値文字列）ならLOAD_SOUND方式のファイル名として
//!   扱われるだけで、いずれの経路もfatal/warningを出さない。
//! - `animation_time_open`/`animation_time_closed`（いずれも`get_int(..., 0)`、
//!   crossing_writer.cc:97,99）の妥当性検証: 無条件フォールバックのみで
//!   `get_int_clamped`ではない。0や負値でもそのまま`uint32`として書き込まれるだけ
//!   （write_uint32は符号なし変換のみ）。bridgeのtopspeed等が見送られたのと同じ理由。
//! - `intro_year`/`intro_month`/`retire_year`/`retire_month`（いずれも`get_int`、
//!   crossing_writer.cc:110-114）の妥当性検証: 無条件フォールバックのみで
//!   `get_int_clamped`ではない（bridgeとは異なりcrossingはこれらにclampを掛けない）。
//!   tunnel/roadsignの同種フィールドが見送られたのと同じ理由。
//! - `front_openimage[ns/ew]`・`closedimage[ns/ew]`・`front_closedimage[ns/ew]`の
//!   未指定警告: 上記の通り、これら3種はcrossing_writer.cc上で明示的に
//!   「optional」とコメントされ、対応するfatal/warning分岐が存在しない
//!   （openimageの`// these must exists!`と対比される設計）。
//! - `imagelist_writer_t::write_obj`のcount不一致警告
//!   （"Expected %i but found %i images"）: `make_list`は非空値のみをappendし、
//!   空値に当たった時点でループを止めるため、appendされた件数（`keys.get_count()`）と
//!   実際に書き込まれた件数（`count`、`image_writer_t::write_obj`の呼び出し回数）は
//!   常に一致する。tunnelの同種警告が見送られたのと同じ理由。
//! - `cursor`/`icon`未指定検証: crossing_writer.cc全文に`cursor`/`icon`への言及が
//!   一つも無く、`cursorskin_writer_t`も呼ばれない。他obj種別と対称的な
//!   「未指定なら見送り」の判断ですらなく、そもそも対象フィールドが存在しない。
//! - `waytype[N]`が既知だが`crossing`として組み合わせ的に不自然な値
//!   （例: `waytype[0]=power`と`waytype[1]=decoration`）の妥当性検証:
//!   crossing_writer.cc:78-84は解決後の列挙値が一致するかどうかしか見ておらず、
//!   どの2つの異なるwaytypeの組み合わせが「意味のある交差」かを判定するロジックは
//!   makeobj側に存在しない（ゲーム側の`crossing_logic_t`が実行時に使い方を決める）。

use super::common::{KNOWN_WAYTYPES, check_image_ref};
use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::t;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// get_waytype.cc:14-49 の文字列->列挙値マッピングが実際に区別する`waytype_t`の
/// 値そのものを表す（代表文字列ではなく型として区別する）。`KNOWN_WAYTYPES`は
/// 13種類の入力文字列を持つが、`schiene_tram`と`tram_track`はどちらも
/// `Waytype::Tram`（get_waytype.cc:36-39の`tram_wt`）に解決される既知の別名ペア
/// のため、distinctなvariant数は12。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Waytype {
    Ignore,
    Road,
    Track,
    Overheadlines,
    Maglev,
    Monorail,
    Narrowgauge,
    Water,
    Air,
    Tram,
    Powerline,
    Decoration,
}

/// get_waytype.cc:14-49 の文字列->列挙値マッピングをそのまま再現する。
/// `schiene_tram`と`tram_track`はどちらも`tram_wt`に解決される既知の別名ペア
/// （get_waytype.cc:36-39）。それ以外は概ね1文字列1列挙値。
///
/// 不正値はget_waytype()内でdbg->fatalになるため、呼び出し元
/// （`IdenticalWaytypesRule`）は既に`KNOWN_WAYTYPES.contains()`を通った値しか
/// 渡さない想定だが、その前提を「代表値へのフォールバック」というコメントで
/// 保証する代わりに`None`という型で表現する（型設計監査で指摘された箇所）。
fn resolve_waytype(raw: &str) -> Option<Waytype> {
    match raw.to_ascii_lowercase().as_str() {
        "none" => Some(Waytype::Ignore),
        "road" => Some(Waytype::Road),
        "track" => Some(Waytype::Track),
        "electrified_track" => Some(Waytype::Overheadlines),
        "maglev_track" => Some(Waytype::Maglev),
        "monorail_track" => Some(Waytype::Monorail),
        "narrowgauge_track" => Some(Waytype::Narrowgauge),
        "water" => Some(Waytype::Water),
        "air" => Some(Waytype::Air),
        // get_waytype.cc:36-39: 両方とも tram_wt に解決される既知の別名ペア。
        "schiene_tram" | "tram_track" => Some(Waytype::Tram),
        "power" => Some(Waytype::Powerline),
        "decoration" => Some(Waytype::Decoration),
        _ => None,
    }
}

pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(WaytypesRequiredRule),
        Box::new(IdenticalWaytypesRule),
        Box::new(SpeedRequiredRule),
        Box::new(OpenImageRequiredRule),
        Box::new(ImageRefRule),
    ]
}

/// `tests/crossing_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_crossing(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("crossing", dat, dat_dir)
}

/// crossing_writer.cc:78-79 は waytype[0]/waytype[1] それぞれについて
/// get_waytype(obj.get("waytype[N]")) を無条件に呼ぶ（他のobj種別と同じ
/// get_waytype()関数を経由）。get_waytype.cc:14-49はSTRICMPが既知13種の
/// いずれにも一致しなければ dbg->fatal("get_waytype()","invalid waytype
/// \"%s\"\n", waytype) で落とす。tabfileobj_t::get()はNULLを返さず欠落キーには
/// 空文字列を返す（tabfile.cc:48-56）ため、waytype[N]未指定も同じfatalパスに入る。
/// 実際のチェックロジックは`common::check_waytype_field`に集約されている
/// （way/bridge/tunnel/roadsign/vehicle/way-object/crossingで共有。crossingは
/// waytype[0]/waytype[1]の2キー分をそれぞれ呼び出して結果を連結する点のみ
/// 他のobj種別と異なる）。
struct WaytypesRequiredRule;
impl Rule for WaytypesRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        (0..2)
            .flat_map(|i| {
                super::common::check_waytype_field(ctx.dat, &format!("waytype[{i}]"), ctx.language)
            })
            .collect()
    }
}

/// crossing_writer.cc:80-82: waytype[0]とwaytype[1]が解決後の値として同一だと
/// dbg->fatal("Crossing", "Identical ways (%s) cannot cross (check waytypes)!", ...)。
/// 両方とも既知値である場合のみ判定する（片方でも不正値ならWaytypesRequiredRuleが
/// 既にFATALを出しており、get_waytype()自身がその時点で終了する＝この比較には
/// 到達しないため）。
struct IdenticalWaytypesRule;
impl Rule for IdenticalWaytypesRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let w0 = ctx.dat.get("waytype[0]").unwrap_or("").to_ascii_lowercase();
        let w1 = ctx.dat.get("waytype[1]").unwrap_or("").to_ascii_lowercase();
        if w0.is_empty()
            || w1.is_empty()
            || !KNOWN_WAYTYPES.contains(&w0.as_str())
            || !KNOWN_WAYTYPES.contains(&w1.as_str())
        {
            return Vec::new();
        }
        if let (Some(r0), Some(r1)) = (resolve_waytype(&w0), resolve_waytype(&w1))
            && r0 == r1
        {
            return vec![Diagnostic::error(
                DiagnosticCode::CrossingIdenticalWaytypes,
                t!(ctx.language,
                    ja: "waytype[0]={w0} と waytype[1]={w1} は同じ種別のwayに解決されます。\
                         makeobjは同一waytype同士の交差をFATAL ERRORにします\
                         （\"Identical ways ({w0}) cannot cross (check waytypes)!\"）",
                    en: "waytype[0]={w0} and waytype[1]={w1} resolve to the same way type. \
                         makeobj treats crossing identical waytypes as a FATAL ERROR \
                         (\"Identical ways ({w0}) cannot cross (check waytypes)!\")",
                    w0 = w0,
                    w1 = w1,
                ),
            )];
        }
        Vec::new()
    }
}

/// crossing_writer.cc:87-92: speed[0]/speed[1]（いずれもget_int(key, 0)）の
/// どちらか一方でも0（未指定含む）だと dbg->fatal("Crossing", "A maxspeed MUST be
/// given for both ways!")。
struct SpeedRequiredRule;
impl Rule for SpeedRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let speed0 = ctx
            .dat
            .get("speed[0]")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0);
        let speed1 = ctx
            .dat
            .get("speed[1]")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0);
        if speed0 == 0 || speed1 == 0 {
            vec![Diagnostic::error(
                DiagnosticCode::CrossingMissingSpeed,
                t!(ctx.language,
                    ja: "speed[0] と speed[1] の両方に0以外の値が必要です。makeobjは\
                         どちらか一方でも0（未指定含む）だとFATAL ERRORにします\
                         （\"A maxspeed MUST be given for both ways!\"）",
                    en: "Both speed[0] and speed[1] must be non-zero. makeobj treats either \
                         being 0 (including unspecified) as a FATAL ERROR \
                         (\"A maxspeed MUST be given for both ways!\")",
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

/// crossing_writer.cc:24-36 `make_list`をそのまま再現する。`{key}[{i}]`形式で
/// i=0,1,2,...と連番走査し、最初に空のキーに当たった時点で止める。
fn make_list<'a>(dat: &'a DatFile, key: &str) -> Vec<(&'a str, String)> {
    let mut list = Vec::new();
    for i in 0.. {
        let full_key = format!("{key}[{i}]");
        let value = dat.get(&full_key).unwrap_or("");
        if value.is_empty() {
            break;
        }
        list.push((value, full_key));
    }
    list
}

/// crossing_writer.cc:129-135: openimage[ns]とopenimage[ew]は「// these must
/// exists!」の通り両方とも最低1枚が必須。どちらかが空リストだと
/// dbg->fatal("Crossing", "Missing images (at least one openimage! (but %i and %i
/// found)!)", ...)。
struct OpenImageRequiredRule;
impl Rule for OpenImageRequiredRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let ns_count = make_list(ctx.dat, "openimage[ns]").len();
        let ew_count = make_list(ctx.dat, "openimage[ew]").len();
        if ns_count == 0 || ew_count == 0 {
            vec![Diagnostic::error(
                DiagnosticCode::CrossingMissingOpenimage,
                t!(ctx.language,
                    ja: "openimage[ns][0] と openimage[ew][0] は両方とも最低1枚必要です\
                         （現在 openimage[ns]={ns_count}枚 / openimage[ew]={ew_count}枚）。\
                         makeobjは片方でも0枚だとFATAL ERRORにします\
                         （\"Missing images (at least one openimage! (but {ns_count} and \
                         {ew_count} found)!)\"）",
                    en: "Both openimage[ns][0] and openimage[ew][0] require at least one image \
                         (currently openimage[ns]={ns_count} / openimage[ew]={ew_count}). \
                         makeobj treats either being 0 as a FATAL ERROR \
                         (\"Missing images (at least one openimage! (but {ns_count} and \
                         {ew_count} found)!)\")",
                    ns_count = ns_count,
                    ew_count = ew_count,
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

/// crossing_writer.cc:120-156: 8種類の画像リスト
/// （openimage[ns/ew], front_openimage[ns/ew], closedimage[ns/ew],
/// front_closedimage[ns/ew]）全てについて、実際に画像を指す値（空文字列以外。
/// make_listは空文字列に当たった時点でループを止めるため、リストに入っている値は
/// 常に非空）の参照ファイル存在・サイズをcommon::check_image_refで検証する
/// （building/way/bridge/tunnel/roadsignと共有）。
struct ImageRefRule;
impl Rule for ImageRefRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        const LIST_KEYS: &[&str] = &[
            "openimage[ns]",
            "openimage[ew]",
            "front_openimage[ns]",
            "front_openimage[ew]",
            "closedimage[ns]",
            "closedimage[ew]",
            "front_closedimage[ns]",
            "front_closedimage[ew]",
        ];
        for key in LIST_KEYS {
            for (value, full_key) in make_list(ctx.dat, key) {
                check_image_ref(value, ctx.dat_dir, &full_key, &mut diags, ctx.language);
            }
        }
        diags
    }
}
