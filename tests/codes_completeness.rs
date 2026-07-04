//! `src/codes.rs`の`ALL_CODES`（`dat_linter list`が表示する一覧）が、実際の
//! ソースファイルで使われている`Diagnostic::error/warning/info/debug("code", ...)`
//! の全codeと過不足なく一致することを保証する。
//!
//! `ALL_CODES`は静的な配列（`Rule::check`は実データが無いと実行できず、完全な
//! 動的収集が非現実的なため。`src/codes.rs`冒頭のdocコメント参照）だが、
//! ルールを追加・削除したのにこの一覧の更新を忘れるとこのテストが落ちることで
//! ドリフトを検出する。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// 「実ソース中で`Diagnostic::error/warning/info/debug("code", ...)`として
/// 発行されることは無いが、`src/codes.rs::ALL_CODES`（延いては`dat_linter list`/
/// `describe`）には正規のcodeとして載せたい」機能トグル専用codeの明示的allowlist。
///
/// 第12弾: `fmt-reorder-applied`が該当する。これは`fmt`のreorder機能そのものを
/// 表すcodeで、`[rules] include/exclude`で機能の有効/無効を切り替えるためだけに
/// 使われる（`config.is_enabled("fmt-reorder-applied")`）。第11弾では実際に
/// `Diagnostic::info("fmt-reorder-applied", ...)`を生成しeprintln!していたが、
/// これにより問題の無い通常の`fmt`実行が毎回1行stderrへ出力するようになり、
/// 「指摘が無ければ完全silent」というlint/analyzeと同じ方針に反する副作用が
/// あった（Main側で発見）。診断として表示する必要は無い（`dat_linter describe
/// fmt-reorder-applied`で説明を見られれば十分）ため、このallowlistへ登録して
/// `Diagnostic::x()`呼び出しを完全に取りやめた。
///
/// 新しくこの種のcode（実際には診断として発行しない、include/exclude専用の
/// 機能トグル）を追加する場合は、ここに追記した上でその理由をコメントに残すこと。
const FEATURE_TOGGLE_ONLY_CODES: &[&str] = &["fmt-reorder-applied"];

/// 1ファイルのテキストから`Diagnostic::(error|warning|info|debug)(\s*\n?\s*"code"`
/// の形を素朴に走査してcode文字列を抽出する。`common::check_image_ref`のような
/// 関数越しの間接呼び出しは対象外（呼び出し元の`Diagnostic::error(...)`自体を
/// 直接文字列走査するため、`common.rs`側の定義さえ拾えれば十分）。
fn extract_codes(text: &str) -> Vec<String> {
    let markers = [
        "Diagnostic::error(",
        "Diagnostic::warning(",
        "Diagnostic::info(",
        "Diagnostic::debug(",
    ];
    let mut codes = Vec::new();
    for marker in markers {
        let mut search_from = 0usize;
        while let Some(rel_idx) = text[search_from..].find(marker) {
            let start = search_from + rel_idx + marker.len();
            // マーカー直後から最初の `"..."` を探す（間に改行・空白があってもよい）。
            let rest = &text[start..];
            let Some(quote_start_rel) = rest.find('"') else {
                break;
            };
            let after_quote = &rest[quote_start_rel + 1..];
            let Some(quote_end_rel) = after_quote.find('"') else {
                break;
            };
            let code = &after_quote[..quote_end_rel];
            codes.push(code.to_string());
            search_from = start + quote_start_rel + 1 + quote_end_rel + 1;
        }
    }
    codes
}

#[test]
fn all_codes_matches_actual_source_usage() {
    let root = crate_root();
    let mut actual: BTreeSet<String> = BTreeSet::new();

    let mut source_files = Vec::new();
    let rules_dir = root.join("src").join("rules");
    for entry in std::fs::read_dir(&rules_dir).expect("src/rules を読み込めません") {
        let entry = entry.expect("read_dir entry error");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            source_files.push(path);
        }
    }
    source_files.push(root.join("src").join("couplings.rs"));
    source_files.push(root.join("src").join("formatter").join("mod.rs"));

    assert!(
        source_files.len() >= 3,
        "対象ソースファイルの収集に失敗した可能性: {source_files:?}"
    );

    for path in &source_files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} の読み込みに失敗: {e}", path.display()));
        for code in extract_codes(&text) {
            actual.insert(code);
        }
    }

    let declared: BTreeSet<String> = dat_linter::codes::ALL_CODES
        .iter()
        .map(|c| c.code.to_string())
        .collect();

    let missing_from_declared: Vec<_> = actual.difference(&declared).collect();
    // `FEATURE_TOGGLE_ONLY_CODES`（実ソースで`Diagnostic::x()`として発行されることが
    // 無いと分かっている機能トグル専用code）は「古くなったcode」の判定から除外する。
    let stale_in_declared: Vec<_> = declared
        .difference(&actual)
        .filter(|code| !FEATURE_TOGGLE_ONLY_CODES.contains(&code.as_str()))
        .collect();

    assert!(
        missing_from_declared.is_empty(),
        "ソース上で使われているのに src/codes.rs::ALL_CODES に未登録のcodeがあります: {missing_from_declared:?}"
    );
    assert!(
        stale_in_declared.is_empty(),
        "src/codes.rs::ALL_CODES に登録されているが、実際にはどのソースファイルでも \
         使われていない（古くなった）codeがあります（FEATURE_TOGGLE_ONLY_CODESに\
         登録済みのものは除く）: {stale_in_declared:?}"
    );
}

/// `FEATURE_TOGGLE_ONLY_CODES`自体が古くならないよう、登録されているcodeが
/// `src/codes.rs::ALL_CODES`に実在することを固定する（typo・削除漏れの検出）。
#[test]
fn feature_toggle_only_codes_exist_in_all_codes() {
    let declared: BTreeSet<&str> = dat_linter::codes::ALL_CODES
        .iter()
        .map(|c| c.code)
        .collect();
    for code in FEATURE_TOGGLE_ONLY_CODES {
        assert!(
            declared.contains(code),
            "FEATURE_TOGGLE_ONLY_CODESに登録された{code:?}がsrc/codes.rs::ALL_CODESに存在しません"
        );
    }
}

#[test]
fn all_codes_has_no_duplicate_entries() {
    let mut seen = BTreeSet::new();
    for c in dat_linter::codes::ALL_CODES {
        assert!(
            seen.insert(c.code),
            "src/codes.rs::ALL_CODES に重複したcodeがあります: {}",
            c.code
        );
    }
}

/// 第10弾（項目6）: `dat_linter describe <code>`が空の説明を表示してしまう
/// リグレッションを防ぐ。新規codeを追加する際に`why`/`how_to_fix`のいずれかの
/// 言語を書き忘れると、このテストが落ちる。
#[test]
fn all_codes_have_non_empty_descriptions_in_both_languages() {
    use dat_linter::i18n::Language;

    for c in dat_linter::codes::ALL_CODES {
        assert!(
            !c.why(Language::English).trim().is_empty(),
            "{}: why(English) が空です",
            c.code
        );
        assert!(
            !c.why(Language::Japanese).trim().is_empty(),
            "{}: why(Japanese) が空です",
            c.code
        );
        assert!(
            !c.how_to_fix(Language::English).trim().is_empty(),
            "{}: how_to_fix(English) が空です",
            c.code
        );
        assert!(
            !c.how_to_fix(Language::Japanese).trim().is_empty(),
            "{}: how_to_fix(Japanese) が空です",
            c.code
        );
    }
}
