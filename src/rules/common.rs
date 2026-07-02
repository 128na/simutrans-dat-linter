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
/// waytypeを表すフィールドの必須性・既知値チェック。`get_waytype()`を無条件に
/// 呼ぶobj種別（way/bridge/tunnel/roadsign/vehicle/way-object/crossing）で
/// 共通のパターンを1箇所に集約する。`key`は`"waytype"`のような単純フィールド名
/// でも、`"own_waytype"`や`"waytype[0]"`のような別名・添字付きキーでもよい。
/// 呼び出し元ごとの`obj=`名はメッセージに含めず、`key`と値のみで表現する
/// （元の各obj種別別メッセージが持っていた「obj=way では」等の接頭辞はここでは
/// 付けない。これは表示テキストの差異であり、`missing-waytype`/`unknown-waytype`/
/// `waytype-ok`の3つの診断コードと重大度（Error/Error/Info）は元の実装と同じ）。
///
/// 根拠: `get_waytype()`（get_waytype.cc）はSTRICMPで`KNOWN_WAYTYPES`の
/// いずれにも一致しなければ`dbg->fatal("get_waytype()","invalid waytype
/// \"%s\"\n", ...)`で落とす。`tabfileobj_t::get()`はNULLを返さず欠落キーには
/// 空文字列を返す（tabfile.cc:48-56）ため、キー未指定も同じfatalパスに入る。
pub fn check_waytype_field(dat: &DatFile, key: &str) -> Vec<Diagnostic> {
    let waytype = dat.get(key).unwrap_or("").to_ascii_lowercase();
    if waytype.is_empty() {
        vec![Diagnostic::error(
            "missing-waytype",
            format!("{key} が必須です（get_waytype()は空文字列もFATAL ERRORにします）"),
        )]
    } else if !KNOWN_WAYTYPES.contains(&waytype.as_str()) {
        vec![Diagnostic::error(
            "unknown-waytype",
            format!("{key}={waytype} は不正な値です（FATAL ERRORになります）"),
        )]
    } else {
        vec![Diagnostic::info("waytype-ok", format!("{key}={waytype}"))]
    }
}

/// `image_writer_t::write_obj`（image_writer.cc:348-364）の構文仕様コメント
/// （`"[> ]imagefilename_without_extension[...]"`）どおり、先頭の`'>'`1文字は
/// 「ズーム不可」フラグとして`an_imagekey[0]=='>'`判定で剥がされ
/// （`an_imagekey = an_imagekey.substr(1)`、image_writer.cc:357-359）、
/// **`'>'`の有無に関わらず無条件で**続けて`trim(an_imagekey)`
/// （image_writer.cc:364、`utils/simstring.cc`の`trim()`本体は半角スペース/タブの
/// 前後除去のみ）が呼ばれる。この処理はobj種別を問わず`image_writer_t::write_obj`を
/// 通る全ての画像キーに無条件で適用される共通の構文であり、特定のobj種別に
/// 限定されない（menuのマイルストーンで実例`Image[0]=> skins.0.4`
/// （aburch/simutrans-pak128.britain:gui/gui64/skins-64.dat）から`'>'`構文の
/// 見落としが発覚し、その調査の過程で`trim()`が`'>'`の有無に関係なく常に
/// 呼ばれる実装であることも判明した。剥がさず・trimせずに`check_image_ref`へ
/// 渡すと、`"> skins.0.4"`や`" station_icon.png.0.0"`のような文字列をそのまま
/// ファイル名として解決を試み、実在するファイルを「見つからない」と誤検知する。
/// Rustの`str::trim()`はUnicode空白全般を対象とし対象がC++の` `/`\t`限定より
/// 広いが、実用上の`.dat`ファイルでこの差が問題になることはない。
fn strip_zoomable_prefix_and_trim(value: &str) -> &str {
    value.strip_prefix('>').unwrap_or(value).trim()
}

pub fn check_image_ref(value: &str, dat_dir: &Path, context: &str, diags: &mut Vec<Diagnostic>) {
    let value = strip_zoomable_prefix_and_trim(value);
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
