//! `src/codes.rs`の`DiagnosticCode`・`ALL`・`Diagnostic::error/warning/info/debug`
//! 呼び出しの整合性を保証する。
//!
//! 第17弾（code smellレビュー・タスク13）: `Diagnostic.code`を`&'static str`から
//! `DiagnosticCode`（enum）へ変更したことで、「`Diagnostic::x(code, ...)`に
//! 存在しないcodeを渡してしまう」「`DiagnosticCode::info()`の網羅matchに
//! armを追加し忘れる」といったドリフトの大部分は**コンパイル時**に検出できる
//! ようになった（`as_str`・`info`はワイルドカードarmを持たない網羅matchのため）。
//!
//! ただし`ALL`（`DiagnosticCode`の全variant一覧、`dat_linter list`/`describe`/
//! `from_str`が使う）は`strum`等のenum列挙クレートを使わず手動保持しているため、
//! 「新しいvariantを追加したのに`ALL`への追記を忘れる」というドリフトだけは
//! コンパイラが検出できない（`as_str`/`info`の網羅matchはvariant自体の追加は
//! 強制するが、`ALL`という単なる`&[DiagnosticCode]`定数への追記は強制しない）。
//! このテストはその一点にだけ的を絞った軽量な回帰テストとして残す
//! （以前の`all_codes_matches_actual_source_usage`のような、実ソースの正規表現
//! スキャン自体は、コンパイル時保証で大部分の価値を失ったため削除した。
//! 「`DiagnosticCode::Xxx`という識別子が実ソースに存在するのに`ALL`に無い」
//! ケースを検出する軽量版のみ残す）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// `DiagnosticCode::xxx`の`xxx`部分が実際にはvariant名ではなく、メソッド呼び出し
/// （`DiagnosticCode::from_str(...)`・`DiagnosticCode::info()`のような、
/// docコメントの地の文でも頻出する）である既知の名前一覧。これらは
/// `extract_referenced_variants`の対象から除外する。
const KNOWN_NON_VARIANT_MEMBERS: &[&str] = &["as_str", "from_str", "info"];

/// 1ファイルのテキストから`DiagnosticCode::Xxx`という識別子参照を素朴に走査し、
/// `Xxx`が（1）`KNOWN_NON_VARIANT_MEMBERS`に含まれない、（2）直後に`(`が
/// 続かない（メソッド呼び出し構文ではない）、の両方を満たす場合のみvariant名候補
/// として収集する。`use dat_linter::codes::DiagnosticCode;`のような`use`文自体
/// （`DiagnosticCode`単体、`::`が続かない）や、docコメントの地の文で
/// `DiagnosticCode::MissingWaytypeという`のように直後に日本語が続くケースも
/// 素朴な文字クラス走査（英数字+アンダースコアのみ）で正しく`MissingWaytype`
/// だけを切り出せる。
fn extract_referenced_variants(text: &str) -> Vec<String> {
    let marker = "DiagnosticCode::";
    let mut variants = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel_idx) = text[search_from..].find(marker) {
        let start = search_from + rel_idx + marker.len();
        let rest = &text[start..];
        // Rust識別子はASCII英数字+アンダースコアのみ（`char::is_alphanumeric()`は
        // 日本語文字も真になるため、`is_ascii_alphanumeric()`で明示的に絞る。
        // これにより`DiagnosticCode::FmtReorderAppliedという`のような、docコメントの
        // 地の文がスペース無しで直後に続くケースでも正しく`FmtReorderApplied`のみを
        // 切り出せる）。
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if end > 0 {
            let name = &rest[..end];
            let followed_by_call = rest[end..].trim_start().starts_with('(');
            if !followed_by_call && !KNOWN_NON_VARIANT_MEMBERS.contains(&name) {
                variants.push(name.to_string());
            }
        }
        search_from = start + end.max(1);
    }
    variants
}

/// 第17弾: `ALL`定数配列（`DiagnosticCode`の全variant一覧、手動保持）が
/// 実ソースで参照されている全variantを過不足なくカバーしていることを確認する。
/// `as_str`/`info`の網羅matchはコンパイラが強制するため、ここでは`ALL`という
/// 単なる配列への追記漏れだけを検出すればよい。
#[test]
fn all_variants_referenced_in_source_are_in_all_array() {
    let root = crate_root();
    let mut referenced: BTreeSet<String> = BTreeSet::new();

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
    source_files.push(root.join("src").join("codes.rs"));

    let commands_dir = root.join("src").join("commands");
    for entry in std::fs::read_dir(&commands_dir).expect("src/commands を読み込めません") {
        let entry = entry.expect("read_dir entry error");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            source_files.push(path);
        }
    }

    assert!(
        source_files.len() >= 3,
        "対象ソースファイルの収集に失敗した可能性: {source_files:?}"
    );

    for path in &source_files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} の読み込みに失敗: {e}", path.display()));
        for variant in extract_referenced_variants(&text) {
            referenced.insert(variant);
        }
    }

    let in_all: BTreeSet<String> = dat_linter::codes::ALL
        .iter()
        .map(|c| format!("{c:?}"))
        .collect();

    let missing_from_all: Vec<_> = referenced.difference(&in_all).collect();
    assert!(
        missing_from_all.is_empty(),
        "実ソースで参照されているのに src/codes.rs::ALL に未登録のDiagnosticCode variantが\
         あります: {missing_from_all:?}"
    );
}

#[test]
fn all_array_has_no_duplicate_entries() {
    let mut seen = BTreeSet::new();
    for c in dat_linter::codes::ALL {
        assert!(
            seen.insert(c.as_str()),
            "src/codes.rs::ALL に重複したcodeがあります: {}",
            c.as_str()
        );
    }
}

/// `DiagnosticCode::as_str()`と`DiagnosticCode::from_str()`が互いに逆変換の
/// 関係にあることを、`ALL`の全variantについて確認する
/// （`from_str`は`ALL`の線形探索なので、このテスト自体は`as_str`の重複が
/// 無いこと＝上の`all_array_has_no_duplicate_entries`が前提）。
#[test]
fn from_str_is_inverse_of_as_str_for_all_variants() {
    for c in dat_linter::codes::ALL {
        let s = c.as_str();
        assert_eq!(
            dat_linter::codes::DiagnosticCode::from_str(s),
            Some(*c),
            "DiagnosticCode::from_str({s:?}) が {c:?} 自身に戻らない"
        );
    }
}

#[test]
fn from_str_rejects_unknown_code() {
    assert_eq!(
        dat_linter::codes::DiagnosticCode::from_str("this-code-does-not-exist"),
        None
    );
}

/// 第10弾（項目6）: `dat_linter describe <code>`が空の説明を表示してしまう
/// リグレッションを防ぐ。新規codeを追加する際に`why`/`how_to_fix`のいずれかの
/// 言語を書き忘れると、このテストが落ちる。
#[test]
fn all_codes_have_non_empty_descriptions_in_both_languages() {
    use dat_linter::i18n::Language;

    for c in dat_linter::codes::ALL {
        let info = c.info();
        assert!(
            !info.why(Language::English).trim().is_empty(),
            "{}: why(English) が空です",
            c.as_str()
        );
        assert!(
            !info.why(Language::Japanese).trim().is_empty(),
            "{}: why(Japanese) が空です",
            c.as_str()
        );
        assert!(
            !info.how_to_fix(Language::English).trim().is_empty(),
            "{}: how_to_fix(English) が空です",
            c.as_str()
        );
        assert!(
            !info.how_to_fix(Language::Japanese).trim().is_empty(),
            "{}: how_to_fix(Japanese) が空です",
            c.as_str()
        );
    }
}
