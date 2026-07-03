//! `tabfile_t::read()`（`dataobj/tabfile.cc:331-512`）が実装するパラメータ展開構文を
//! パーサの前処理として再現するモジュール。
//!
//! ## 対応する構文（実データで確認できた範囲）
//! - キー中の`[...]`フィールドに書かれた**数値**のカンマ区切りリスト
//!   （例 `[0,1,2,3,4,5,6,7]`）またはダッシュ範囲（例 `[0-7]`）。1行のキーが
//!   その`[...]`フィールドの値の個数だけ複数の実キーに展開される。
//! - 値中の`<...>`算術式。`$N`はN番目（0始まり）の`[...]`フィールドの、その回の
//!   展開における実際の値を指す。`+`/`-`/`*`/`/`/`%`の二項演算子に対応する。
//!
//! ## スコープ外（REJECTED）: 方向名（ribi）文字列パラメータ展開
//! `tabfile.cc`の`match_ribi()`（319-327行目）は`[...]`フィールドの中身が
//! `"n"`/`"e"`/`"s"`/`"w"`で始まる、または`"-"`直後が数字でない場合に
//! 「ribi文字列パラメータ」として扱う分岐を持つ（`parameter_name[]`配列に文字列を
//! 蓄積し、展開時は数値の代わりにこの文字列をキーへ埋め込む）。しかし
//! `refs/linter_test`の実データ（`KSN-128op_Rail-yard_0001.dat`等）で確認できた
//! パラメータ展開は数値カンマリスト・数値ダッシュ範囲・`$N`算術参照のみで、
//! 方向名文字列を使うキーは1つも出現しなかった。実データで検証できない構文を
//! 実装すると根拠の無い挙動を持ち込むことになるため、今回はこのribi文字列分岐を
//! 意図的に対象外とする（`match_ribi`自体は`tabfile.cc`に実在し、将来ribiパラメータ
//! 展開を使う`.dat`が見つかった場合は本モジュールの拡張対象になる）。
//!
//! ## `*`/`%`演算子について
//! `tabfile.cc`の`calculate_internal`は`+`/`-`/`*`/`/`/`%`の5演算子全てに対応するが、
//! 実データで確認できたのは`<$0>`（参照そのまま）と`<$-8>`（`$`直後に数字が続かない
//! ため`strtok`が`$`と`8`に分割し`-`が減算演算子になる、実質`$0 - 8`）の2パターンのみ。
//! `*`/`%`は実データに出現しないが、`calculate_internal`の実装は演算子を機械的に
//! 走査するだけで`+`/`-`と扱いに差が無いため、REJECTEDにはせず併せて実装する
//! （数値展開自体をサポートする以上、同じ`calculate_internal`ロジックの一部である
//! 演算子だけを恣意的に除外する理由が無いため）。

#[cfg(test)]
use std::collections::BTreeMap;

/// 1つの`[...]`パラメータフィールドが展開する数値のリスト。
type ParamValues = Vec<i64>;

/// 1行の`key=value`をパラメータ展開する。展開が不要な行（`[...]`にカンマ/ダッシュを
/// 含まない、または`<...>`を含まない）はそのまま`vec![(key, value)]`を返す。
///
/// `key`は呼び出し前に`format_key`（小文字化・`[`/`]`内空白除去）済みであること
/// （実際のmakeobjも`format_key`を先に適用してから展開解析するため、この順序は
/// `find_parameter_expansion`の呼び出し順と一致させている）。
pub fn expand_line(key: &str, value: &str) -> Vec<(String, String)> {
    let params = find_bracket_params(key);
    let has_value_expansion = value.contains('<') && value.contains('>');

    if params.is_empty() && !has_value_expansion {
        return vec![(key.to_string(), value.to_string())];
    }

    // パラメータフィールド(bracket index -> 展開後の値リスト)を解決する。
    // 展開対象でない([...]内にカンマもダッシュも無い)フィールドはNoneのままにし、
    // 元の中身をそのまま1要素のリストとして扱う。
    let mut resolved: Vec<Option<ParamValues>> = Vec::new();
    for p in &params {
        if p.is_expansion {
            resolved.push(Some(expand_numeric_field(&p.content)));
        } else {
            resolved.push(None);
        }
    }

    // 展開対象が1つも無ければ(=[...]は全部非展開)、パラメータ展開なしのパスへ。
    let any_key_expansion = resolved.iter().any(|r| r.is_some());
    if !any_key_expansion && !has_value_expansion {
        return vec![(key.to_string(), value.to_string())];
    }

    // combinations = 展開対象フィールドの値の個数の積(tabfile.cc: combinations*=parameter_values[i])。
    // 非展開フィールドは1個として扱う。
    let counts: Vec<usize> = resolved
        .iter()
        .map(|r| r.as_ref().map(|v| v.len()).unwrap_or(1))
        .collect();
    let combinations: usize = counts.iter().product::<usize>().max(1);

    let mut out = Vec::with_capacity(combinations);
    for c in 0..combinations {
        // tabfile.cc:438-444 と同じ「桁上げ」方式でこの回の各フィールドの値を決める。
        let mut combination: Vec<i64> = Vec::with_capacity(params.len());
        let mut acc = c;
        for (i, count) in counts.iter().enumerate() {
            let idx = acc % count;
            acc /= count;
            let value = match &resolved[i] {
                Some(values) => values[idx],
                // 非展開フィールドは値を持たないが、$N参照の対象にはなり得ないため
                // プレースホルダとして0を入れる（実際には使われない）。
                None => 0,
            };
            combination.push(value);
        }

        let expanded_key = build_expanded_key(key, &params, &resolved, &combination);
        let expanded_value = if has_value_expansion {
            expand_value_expression(value, &combination)
        } else {
            value.to_string()
        };
        out.push((expanded_key, expanded_value));
    }

    out
}

/// キー中の`[...]`フィールド1つ分の情報。
struct BracketParam {
    /// `[`と`]`の間の中身（例 `"0,1,2,3,4,5,6,7"` や `"0"`）。
    content: String,
    /// キー中でこのフィールドが占める範囲（`[`を含み`]`を含む）。置換用。
    span: (usize, usize),
    /// カンマ/ダッシュを含み、実際に展開対象となるフィールドか。
    is_expansion: bool,
}

/// `find_parameter_expansion`のキー側走査を再現する: `[...]`を全て見つけ、
/// 中身にカンマ/ダッシュを含むものだけを`is_expansion=true`とする。
/// 位置(バイトオフセット)は元の`key`文字列に対するもの。
fn find_bracket_params(key: &str) -> Vec<BracketParam> {
    let bytes = key.as_bytes();
    let mut params = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let start = i;
            let content_start = i + 1;
            let mut j = content_start;
            let mut has_comma_or_dash = false;
            while j < bytes.len() && bytes[j] != b']' {
                // tabfile.cc: `*(s-1) != '['` の条件どおり、`[`直後の位置に
                // 現れる`-`（負数のような書き方）はパラメータ扱いにしない。
                if (bytes[j] == b',' || bytes[j] == b'-') && j != content_start {
                    has_comma_or_dash = true;
                }
                j += 1;
            }
            if j < bytes.len() {
                // j はここで ']' の位置
                let content = key[content_start..j].to_string();
                params.push(BracketParam {
                    content,
                    span: (start, j + 1),
                    is_expansion: has_comma_or_dash,
                });
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    params
}

/// 1つの`[...]`フィールドの中身（カンマ/ダッシュ区切り）を数値リストへ展開する。
/// `tabfile.cc:384-428`の数値専用パスを再現する（ribi文字列分岐は対象外、
/// モジュール冒頭のREJECTEDコメント参照）。
fn expand_numeric_field(content: &str) -> ParamValues {
    let mut values = Vec::new();
    // strtokは"-,"をデリミタとして扱うため、まず","で素朴に分割してから
    // それぞれのトークンを"-"でさらに分割し、range展開する。
    // 例: "0,1,2,3,4,5,6,7" -> ["0","1",...,"7"]（dashなし、そのままatoi）
    //     "0-7" -> ["0-7"] -> start=0, end=7 -> [0,1,...,7]
    for comma_part in content.split(',') {
        if let Some((start_str, end_str)) = comma_part.split_once('-') {
            // ダッシュ範囲。tabfile.cc: 開始値は既にparameter_value[i][0]として
            // 積まれた上で、range=start..end(exclusive)についてrange+1を追加する。
            // つまり [start, start+1, ..., end] （両端含む）。
            let start = start_str.trim().parse::<i64>().unwrap_or(0);
            let end = end_str.trim().parse::<i64>().unwrap_or(start);
            values.push(start);
            let mut range = start;
            while range < end {
                values.push(range + 1);
                range += 1;
            }
        } else {
            values.push(comma_part.trim().parse::<i64>().unwrap_or(0));
        }
    }
    if values.is_empty() {
        values.push(0);
    }
    values
}

/// この回の`combination`を使ってキーを展開する。展開対象フィールドは実際の値の
/// 数値へ、非展開フィールドは元の中身のまま書き戻す。
fn build_expanded_key(
    key: &str,
    params: &[BracketParam],
    resolved: &[Option<ParamValues>],
    combination: &[i64],
) -> String {
    let mut out = String::with_capacity(key.len());
    let mut prev_end = 0usize;
    for (i, p) in params.iter().enumerate() {
        out.push_str(&key[prev_end..p.span.0]);
        out.push('[');
        if resolved[i].is_some() {
            out.push_str(&combination[i].to_string());
        } else {
            out.push_str(&p.content);
        }
        out.push(']');
        prev_end = p.span.1;
    }
    out.push_str(&key[prev_end..]);
    out
}

/// 値中の`<...>`式を全て評価し、算術結果の数値へ置換する。
fn expand_value_expression(value: &str, combination: &[i64]) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('<') {
        let Some(end_rel) = rest[start..].find('>') else {
            // 対応する'>'が無ければそれ以上展開せず残りをそのまま出力する。
            out.push_str(rest);
            return out;
        };
        let end = start + end_rel;
        out.push_str(&rest[..start]);
        let expr = &rest[start + 1..end];
        let result = evaluate_expression(expr, combination);
        out.push_str(&result.to_string());
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `calculate`/`calculate_internal`（tabfile.cc:603-841）を再現した簡易評価器。
/// 左結合で`+`/`-`/`*`/`/`/`%`を処理する（元の実装は括弧を挿入してから再帰評価する
/// 方式だが、実データに現れる式は単項の`$N`単体か`$N op literal`程度の単純な
/// 左結合式のみのため、括弧無し左結合の素朴な実装で同じ結果になる。括弧を含む式は
/// 実データに出現しないためサポートしない）。
fn evaluate_expression(expr: &str, combination: &[i64]) -> i64 {
    // 空白除去（tabfile.cc: add_operator_parensの冒頭で空白を除去する）。
    let cleaned: String = expr.chars().filter(|c| !c.is_whitespace()).collect();

    let mut tokens: Vec<Token> = Vec::new();
    let mut current = String::new();

    fn flush_operand(current: &mut String, tokens: &mut Vec<Token>, combination: &[i64]) {
        if current.is_empty() {
            return;
        }
        if let Some(rest) = current.strip_prefix('$') {
            // tabfile.cc: `atoi(token_ptr+1)`。`$`の直後に数字が続かない場合
            // （例 `$-8`のように`$`単体しか残らない場合）は`atoi("")==0`となり
            // 暗黙に$0を指す。
            let idx: usize = rest.parse().unwrap_or(0);
            let v = combination.get(idx).copied().unwrap_or(0);
            tokens.push(Token::Num(v));
        } else {
            tokens.push(Token::Num(current.parse::<i64>().unwrap_or(0)));
        }
        current.clear();
    }

    for c in cleaned.chars() {
        match c {
            '+' | '-' | '*' | '/' | '%' => {
                // `$`の直後の`-`は「$と-8に分割」（strtokの区切り文字に'$'は
                // 含まれないが、'-'は含まれるためこう解釈される）というmemoの通り、
                // '$'単体はここで値0の被演算子として確定し、続く'-'は演算子になる。
                flush_operand(&mut current, &mut tokens, combination);
                tokens.push(Token::Op(c));
            }
            _ => current.push(c),
        }
    }
    flush_operand(&mut current, &mut tokens, combination);

    // 左結合で評価する。
    let mut iter = tokens.into_iter();
    let Some(Token::Num(mut answer)) = iter.next() else {
        return 0;
    };
    let mut pending_op: Option<char> = None;
    for tok in iter {
        match tok {
            Token::Op(op) => pending_op = Some(op),
            Token::Num(v) => {
                if let Some(op) = pending_op.take() {
                    answer = apply_op(answer, op, v);
                }
            }
        }
    }
    answer
}

enum Token {
    Num(i64),
    Op(char),
}

fn apply_op(lhs: i64, op: char, rhs: i64) -> i64 {
    match op {
        '+' => lhs + rhs,
        '-' => lhs - rhs,
        '*' => lhs * rhs,
        '/' => {
            if rhs == 0 {
                0
            } else {
                lhs / rhs
            }
        }
        '%' => {
            if rhs == 0 {
                0
            } else {
                lhs % rhs
            }
        }
        _ => rhs,
    }
}

/// テスト用: 展開結果をBTreeMapへ集約するヘルパー（テストの可読性のため）。
#[cfg(test)]
pub(crate) fn expand_to_map(key: &str, value: &str) -> BTreeMap<String, String> {
    expand_line(key, value).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_expansion_when_no_brackets_needed() {
        let out = expand_line("name", "Foo");
        assert_eq!(out, vec![("name".to_string(), "Foo".to_string())]);
    }

    #[test]
    fn single_index_bracket_is_not_expansion() {
        // "[0]"はカンマもダッシュも含まないため展開対象にならない。
        let out = expand_line("backimage[0][0][0][0][0]", "foo.0.0");
        assert_eq!(
            out,
            vec![(
                "backimage[0][0][0][0][0]".to_string(),
                "foo.0.0".to_string()
            )]
        );
    }

    #[test]
    fn comma_list_expands_to_one_entry_per_value() {
        let out = expand_line("backimage[0,1,2,3,4,5,6,7][0][0][0][0][0]", "foo.0.<$0>");
        assert_eq!(out.len(), 8);
        assert_eq!(
            out[0],
            (
                "backimage[0][0][0][0][0][0]".to_string(),
                "foo.0.0".to_string()
            )
        );
        assert_eq!(
            out[7],
            (
                "backimage[7][0][0][0][0][0]".to_string(),
                "foo.0.7".to_string()
            )
        );
    }

    #[test]
    fn dash_range_is_inclusive_both_ends() {
        // KSN-128op_Rail-yard_0001.dat の実例:
        // BackImage[8,9,10,11,12,13,14,15][...]=...2.<$-8>
        // これと同義の[8-15]表記でも同じ8個(8..=15)になることを確認する。
        let out = expand_line("backimage[8-15][0][0][0][0][0]", "foo.2.<$-8>");
        assert_eq!(out.len(), 8);
        // $-8は"$"(=$0, 値8)から8を引く: 8-8=0, 9-8=1, ..., 15-8=7
        assert_eq!(
            out[0],
            (
                "backimage[8][0][0][0][0][0]".to_string(),
                "foo.2.0".to_string()
            )
        );
        assert_eq!(
            out[7],
            (
                "backimage[15][0][0][0][0][0]".to_string(),
                "foo.2.7".to_string()
            )
        );
    }

    #[test]
    fn real_ksn_rail_yard_example_expands_correctly() {
        // 実データ(KSN-128op_Rail-yard_0001.dat)そのままの構文。
        let out = expand_line(
            "backimage[0,1,2,3,4,5,6,7][0][0][0][0][0]",
            "KSN-128op_Rail-yard_0001.           0.<$0>",
        );
        assert_eq!(out.len(), 8);
        assert_eq!(
            out[0],
            (
                "backimage[0][0][0][0][0][0]".to_string(),
                "KSN-128op_Rail-yard_0001.           0.0".to_string()
            )
        );

        let out2 = expand_line(
            "backimage[8,9,10,11,12,13,14,15][0][0][0][0][0]",
            "KSN-128op_Rail-yard_0001.     2.<$-8>",
        );
        assert_eq!(out2.len(), 8);
        assert_eq!(
            out2[0],
            (
                "backimage[8][0][0][0][0][0]".to_string(),
                "KSN-128op_Rail-yard_0001.     2.0".to_string()
            )
        );
        assert_eq!(
            out2[7],
            (
                "backimage[15][0][0][0][0][0]".to_string(),
                "KSN-128op_Rail-yard_0001.     2.7".to_string()
            )
        );
    }

    #[test]
    fn value_without_dollar_prefix_is_left_as_is_when_no_key_expansion() {
        let out = expand_line("name", "<5+3>");
        assert_eq!(out, vec![("name".to_string(), "8".to_string())]);
    }

    #[test]
    fn multiplication_and_modulo_are_supported() {
        let out = expand_to_map("name", "<3*4>");
        assert_eq!(out.get("name"), Some(&"12".to_string()));
        let out = expand_to_map("name2", "<10%3>");
        assert_eq!(out.get("name2"), Some(&"1".to_string()));
    }
}
