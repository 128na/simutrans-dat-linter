use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// 1つのkey=valueペア。`line`は**最初に**このキーが出現した行番号（1始まり）。
pub struct Entry {
    pub value: String,
    pub line: usize,
}

/// 同一obj内でキーが複数回定義された事実。`first_line`が採用された値の行、
/// `duplicate_line`は無視された（makeobjが読み捨てる）方の行。
pub struct DuplicateKey {
    pub key: String,
    pub first_line: usize,
    pub duplicate_line: usize,
}

/// 簡易 .dat パーサ。makeobj の tabfile_t::read() を模倣する:
/// キーは前後空白をトリムして小文字化、'['と']'の間の空白は除去する。
/// 値は tabfile.cc と同様に**一切トリムしない**（`name= Hoge`のような書き方で
/// 値に意図しない先頭スペースが混入するのを、linterが正しく検出できるようにするため）。
///
/// 重複キーは**先勝ち**（`tabfileobj_t::put()`の実装 `if(objinfo.get(key).str) return false;`
/// と `tabfile.h`のdocコメント「If keys are duplicated for one object, the first value is used」
/// を根拠とする。以前の実装は`BTreeMap::insert()`による後勝ちで、実際のmakeobjと異なっていた）。
pub struct DatFile {
    pub pairs: BTreeMap<String, Entry>,
    pub duplicates: Vec<DuplicateKey>,
}

impl DatFile {
    pub fn parse(path: &Path) -> std::io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let mut pairs: BTreeMap<String, Entry> = BTreeMap::new();
        let mut duplicates = Vec::new();

        for (i, raw_line) in text.lines().enumerate() {
            let lineno = i + 1;
            let line = raw_line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') || line.starts_with(' ') {
                continue;
            }
            let Some((key_raw, value)) = line.split_once('=') else {
                continue;
            };
            let key = format_key(key_raw);
            match pairs.get(&key) {
                Some(existing) => {
                    duplicates.push(DuplicateKey {
                        key,
                        first_line: existing.line,
                        duplicate_line: lineno,
                    });
                }
                None => {
                    pairs.insert(
                        key,
                        Entry {
                            value: value.to_string(),
                            line: lineno,
                        },
                    );
                }
            }
        }

        Ok(DatFile { pairs, duplicates })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.get(key).map(|e| e.value.as_str())
    }

    /// キーが最初に出現した行番号。値が存在しないキーには使えない。
    pub fn line_of(&self, key: &str) -> Option<usize> {
        self.pairs.get(key).map(|e| e.line)
    }

    pub fn get_ints(&self, key: &str) -> Vec<i64> {
        match self.get(key) {
            Some(v) => v
                .split(',')
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect(),
            None => Vec::new(),
        }
    }
}

/// `tabfile_t::format_key()` を模倣: 末尾空白トリム、小文字化、'['と']'内の空白除去。
/// formatter.rs からも参照する。
pub(crate) fn format_key(raw: &str) -> String {
    let trimmed = raw.trim_end();
    let mut out = String::with_capacity(trimmed.len());
    let mut in_bracket = false;
    for c in trimmed.chars() {
        match c {
            '[' => {
                in_bracket = true;
                out.push(c);
            }
            ']' => {
                in_bracket = false;
                out.push(c);
            }
            ' ' if in_bracket => {}
            _ => out.push(c.to_ascii_lowercase()),
        }
    }
    out
}
