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

/// .dat のテキストを読み込む。UTF-8として不正な場合はShift-JIS(CP932)として
/// デコードし直す（古いpak128.japan系アドオンに日本語コメントがShift-JISで
/// 保存されたまま配布されているケースへの対応。makeobj自体は文字コードを
/// 検証しないため、実際に読み込めるファイルをlinterが「読み込み失敗」で
/// 弾いてしまわないようにする）。
pub fn read_dat_text(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(e) => {
            let bytes = e.into_bytes();
            let (text, _, _had_errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
            Ok(text.into_owned())
        }
    }
}

impl DatFile {
    /// 1ファイル中の**最初のobj定義のみ**をパースする（後方互換用）。
    /// 1ファイルに複数のobj定義が`-`始まりの区切り行で連結されている場合
    /// （例: 建物の複数ステージを1つの.datにまとめた実例）は[`DatFile::parse_all`]
    /// を使うこと。
    pub fn parse(path: &Path) -> std::io::Result<Self> {
        let text = read_dat_text(path)?;
        Ok(parse_records(&text)
            .into_iter()
            .next()
            .unwrap_or_else(|| DatFile {
                pairs: BTreeMap::new(),
                duplicates: Vec::new(),
            }))
    }

    /// 1ファイル中の**全てのobj定義**をパースする。real makeobj
    /// (`tabfile_t::read()`: `while(read_line(...) && *line != '-')`)と同じく、
    /// 行頭が`-`の行（`#`コメントの区切り線ではなく素のダッシュ行）でobj定義を
    /// 区切る。区切られた各obj定義は独立した[`DatFile`]として返るため、
    /// 2つ目以降の定義の`obj=`/`name=`等が1つ目との「重複キー」に
    /// 誤判定されることはない。
    pub fn parse_all(path: &Path) -> std::io::Result<Vec<Self>> {
        let text = read_dat_text(path)?;
        Ok(parse_records(&text))
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

/// テキスト全体を複数のobj定義（レコード）に分割してパースする。
/// 行頭が`-`の行（`#`コメントや行頭スペースはスキップ済みの行のみ判定対象、
/// real makeobjの`read_line()`によるコメントスキップの後で区切り判定する
/// 順序を再現している）に達するたびに、それまで蓄積した1レコードを確定し、
/// 次のレコードの蓄積を始める。空のまま区切り行に達したレコード（区切り行が
/// 連続している場合や末尾の余white）は読み飛ばす（real makeobjの
/// `do { ... } while(!lines && !feof(file))`= 「空オブジェクトはスキップ」を再現）。
/// 行番号はレコードごとにリセットせず、ファイル先頭からの絶対行番号を使う
/// （real makeobjは`current_line_number`をread()呼び出しごとにリセットするが、
/// これは内部の数式展開エラー表示専用でありユーザー向け診断には使われない。
/// 本linterの診断はエディタ上の行番号を指すべきなので絶対行番号が正しい）。
fn parse_records(text: &str) -> Vec<DatFile> {
    let mut records = Vec::new();
    let mut pairs: BTreeMap<String, Entry> = BTreeMap::new();
    let mut duplicates: Vec<DuplicateKey> = Vec::new();

    for (i, raw_line) in text.lines().enumerate() {
        let lineno = i + 1;
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') || line.starts_with(' ') {
            continue;
        }
        if line.starts_with('-') {
            if !pairs.is_empty() {
                records.push(DatFile {
                    pairs: std::mem::take(&mut pairs),
                    duplicates: std::mem::take(&mut duplicates),
                });
            }
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
    if !pairs.is_empty() {
        records.push(DatFile { pairs, duplicates });
    }

    records
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
