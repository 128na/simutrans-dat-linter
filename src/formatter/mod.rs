use crate::i18n::{Language, t};
use crate::parser::format_key;

pub mod order;
use order::{OrderSpec, Section, order_for};

/// 1行をパースした結果。tabfile_t::read()/read_line() の挙動に合わせて分類する:
/// - `#`または行頭スペースで始まる行は、実際のmakeobjでも丸ごと無視される
///   （read_line(): `while(*dest=='#' || *dest==' ')`でスキップされ続ける）。
///   フォーマッタはこれを「無効化」も「有効化」もせず、原文のまま通す
/// - 行頭が`-`の行は、実際のmakeobjでは1つのobj定義の終端マーカーである
///   （`tabfile_t::read()`: `while(read_line(...) && *line != '-')`）。1ファイルに
///   複数のobj定義が連結されている場合の区切りであり、`=`が無くても
///   Malformed扱いにはしない（警告も出さない）
/// - `=`が無い非空行（区切り行を除く）は、実際のmakeobjでも
///   `dbg->warning("No data in ...")`になる
pub enum Entry {
    Pair { key: String, value: String },
    Comment(String),
    Blank,
    SkippedLeadingSpace(String),
    Separator(String),
    Malformed(String),
}

pub struct ParsedDat {
    pub entries: Vec<Entry>,
    pub warnings: Vec<String>,
}

pub fn parse_entries(text: &str, lang: Language) -> ParsedDat {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for (i, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        let lineno = i + 1;
        if line.is_empty() {
            entries.push(Entry::Blank);
        } else if line.starts_with('#') {
            entries.push(Entry::Comment(line.to_string()));
        } else if line.starts_with(' ') {
            warnings.push(t!(lang,
                ja: "line {lineno}: 行頭にスペースがあるため makeobj から無視されます（コメント扱い）: \"{line}\"",
                en: "line {lineno}: ignored by makeobj because it starts with whitespace (treated as a comment): \"{line}\"",
                lineno = lineno,
                line = line,
            ));
            entries.push(Entry::SkippedLeadingSpace(line.to_string()));
        } else if line.starts_with('-') {
            entries.push(Entry::Separator(line.to_string()));
        } else if let Some((key_raw, value)) = line.split_once('=') {
            entries.push(Entry::Pair {
                key: format_key(key_raw),
                value: value.to_string(),
            });
        } else {
            warnings.push(t!(lang,
                ja: "line {lineno}: '=' が無いため makeobj から無視されます: \"{line}\"",
                en: "line {lineno}: ignored by makeobj because it has no '=': \"{line}\"",
                lineno = lineno,
                line = line,
            ));
            entries.push(Entry::Malformed(line.to_string()));
        }
    }

    ParsedDat { entries, warnings }
}

/// パース済みentriesから`obj=`の値を取り出す。`format_reordered`が二重パースせずに
/// obj種別ごとの並び順テーブルを選べるようにするためのヘルパー。
pub fn obj_of(entries: &[Entry]) -> Option<&str> {
    entries.iter().find_map(|e| match e {
        Entry::Pair { key, value } if key == "obj" => Some(value.trim()),
        _ => None,
    })
}

/// 既存の行順を保ったまま、Pair行だけ `key=value`（=前後の空白なし、値は前後トリムのみ）
/// に正規化する。コメント・スキップ行・不正行は原文のまま通す（意味を変えない）。
pub fn format_preserve_order(entries: &[Entry]) -> String {
    let mut out = String::new();
    for entry in entries {
        match entry {
            Entry::Pair { key, value } => {
                out.push_str(key);
                out.push('=');
                out.push_str(value.trim());
                out.push('\n');
            }
            Entry::Comment(s)
            | Entry::SkippedLeadingSpace(s)
            | Entry::Separator(s)
            | Entry::Malformed(s) => {
                out.push_str(s);
                out.push('\n');
            }
            Entry::Blank => out.push('\n'),
        }
    }
    out
}

/// `--reorder`: `obj=`の値に応じた慣習的な並び順（`order`モジュール参照）に再構成する。
/// コメント・空行・スキップ行・不正行は並び替えとの整合が取れないため出力には
/// 含めない（dropした件数をwarningsで返す）。`obj`が未対応の値の場合は並び替えを
/// 行わず、元の行順を保ったまま（`format_preserve_order`相当）出力する。
///
/// 1ファイルに`Entry::Separator`（`-`始まりの区切り行）で複数のobj定義が
/// 連結されている場合は、区切り行ごとにセグメントへ分割し、**セグメントごとに
/// 独立して**並び替える（全セグメントに同じ`obj`の並び順仕様を適用する。
/// 建物の複数ステージ等、連結された定義は通常すべて同じobj種別のため）。
/// 区切り行自体は元の位置・原文のまま復元する。
pub fn format_reordered(entries: &[Entry], obj: &str, lang: Language) -> (String, Vec<String>) {
    let mut warnings = Vec::new();

    let Some(spec) = order_for(obj) else {
        warnings.push(t!(lang,
            ja: "--reorder: obj={obj} は並び替えに未対応です。元の順序のまま出力します",
            en: "--reorder: obj={obj} is not supported for reordering. Output uses the original order",
            obj = obj,
        ));
        return (format_preserve_order(entries), warnings);
    };

    let segments: Vec<&[Entry]> = entries
        .split(|e| matches!(e, Entry::Separator(_)))
        .collect();
    let separators: Vec<&str> = entries
        .iter()
        .filter_map(|e| match e {
            Entry::Separator(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();

    let mut out = String::new();
    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            out.push_str(separators[i - 1]);
            out.push('\n');
        }
        let (segment_out, segment_warnings) = format_reordered_segment(segment, spec, lang);
        out.push_str(&segment_out);
        warnings.extend(segment_warnings);
    }

    (out, warnings)
}

/// 「直前に紐づいたコメント（0件以上）+ Pair」を1単位として扱うグループ化・
/// 並び替え単位。コメントはこの`Pair`と一緒に移動し、出力時は`comments`を
/// `key=value`の直前にそのまま復元する。
struct PairGroup<'a> {
    comments: Vec<&'a str>,
    key: &'a String,
    value: &'a String,
}

/// `entries`を走査し、「`#`始まりコメント（連続可、間の空行はスキップして
/// 読み飛ばす）が直後に現れる最初の`Entry::Pair`に紐づく」というユーザー承認済みの
/// 仕様に従って`PairGroup`のリストへ変換する。紐づけ先が見つからなかった
/// コメント・`Malformed`・`SkippedLeadingSpace`の行数を`dropped`として返す
/// （これらは`--reorder`後の位置が一意に決まらないため出力から削除される）。
///
/// 紐づけが成立しない具体的なケース（ユーザー承認済みの仕様）:
/// - 保留中のコメントの直後に`Malformed`/`SkippedLeadingSpace`が来た場合
///   （その行自体とコメントは道連れでdropされる）
/// - セグメント（`format_reordered`が`Entry::Separator`で分割した単位）の末尾まで
///   `Pair`が現れなかった場合（末尾コメントとして残りcommentsもdropされる。
///   `format_reordered`が呼び出し元でセグメントごとに独立してこの関数を呼ぶため、
///   セグメント境界をまたいだ紐づけは構造上発生しない）
fn collect_pair_groups<'a>(entries: &'a [Entry]) -> (Vec<PairGroup<'a>>, usize) {
    let mut groups = Vec::new();
    let mut pending_comments: Vec<&'a str> = Vec::new();
    let mut dropped = 0usize;

    for entry in entries {
        match entry {
            Entry::Comment(s) => pending_comments.push(s.as_str()),
            Entry::Blank => {
                // 保留中のコメントとPairの間の空行は読み飛ばすだけで、
                // 紐づけ判定には影響しない（保留中コメントはリセットしない）。
            }
            Entry::Pair { key, value } => {
                groups.push(PairGroup {
                    comments: std::mem::take(&mut pending_comments),
                    key,
                    value,
                });
            }
            Entry::SkippedLeadingSpace(_) | Entry::Malformed(_) => {
                // 保留中のコメント（あれば）とこの行自体は道連れでdropする。
                dropped += pending_comments.len() + 1;
                pending_comments.clear();
            }
            Entry::Separator(_) => {
                // format_reorderedがセグメント分割後に呼ぶため、このentries内には
                // 通常出現しない。念のため保留中コメントを道連れdropする防御的分岐。
                dropped += pending_comments.len();
                pending_comments.clear();
            }
        }
    }

    // セグメント末尾まで紐づけ先が見つからなかった末尾コメント。
    dropped += pending_comments.len();

    (groups, dropped)
}

fn format_reordered_segment(
    entries: &[Entry],
    spec: &OrderSpec,
    lang: Language,
) -> (String, Vec<String>) {
    let mut warnings = Vec::new();

    let (pairs, dropped) = collect_pair_groups(entries);
    if dropped > 0 {
        warnings.push(t!(lang,
            ja: "--reorder: コメント/スキップ行/不正行 {dropped} 件は並び替え後の位置が一意に決まらないため出力から削除されました",
            en: "--reorder: {dropped} comment/skipped/malformed line(s) were dropped from the output \
                 because their position after reordering would not be well-defined",
            dropped = dropped,
        ));
    }

    let unknown_idx = spec
        .sections
        .iter()
        .position(|s| matches!(s, Section::Unknown));

    let mut groups: Vec<Vec<PairGroup>> = (0..spec.sections.len()).map(|_| Vec::new()).collect();

    for pair in pairs {
        let matched_idx = spec.sections.iter().position(|section| match section {
            Section::Named(names) => names.contains(&pair.key.as_str()),
            Section::Bracket(prefixes) => prefixes.iter().any(|p| pair.key.starts_with(p)),
            Section::Unknown => false,
        });
        // matched_idxが無い場合はunknown_idxへフォールバックする。どちらも無い
        // （このOrderSpecにSection::Unknownが無い）場合、このpairはどのセクションにも
        // 属せず出力から漏れる。既存のOrderSpec設計（全obj種別が終端にSection::Unknownを
        // 持つ）を前提とした従来の挙動をそのまま維持している。
        if let Some(i) = matched_idx.or(unknown_idx) {
            groups[i].push(pair);
        }
    }

    for (i, section) in spec.sections.iter().enumerate() {
        match section {
            Section::Named(names) => {
                groups[i].sort_by_key(|pair| {
                    names.iter().position(|n| n == &pair.key.as_str()).unwrap()
                });
            }
            Section::Bracket(_) => {
                groups[i].sort_by_key(|pair| (bracket_indices(pair.key), pair.key.clone()));
            }
            Section::Unknown => {} // 挿入順（パース順）を保持。安定ソート前提でコメント対応も保たれる
        }
    }

    let mut out = String::new();
    let mut first_group = true;
    for group in &groups {
        if group.is_empty() {
            continue;
        }
        if !first_group {
            out.push('\n');
        }
        first_group = false;
        for pair in group {
            for comment in &pair.comments {
                out.push_str(comment);
                out.push('\n');
            }
            out.push_str(pair.key);
            out.push('=');
            out.push_str(pair.value.trim());
            out.push('\n');
        }
    }

    (out, warnings)
}

fn bracket_indices(key: &str) -> Vec<i64> {
    let Some(start) = key.find('[') else {
        return Vec::new();
    };
    key[start..]
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split("][")
        .map(|s| s.parse::<i64>().unwrap_or(0))
        .collect()
}
