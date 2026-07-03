//! `obj=building` の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。

use super::common::{KNOWN_WAYTYPES, check_image_ref};
use crate::diagnostics::Diagnostic;
use crate::i18n::{Language, t};
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

const KNOWN_TYPES: &[&str] = &[
    "res",
    "com",
    "ind",
    "cur",
    "mon",
    "tow",
    "hq",
    "habour",
    "harbour",
    "dock",
    "fac",
    "stop",
    "extension",
    "depot",
    "any",
    "",
];
const OBSOLETE_TYPES: &[&str] = &[
    "station",
    "railstop",
    "monorailstop",
    "busstop",
    "carstop",
    "airport",
    "wharf",
    "hall",
    "post",
    "shed",
];
const TYPES_REQUIRING_WAYTYPE: &[&str] = &["stop", "depot"];

/// この obj 種別に対する検査項目一式。`DimsRule`が返す(size_x, size_y, layouts)を
/// `TileImageRule`のコンストラクタへ渡す必要があるため、ここで一度だけ`resolve_dims`を
/// 呼んで解決してから各ルールを構築する（interior mutabilityを使わないための設計）。
pub fn all(dat: &DatFile) -> Vec<Box<dyn Rule>> {
    let (size_x, size_y, layouts) = resolve_dims(dat);
    vec![
        Box::new(PreludeDebugRule),
        Box::new(TypeWaytypeRule),
        Box::new(ObsoleteKeywordRule),
        Box::new(DimsRule),
        Box::new(CursorIconRule),
        Box::new(TileImageRule {
            size_x,
            size_y,
            layouts,
        }),
    ]
}

/// 後方互換の薄いラッパー。`tests/building.rs`はこの関数を直接呼ぶ。
pub fn check_building(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext {
        dat,
        dat_dir,
        language: crate::i18n::Language::default(),
    };
    all(dat).iter().flat_map(|r| r.check(&ctx)).collect()
}

struct PreludeDebugRule;
impl Rule for PreludeDebugRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let type_name = ctx.dat.get("type").unwrap_or("").to_ascii_lowercase();
        let waytype = ctx.dat.get("waytype").unwrap_or("").to_ascii_lowercase();
        vec![
            Diagnostic::debug(
                "parsed-pairs",
                t!(ctx.language,
                    ja: "{n} 個のkey=valueを読み込み",
                    en: "Loaded {n} key=value pair(s)",
                    n = ctx.dat.pairs.len(),
                ),
            ),
            Diagnostic::debug(
                "raw-type-waytype",
                format!("type=\"{type_name}\" waytype=\"{waytype}\""),
            ),
        ]
    }
}

struct TypeWaytypeRule;
impl Rule for TypeWaytypeRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let type_name = ctx.dat.get("type").unwrap_or("").to_ascii_lowercase();
        let waytype = ctx.dat.get("waytype").unwrap_or("").to_ascii_lowercase();
        let mut diags = Vec::new();
        check_type_and_waytype(&type_name, &waytype, &mut diags, ctx.language);
        diags
    }
}

fn check_type_and_waytype(
    type_name: &str,
    waytype: &str,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    if OBSOLETE_TYPES.contains(&type_name) {
        diags.push(Diagnostic::error(
            "obsolete-type",
            t!(lang,
                ja: "type={type_name} は obsolete です。stop/extension と waytype を使ってください",
                en: "type={type_name} is obsolete. Use stop/extension with waytype instead",
                type_name = type_name,
            ),
        ));
        return;
    }
    if !KNOWN_TYPES.contains(&type_name) {
        diags.push(Diagnostic::error(
            "unknown-type",
            t!(lang,
                ja: "type={type_name} は makeobj が認識できない値です（FATAL ERRORになります）",
                en: "type={type_name} is not a value makeobj recognizes (this becomes a FATAL ERROR)",
                type_name = type_name,
            ),
        ));
        return;
    }

    if TYPES_REQUIRING_WAYTYPE.contains(&type_name) {
        if waytype.is_empty() {
            diags.push(Diagnostic::error(
                "missing-waytype",
                t!(lang,
                    ja: "type={type_name} では waytype が必須です（未指定だとmakeobjがFATAL ERRORになります）",
                    en: "waytype is required when type={type_name} (omitting it makes makeobj FATAL ERROR)",
                    type_name = type_name,
                ),
            ));
        } else if !KNOWN_WAYTYPES.contains(&waytype) {
            diags.push(Diagnostic::error(
                "unknown-waytype",
                t!(lang,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            ));
        } else {
            diags.push(Diagnostic::info(
                "type-waytype-ok",
                format!("type={type_name} waytype={waytype}"),
            ));
        }
    } else if type_name == "extension" {
        if waytype.is_empty() {
            diags.push(Diagnostic::warning(
                "generic-extension",
                t!(lang,
                    ja: "type=extension で waytype 未指定は「全waytypeに適合する汎用拡張」として解釈されます。意図的でなければ waytype を指定してください",
                    en: "type=extension without waytype is interpreted as a \"generic extension \
                         that fits any waytype\". Specify waytype unless this is intentional",
                ),
            ));
        } else if !KNOWN_WAYTYPES.contains(&waytype) {
            diags.push(Diagnostic::error(
                "unknown-waytype",
                t!(lang,
                    ja: "waytype={waytype} は不正な値です（FATAL ERRORになります）",
                    en: "waytype={waytype} is not a valid value (this becomes a FATAL ERROR)",
                    waytype = waytype,
                ),
            ));
        } else {
            diags.push(Diagnostic::info(
                "type-waytype-ok",
                format!("type={type_name} waytype={waytype}"),
            ));
        }
    } else {
        diags.push(Diagnostic::info(
            "type-waytype-ok",
            format!("type={type_name}"),
        ));
    }
}

struct ObsoleteKeywordRule;
impl Rule for ObsoleteKeywordRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        if ctx.dat.get("extension_building").is_some() {
            vec![Diagnostic::error(
                "obsolete-keyword",
                t!(ctx.language,
                    ja: "extension_building は obsolete です。type=stop/extension と waytype を使ってください",
                    en: "extension_building is obsolete. Use type=stop/extension with waytype instead",
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

/// `Dims=`を`(size_x, size_y, layouts)`へ解決する。診断は伴わない純粋な計算。
/// `DimsRule`（診断を出す）と`TileImageRule`（値だけ必要）の両方から呼ばれる。
fn resolve_dims(dat: &DatFile) -> (i64, i64, i64) {
    let ints = dat.get_ints("dims");
    let size_x = ints.first().copied().unwrap_or(1);
    let size_y = ints.get(1).copied().unwrap_or(1);
    let mut layouts = ints.get(2).copied().unwrap_or(0);
    if layouts == 0 {
        layouts = if size_x == size_y { 1 } else { 2 };
    }
    (size_x, size_y, layouts)
}

struct DimsRule;
impl Rule for DimsRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let ints = ctx.dat.get_ints("dims");
        let (size_x, size_y, layouts) = resolve_dims(ctx.dat);
        let mut diags = vec![Diagnostic::debug(
            "dims-resolved",
            format!("Dims={ints:?} -> size_x={size_x} size_y={size_y} layouts={layouts}"),
        )];

        if size_x * size_y == 0 {
            diags.push(Diagnostic::error(
                "zero-size",
                t!(ctx.language,
                    ja: "Dims のサイズが0です (size_x={size_x}, size_y={size_y})",
                    en: "Dims size is 0 (size_x={size_x}, size_y={size_y})",
                    size_x = size_x,
                    size_y = size_y,
                ),
            ));
        } else {
            diags.push(Diagnostic::info(
                "dims-ok",
                format!("size={size_x}x{size_y} layouts={layouts}"),
            ));
        }
        diags
    }
}

struct CursorIconRule;
impl Rule for CursorIconRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let cursor = ctx.dat.get("cursor").unwrap_or("");
        let icon = ctx.dat.get("icon").unwrap_or("");
        let mut diags = vec![Diagnostic::debug(
            "raw-cursor-icon",
            format!("cursor=\"{cursor}\" icon=\"{icon}\""),
        )];

        if cursor.is_empty() && icon.is_empty() {
            diags.push(Diagnostic::error(
                "missing-cursor-icon",
                t!(ctx.language,
                    ja: "cursor と icon が両方とも未指定です。makeobjはエラーを出さずにビルドしますが、ゲーム内のビルドメニューに表示されません",
                    en: "Both cursor and icon are unspecified. makeobj builds without error, but the \
                         object will not appear in the in-game build menu",
                ),
            ));
            return diags;
        }

        if !icon.is_empty() {
            check_image_ref(icon, ctx.dat_dir, "icon", &mut diags, ctx.language);
        }
        if !cursor.is_empty() {
            check_image_ref(cursor, ctx.dat_dir, "cursor", &mut diags, ctx.language);
        }
        diags
    }
}

struct TileImageRule {
    size_x: i64,
    size_y: i64,
    layouts: i64,
}
impl Rule for TileImageRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let dat = ctx.dat;
        let dat_dir = ctx.dat_dir;

        for l in 0..self.layouts {
            // building_writer.cc: 奇数レイアウトは縦横を入れ替えて走査する
            let (w, h) = if l % 2 == 1 {
                (self.size_y, self.size_x)
            } else {
                (self.size_x, self.size_y)
            };
            for y in 0..h {
                for x in 0..w {
                    let front5 = format!("frontimage[{l}][{y}][{x}][0][0]");
                    let back5 = format!("backimage[{l}][{y}][{x}][0][0]");
                    let front6 = format!("frontimage[{l}][{y}][{x}][0][0][0]");
                    let back6 = format!("backimage[{l}][{y}][{x}][0][0][0]");

                    diags.push(Diagnostic::debug(
                        "tile-key-lookup",
                        format!("layout {l} tile ({x},{y}): {front5} / {back5} ({front6} / {back6} もfallback確認)"),
                    ));

                    let front = dat.get(&front5).or_else(|| dat.get(&front6));
                    let back = dat.get(&back5).or_else(|| dat.get(&back6));

                    if front.is_none() && back.is_none() {
                        diags.push(Diagnostic::error(
                            "missing-tile-image",
                            t!(ctx.language,
                                ja: "layout {l} tile ({x},{y}) に front/backimage が1枚もありません\
                                     （makeobjはエラーを出さず空画像のタイルを生成します）",
                                en: "layout {l} tile ({x},{y}) has no front/backimage \
                                     (makeobj generates an empty tile without error)",
                                l = l,
                                x = x,
                                y = y,
                            ),
                        ));
                    } else {
                        diags.push(Diagnostic::info(
                            "tile-image-ok",
                            format!("layout {l} tile ({x},{y})"),
                        ));
                        if let Some(v) = front {
                            check_image_ref(
                                v,
                                dat_dir,
                                &format!("frontimage[{l}][{y}][{x}]"),
                                &mut diags,
                                ctx.language,
                            );
                        }
                        if let Some(v) = back {
                            check_image_ref(
                                v,
                                dat_dir,
                                &format!("backimage[{l}][{y}][{x}]"),
                                &mut diags,
                                ctx.language,
                            );
                        }
                    }
                }
            }
        }

        // frontimage の高さ(h)は0のみ許可。h>0が定義されていないか確認する
        for key in dat.pairs.keys() {
            if let Some(rest) = key.strip_prefix("frontimage[") {
                let indices: Vec<&str> = rest.trim_end_matches(']').split("][").collect();
                // [l][y][x][h][phase] (+season)
                if let Some(h_str) = indices.get(3)
                    && h_str.parse::<i64>().unwrap_or(0) > 0
                {
                    diags.push(Diagnostic::error(
                        "frontimage-height",
                        t!(ctx.language,
                            ja: "{key} : frontimageの高さ(h)は0のみ有効です\
                                 （makeobjはエラーログを出すだけで処理を継続します）",
                            en: "{key}: frontimage height (h) must be 0 \
                                 (makeobj logs an error but continues processing)",
                            key = key,
                        ),
                    ));
                }
            }
        }

        diags
    }
}
