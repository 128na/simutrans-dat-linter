//! 診断メッセージの言語切り替え（`RuleContext.language`経由）が実際に
//! `Diagnostic.message`へ反映されることを確認する統合テスト。
//!
//! `config.rs`のユニットテストは`LintConfig::language()`の解決ロジック
//! （TOMLパース・デフォルト値）を検証しているが、それが実際にルール実装の
//! メッセージ文言まで届いているかは別の懸念事項のため、ここで
//! `RuleContext`を直接組み立てて日本語・英語両方の出力を確認する。

use dat_linter::diagnostics::Severity;
use dat_linter::i18n::Language;
use dat_linter::parser::DatFile;
use dat_linter::registry::{RuleContext, RuleSet};
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定言語で`obj=building`のルールセットを実行し、`missing-cursor-icon`
/// エラーの`Diagnostic.message`本文を返す。
fn missing_cursor_icon_message(language: Language) -> String {
    let dir = testdata_dir();
    let path = dir.join("broken_no_icon.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");
    let ctx = RuleContext {
        dat: &dat,
        dat_dir: &dir,
        language,
    };
    let rule_set = RuleSet::for_obj_type("building", &dat).expect("building は対応済みのはず");
    let diags = rule_set.run(&ctx);
    let diag = diags
        .iter()
        .find(|d| d.severity == Severity::Error && d.code == "missing-cursor-icon")
        .expect("missing-cursor-icon エラーが出るはず");
    diag.message.clone()
}

#[test]
fn japanese_language_produces_japanese_message() {
    let msg = missing_cursor_icon_message(Language::Japanese);
    assert!(
        msg.contains("未指定"),
        "日本語メッセージが期待通りではない: {msg:?}"
    );
}

#[test]
fn english_language_produces_english_message() {
    let msg = missing_cursor_icon_message(Language::English);
    assert!(
        msg.contains("unspecified"),
        "英語メッセージが期待通りではない: {msg:?}"
    );
    assert!(
        !msg.contains("未指定"),
        "英語選択時に日本語文字列が混入している: {msg:?}"
    );
}

#[test]
fn default_language_is_english() {
    // Language::default()がEnglishであること自体はi18n.rs側の単体テストで
    // 確認済みだが、ここではRuleContext経由で実際にデフォルト値を使った場合の
    // 出力がEnglishになることも合わせて確認する（config.rsのLintConfig::language()
    // がデフォルトでLanguage::default()を返す設計と整合しているかの回帰）。
    let msg = missing_cursor_icon_message(Language::default());
    assert!(
        msg.contains("unspecified"),
        "デフォルト言語がEnglishではない: {msg:?}"
    );
}

/// `common::check_duplicate_keys`（obj種別を問わず適用される重複キー警告）も
/// 同様に言語切り替えが効くことを確認する（`RuleContext`を経由しない
/// スタンドアロン関数のため別枠で検証する）。
#[test]
fn duplicate_key_warning_switches_language() {
    let dir = testdata_dir();
    let path = dir.join("duplicate_key.dat");
    let dat = DatFile::parse(&path).expect("パースに失敗");

    let ja = dat_linter::rules::check_duplicate_keys(&dat, Language::Japanese);
    let en = dat_linter::rules::check_duplicate_keys(&dat, Language::English);

    assert_eq!(ja.len(), 1);
    assert_eq!(en.len(), 1);
    assert!(ja[0].message.contains("複数回定義"));
    assert!(en[0].message.contains("more than once"));
}
