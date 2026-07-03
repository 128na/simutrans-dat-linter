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

pub fn parse_entries(text: &str) -> ParsedDat {
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
            warnings.push(format!(
                "line {lineno}: 行頭にスペースがあるため makeobj から無視されます（コメント扱い）: \"{line}\""
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
            warnings.push(format!(
                "line {lineno}: '=' が無いため makeobj から無視されます: \"{line}\""
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
pub fn format_reordered(entries: &[Entry], obj: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();

    let Some(spec) = order_for(obj) else {
        warnings.push(format!(
            "--reorder: obj={obj} は並び替えに未対応です。元の順序のまま出力します"
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
        let (segment_out, segment_warnings) = format_reordered_segment(segment, spec);
        out.push_str(&segment_out);
        warnings.extend(segment_warnings);
    }

    (out, warnings)
}

fn format_reordered_segment(entries: &[Entry], spec: &OrderSpec) -> (String, Vec<String>) {
    let mut warnings = Vec::new();

    let mut pairs: Vec<(&String, &String)> = Vec::new();
    let mut dropped = 0usize;

    for entry in entries {
        match entry {
            Entry::Pair { key, value } => pairs.push((key, value)),
            Entry::Blank => {}
            _ => dropped += 1,
        }
    }
    if dropped > 0 {
        warnings.push(format!(
            "--reorder: コメント/スキップ行/不正行 {dropped} 件は並び替え後の位置が一意に決まらないため出力から削除されました"
        ));
    }

    let unknown_idx = spec
        .sections
        .iter()
        .position(|s| matches!(s, Section::Unknown));

    let mut groups: Vec<Vec<(&String, &String)>> =
        (0..spec.sections.len()).map(|_| Vec::new()).collect();

    for (k, v) in &pairs {
        let mut placed = false;
        for (i, section) in spec.sections.iter().enumerate() {
            let matched = match section {
                Section::Named(names) => names.contains(&k.as_str()),
                Section::Bracket(prefixes) => prefixes.iter().any(|p| k.starts_with(p)),
                Section::Unknown => false,
            };
            if matched {
                groups[i].push((k, v));
                placed = true;
                break;
            }
        }
        if !placed && let Some(i) = unknown_idx {
            groups[i].push((k, v));
        }
    }

    for (i, section) in spec.sections.iter().enumerate() {
        match section {
            Section::Named(names) => {
                groups[i].sort_by_key(|(k, _)| names.iter().position(|n| n == k).unwrap());
            }
            Section::Bracket(_) => {
                groups[i].sort_by_key(|(k, _)| (bracket_indices(k), (*k).clone()));
            }
            Section::Unknown => {} // 挿入順（パース順）を保持
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
        for (k, v) in group {
            out.push_str(k);
            out.push('=');
            out.push_str(v.trim());
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
