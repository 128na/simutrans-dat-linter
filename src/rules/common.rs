//! building/vehicle 両方のルールから参照される共通の定数・ヘルパー。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。

use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use std::path::Path;

/// obj種別を問わず、パーサレベルで検出した重複キーを警告として出す。
/// `tabfileobj_t::put()`の実装（`if(objinfo.get(key).str) return false;`）と
/// `tabfile.h`のdocコメント「If keys are duplicated for one object, the first
/// value is used」により、実際のmakeobjは重複キーを**先勝ち**で無音に無視する。
/// makeobj自身はfatal/errorにしないためWarning止まりだが、意図しない値の
/// 上書き忘れである可能性が高いため検出する。
pub fn check_duplicate_keys(dat: &DatFile) -> Vec<Diagnostic> {
    dat.duplicates
        .iter()
        .map(|d| {
            Diagnostic::warning(
                "duplicate-key",
                format!(
                    "キー \"{}\" が複数回定義されています（{}行目の値が採用され、\
                     {}行目は無視されます）。makeobjのtabfileobj_t::put()は既存キーを\
                     上書きしません（先勝ち、tabfile.h:45）",
                    d.key, d.first_line, d.duplicate_line
                ),
            )
            .at(d.duplicate_line, d.key.clone())
        })
        .collect()
}

/// `get_waytype()`（get_waytype.cc）がSTRICMPで受理する既知waytype一覧。
/// これ以外の値、および欠落は `dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`。
/// building・vehicle 両方の`get_waytype()`呼び出しがこの同じ関数を経由するため、
/// このリストは両obj種別で共有する（重複定義しない）。
pub const KNOWN_WAYTYPES: &[&str] = &[
    "none",
    "road",
    "track",
    "electrified_track",
    "maglev_track",
    "monorail_track",
    "narrowgauge_track",
    "water",
    "air",
    "schiene_tram",
    "tram_track",
    "power",
    "decoration",
];

/// pak128の画像タイルサイズ。このプロジェクトは現状pak128のみを対象とする
/// （image_writer.cc: `if ((width%img_size!=0)||(height%img_size!=0)) dbg->fatal(...)`）。
pub const PAK_TILE_SIZE: u32 = 128;

/// 画像参照（`icon=`, `frontimage[...]=`等）を検証する。ファイル存在確認と
/// サイズが`PAK_TILE_SIZE`の倍数か（`image_writer.cc`の該当fatalチェック）を見る。
/// building・vehicleの画像フィールドはどちらも同じ`layer.season.frame`形式の
/// サフィックスを持つため、両者から共有する。
pub fn check_image_ref(value: &str, dat_dir: &Path, context: &str, diags: &mut Vec<Diagnostic>) {
    let base = value.split(',').next().unwrap_or(value);
    let parts: Vec<&str> = base.split('.').collect();
    let filename = if parts.len() >= 2
        && parts[parts.len() - 1].parse::<i64>().is_ok()
        && parts[parts.len() - 2].parse::<i64>().is_ok()
    {
        parts[..parts.len() - 2].join(".")
    } else {
        base.to_string()
    };

    let path = dat_dir.join(&filename);
    diags.push(Diagnostic::debug(
        "image-ref-resolved",
        format!(
            "{context}: \"{value}\" -> filename=\"{filename}\" path={}",
            path.display()
        ),
    ));

    if !path.is_file() {
        diags.push(Diagnostic::error(
            "missing-image-file",
            format!(
                "{context}: 参照画像 {filename} が見つかりません ({})",
                path.display()
            ),
        ));
        return;
    }

    match image::open(&path) {
        Ok(img) => {
            let (w, h) = (img.width(), img.height());
            if w % PAK_TILE_SIZE != 0 || h % PAK_TILE_SIZE != 0 {
                diags.push(Diagnostic::error(
                    "image-size-not-multiple-of-128",
                    format!(
                        "{context}: {filename} のサイズが {w}x{h} です。makeobj pak128 は{PAK_TILE_SIZE}の倍数でないとエラーになります"
                    ),
                ));
            } else {
                diags.push(Diagnostic::info(
                    "image-ok",
                    format!("{context}: {filename} {w}x{h}"),
                ));
            }
        }
        Err(e) => {
            diags.push(Diagnostic::error(
                "unreadable-image",
                format!("{context}: {filename} を画像として読み込めません ({e})"),
            ));
        }
    }
}
