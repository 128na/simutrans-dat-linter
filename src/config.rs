//! `lint`/`fmt`/`couplings`共通の設定ファイル（TOML）。
//!
//! ## 配置場所・探索順
//! - `--config <path>` が明示された場合はそのパスのみを読む（存在しなければエラー）。
//! - 明示が無い場合、カレントディレクトリ直下の `dat_linter.toml` を自動探索する。
//!   存在しなければ、[`LintConfig::load_or_default`] が[`generate_default_config_file`]で
//!   コメント付きのデフォルト設定ファイルを**カレントディレクトリに自動生成**した上で、
//!   「設定ファイル無し」相当のデフォルト設定（全ルール有効・`language=en`）を返す
//!   （ユーザー確認済みの決定事項: 生成先はexe隣接ではなく既存の自動探索と同じ
//!   カレントディレクトリ）。生成に失敗しても致命的エラーにはせず、動作は継続する
//!   （書き込み権限が無いディレクトリでも従来通りツールを使い続けられることを優先する）。
//!
//! ## スキーマ
//! ```toml
//! [general]
//! language = "en"  # "en" (デフォルト) または "ja"
//!
//! [fmt]
//! reorder = true  # true (デフォルト) または false
//!
//! [rules]
//! include = ["obsolete-type", "missing-waytype"]
//! exclude = ["duplicate-key"]
//! ```
//!
//! ## include/exclude の意味論
//! 1. `include` が空なら「全ルール（`Diagnostic.code`）が候補」。非空なら
//!    「`include` に列挙された code のみが候補」（それ以外の code は無条件で無効）。
//! 2. 上記の候補集合から、`exclude` に列挙された code をさらに除外する。
//!
//! つまり `exclude` は常に `include` より優先される（同じ code が両方に
//! 書かれていれば無効）。フィルタは`Diagnostic.code`単位の後段フィルタとして
//! 実装しており（`is_enabled`）、`Rule` trait や各obj種別モジュールの構造には
//! 一切手を入れていない（このツールの各ルールは`Diagnostic { code, .. }`を
//! 生成するだけの副作用フリーな設計のため、後段フィルタが最も低リスク）。
//!
//! ## `language`のデフォルト
//! 設定ファイルが無い場合・`[general] language`未指定の場合のデフォルトは
//! **英語（`en`）**（ユーザー確認済みの決定事項。既存の日本語固定挙動から
//! 変更されている点に注意。第1弾実装時点ではメッセージが日本語固定だったが、
//! i18n対応後はこのデフォルトに従う）。
//!
//! ## `[fmt] reorder`のデフォルト
//! 設定ファイルが無い場合・`[fmt] reorder`未指定の場合のデフォルトは**`true`**
//! （`fmt`は慣習的な並び順に並び替えるのがデフォルト挙動。CLIの`--no-reorder`
//! フラグはこの設定を強制的に上書きしてpreserve-order挙動にする。優先順位は
//! 「`--no-reorder`指定 > config設定（未指定時デフォルトtrue）」。第5弾で
//! `--reorder`bool flagから`--no-reorder`へ置き換えた際に導入された設定）。

use crate::i18n::{Language, t};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 自動探索・自動生成時に見る/書くファイル名。
const DEFAULT_CONFIG_FILENAME: &str = "dat_linter.toml";

/// 設定ファイルが存在しない場合にカレントディレクトリへ自動生成する
/// デフォルト内容。コメントでスキーマの意味を説明し、値は全て
/// デフォルト相当（`language = "en"`・include/exclude空）にする。
const DEFAULT_CONFIG_TEMPLATE: &str = r#"# dat_linter configuration file
# Auto-generated on first run. Feel free to edit or delete this file.
#
# [general]
# language: message language for lint/fmt/analyze output.
#   "en" (default) or "ja".
# language = "en"
#
# [fmt]
# reorder: whether `fmt` reorders keys into the conventional order by default.
#   true (default) or false. The CLI flag --no-reorder always overrides this
#   to false for a single invocation, regardless of this setting.
# reorder = true
#
# [rules]
# include: rule codes (Diagnostic.code) to enable. Empty = all rules enabled (default).
# exclude: rule codes to disable, applied after include. exclude always wins.
# include = []
# exclude = []

[general]
language = "en"

[fmt]
reorder = true

[rules]
include = []
exclude = []
"#;

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    general: RawGeneralConfig,
    #[serde(default)]
    fmt: RawFmtConfig,
    #[serde(default)]
    rules: RawRulesConfig,
}

#[derive(Debug, Default, Deserialize)]
struct RawGeneralConfig {
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFmtConfig {
    #[serde(default)]
    reorder: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRulesConfig {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

/// 読み込み・正規化済みの設定。`is_enabled`で`Diagnostic.code`ごとに
/// 有効/無効を判定し、`language()`で出力言語を、`fmt_reorder()`で
/// `fmt`のデフォルト並び替え挙動を取得する。
#[derive(Debug)]
pub struct LintConfig {
    include: HashSet<String>,
    exclude: HashSet<String>,
    language: Language,
    fmt_reorder: bool,
}

impl Default for LintConfig {
    fn default() -> Self {
        LintConfig::all_enabled()
    }
}

impl LintConfig {
    /// 全ルールが有効・言語はデフォルト(English)・fmt reorderはデフォルト(true)の設定
    /// （設定ファイル無しの状態と同義）。
    pub fn all_enabled() -> Self {
        LintConfig {
            include: HashSet::new(),
            exclude: HashSet::new(),
            language: Language::default(),
            fmt_reorder: true,
        }
    }

    /// `--config`指定または自動探索の結果に応じて設定を読み込む。
    ///
    /// - `explicit_path`が`Some`: そのパスを読む。存在しない・パースエラーは`Err`。
    ///   自動生成は行わない（明示指定されたパスが無いのはユーザーの意図的な指定
    ///   ミスの可能性が高く、黙って別の設定を生成すると意図しない挙動になるため）。
    /// - `explicit_path`が`None`: カレントディレクトリの`dat_linter.toml`を探す。
    ///   存在すればそれを読む。存在しなければ[`generate_default_config_file`]で
    ///   自動生成を試みた上で（失敗しても無視する）、`Ok(LintConfig::all_enabled())`
    ///   を返す（エラーにしない。生成の成否に関わらずデフォルト設定で動作を継続する）。
    pub fn load_or_default(explicit_path: Option<&Path>) -> Result<Self, String> {
        match explicit_path {
            Some(path) => Self::load_from(path),
            None => {
                let default_path = PathBuf::from(DEFAULT_CONFIG_FILENAME);
                if default_path.is_file() {
                    Self::load_from(&default_path)
                } else {
                    // 自動生成の成否はここでは無視する。書き込み権限が無い等の理由で
                    // 生成に失敗しても、従来通り設定ファイル無しとして動作を継続する。
                    let _ = generate_default_config_file(&default_path);
                    Ok(Self::all_enabled())
                }
            }
        }
    }

    /// 設定ファイル自体の読み込み・パースに失敗した場合のエラーメッセージ言語について:
    /// この時点では設定ファイルの中身（`[general] language`）がまだ読めていない
    /// （読めていれば、そもそもこの関数は成功して`Ok`を返している）ため、
    /// 「どの言語でエラーを報告すべきか」を設定ファイルから決めることができない
    /// （ニワトリが先か卵が先かの問題）。`--config`引数で明示的に指定した言語も
    /// 存在しない（`Language`はCLI引数ではなく設定ファイル経由でのみ選択される設計）。
    /// そのため、ここでのエラーメッセージは`Language::default()`（English）固定とする。
    /// 実用上も、設定ファイルが読めない/壊れているという状況を報告するメッセージが
    /// 意図した言語（`ja`）で出ないケースよりも、常に一貫した言語（English）で
    /// 出る方が挙動を予測しやすいと判断した。
    fn load_from(path: &Path) -> Result<Self, String> {
        let error_lang = Language::default();
        let text = std::fs::read_to_string(path).map_err(|e| {
            t!(error_lang,
                ja: "{p} を読み込めません ({e})",
                en: "Cannot read {p} ({e})",
                p = path.display(),
                e = e,
            )
        })?;
        let raw: RawConfig = toml::from_str(&text).map_err(|e| {
            t!(error_lang,
                ja: "{p} のTOML解析に失敗しました ({e})",
                en: "Failed to parse TOML in {p} ({e})",
                p = path.display(),
                e = e,
            )
        })?;
        let language = raw
            .general
            .language
            .as_deref()
            .and_then(Language::from_str)
            .unwrap_or_default();
        Ok(LintConfig {
            include: raw.rules.include.into_iter().collect(),
            exclude: raw.rules.exclude.into_iter().collect(),
            language,
            fmt_reorder: raw.fmt.reorder.unwrap_or(true),
        })
    }

    /// この`code`の診断を出力すべきか。
    /// `include`が空なら常に候補入り、非空なら`include`に含まれる場合のみ候補入り。
    /// その後`exclude`に含まれていればどちらの場合も無効化する。
    pub fn is_enabled(&self, code: &str) -> bool {
        let included = self.include.is_empty() || self.include.contains(code);
        included && !self.exclude.contains(code)
    }

    /// 出力言語。設定ファイル無し・`[general] language`未指定の場合は
    /// `Language::default()`（English）。
    pub fn language(&self) -> Language {
        self.language
    }

    /// `fmt`のデフォルト並び替え挙動。設定ファイル無し・`[fmt] reorder`未指定の
    /// 場合は`true`（デフォルトでreorderする）。CLIの`--no-reorder`はこの値を
    /// 呼び出し元（`main.rs::run_fmt`）で強制的に上書きする（この関数自体は
    /// CLI引数を関知しない）。
    pub fn fmt_reorder(&self) -> bool {
        self.fmt_reorder
    }
}

/// `path`（通常はカレントディレクトリの`dat_linter.toml`）にコメント付きの
/// デフォルト設定ファイルを新規作成する。既にファイルが存在する場合は
/// 上書きしない（呼び出し元は「存在しない」ことを確認済みの前提だが、
/// 競合（TOCTOU）による意図しない上書きを避けるため`create_new`で書き込む）。
fn generate_default_config_file(path: &Path) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(DEFAULT_CONFIG_TEMPLATE.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_enabled_allows_everything() {
        let cfg = LintConfig::all_enabled();
        assert!(cfg.is_enabled("obsolete-type"));
        assert!(cfg.is_enabled("anything"));
    }

    #[test]
    fn all_enabled_defaults_to_english() {
        assert_eq!(LintConfig::all_enabled().language(), Language::English);
    }

    #[test]
    fn empty_include_means_all_enabled_by_default() {
        let raw = RawConfig::default();
        let cfg = LintConfig {
            include: raw.rules.include.into_iter().collect(),
            exclude: raw.rules.exclude.into_iter().collect(),
            language: Language::default(),
            fmt_reorder: true,
        };
        assert!(cfg.is_enabled("obsolete-type"));
    }

    #[test]
    fn non_empty_include_restricts_to_listed_codes() {
        let cfg = LintConfig {
            include: ["obsolete-type".to_string()].into_iter().collect(),
            exclude: HashSet::new(),
            language: Language::default(),
            fmt_reorder: true,
        };
        assert!(cfg.is_enabled("obsolete-type"));
        assert!(!cfg.is_enabled("missing-waytype"));
    }

    #[test]
    fn exclude_wins_even_if_also_included() {
        let cfg = LintConfig {
            include: ["obsolete-type".to_string()].into_iter().collect(),
            exclude: ["obsolete-type".to_string()].into_iter().collect(),
            language: Language::default(),
            fmt_reorder: true,
        };
        assert!(!cfg.is_enabled("obsolete-type"));
    }

    #[test]
    fn exclude_only_removes_from_default_all_enabled_set() {
        let cfg = LintConfig {
            include: HashSet::new(),
            exclude: ["duplicate-key".to_string()].into_iter().collect(),
            language: Language::default(),
            fmt_reorder: true,
        };
        assert!(!cfg.is_enabled("duplicate-key"));
        assert!(cfg.is_enabled("obsolete-type"));
    }

    #[test]
    fn parses_toml_text() {
        let raw: RawConfig = toml::from_str(
            r#"
            [rules]
            include = ["obsolete-type", "missing-waytype"]
            exclude = ["missing-waytype"]
            "#,
        )
        .unwrap();
        let cfg = LintConfig {
            include: raw.rules.include.into_iter().collect(),
            exclude: raw.rules.exclude.into_iter().collect(),
            language: Language::default(),
            fmt_reorder: true,
        };
        assert!(cfg.is_enabled("obsolete-type"));
        assert!(!cfg.is_enabled("missing-waytype"));
        assert!(!cfg.is_enabled("unrelated-code"));
    }

    #[test]
    fn parses_language_ja() {
        let raw: RawConfig = toml::from_str(
            r#"
            [general]
            language = "ja"
            "#,
        )
        .unwrap();
        assert_eq!(raw.general.language.as_deref(), Some("ja"));
    }

    #[test]
    fn missing_language_key_defaults_to_english_via_load_from() {
        let tmp = std::env::temp_dir().join("dat_linter_test_no_language_key.toml");
        std::fs::write(&tmp, "[rules]\ninclude = []\n").unwrap();
        let cfg = LintConfig::load_from(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(cfg.language(), Language::English);
    }

    #[test]
    fn explicit_ja_language_is_honored_via_load_from() {
        let tmp = std::env::temp_dir().join("dat_linter_test_ja_language.toml");
        std::fs::write(&tmp, "[general]\nlanguage = \"ja\"\n").unwrap();
        let cfg = LintConfig::load_from(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(cfg.language(), Language::Japanese);
    }

    #[test]
    fn unknown_language_value_falls_back_to_english() {
        let tmp = std::env::temp_dir().join("dat_linter_test_unknown_language.toml");
        std::fs::write(&tmp, "[general]\nlanguage = \"fr\"\n").unwrap();
        let cfg = LintConfig::load_from(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(cfg.language(), Language::English);
    }

    #[test]
    fn all_enabled_defaults_fmt_reorder_to_true() {
        assert!(LintConfig::all_enabled().fmt_reorder());
    }

    #[test]
    fn missing_fmt_section_defaults_reorder_to_true_via_load_from() {
        let tmp = std::env::temp_dir().join("dat_linter_test_no_fmt_section.toml");
        std::fs::write(&tmp, "[rules]\ninclude = []\n").unwrap();
        let cfg = LintConfig::load_from(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(cfg.fmt_reorder());
    }

    #[test]
    fn explicit_reorder_false_is_honored_via_load_from() {
        let tmp = std::env::temp_dir().join("dat_linter_test_reorder_false.toml");
        std::fs::write(&tmp, "[fmt]\nreorder = false\n").unwrap();
        let cfg = LintConfig::load_from(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(!cfg.fmt_reorder());
    }

    #[test]
    fn explicit_reorder_true_is_honored_via_load_from() {
        let tmp = std::env::temp_dir().join("dat_linter_test_reorder_true.toml");
        std::fs::write(&tmp, "[fmt]\nreorder = true\n").unwrap();
        let cfg = LintConfig::load_from(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(cfg.fmt_reorder());
    }

    #[test]
    fn generate_default_config_file_includes_fmt_reorder_true() {
        let tmp = std::env::temp_dir().join("dat_linter_test_generated_config_fmt.toml");
        let _ = std::fs::remove_file(&tmp);
        generate_default_config_file(&tmp).expect("生成に失敗");
        let content = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(content.contains("[fmt]"));
        assert!(content.contains("reorder"));
        let raw: RawConfig = toml::from_str(&content).expect("生成された設定のパースに失敗");
        assert_eq!(raw.fmt.reorder, Some(true));
    }

    /// 設定ファイル自体が存在しない場合の読み込みエラーメッセージは、
    /// `load_from`のdocコメントに記載の通り常に`Language::default()`（English）
    /// 固定である（設定ファイルの中身が読めていない時点でユーザーの言語選択を
    /// 知りようがないため）。この方針が実際にコード側でも守られていることを固定する。
    #[test]
    fn unreadable_file_error_message_is_always_english() {
        let tmp = std::env::temp_dir().join("dat_linter_test_nonexistent_file_for_error.toml");
        let _ = std::fs::remove_file(&tmp);
        let err = LintConfig::load_from(&tmp).unwrap_err();
        assert!(
            err.contains("Cannot read"),
            "存在しないファイルのエラーは常に英語であるべき: {err:?}"
        );
        assert!(
            !err.contains("読み込めません"),
            "存在しないファイルのエラーに日本語文字列が混入している: {err:?}"
        );
    }

    /// 不正なTOML構文のパースエラーメッセージも同様に常にEnglish固定。
    #[test]
    fn invalid_toml_error_message_is_always_english() {
        let tmp = std::env::temp_dir().join("dat_linter_test_invalid_toml.toml");
        std::fs::write(&tmp, "this is not valid toml [[[").unwrap();
        let err = LintConfig::load_from(&tmp).unwrap_err();
        let _ = std::fs::remove_file(&tmp);
        assert!(
            err.contains("Failed to parse TOML"),
            "不正なTOMLのエラーは常に英語であるべき: {err:?}"
        );
        assert!(
            !err.contains("TOML解析に失敗しました"),
            "不正なTOMLのエラーに日本語文字列が混入している: {err:?}"
        );
    }

    #[test]
    fn generate_default_config_file_writes_template_with_both_sections() {
        let tmp = std::env::temp_dir().join("dat_linter_test_generated_config.toml");
        let _ = std::fs::remove_file(&tmp);
        generate_default_config_file(&tmp).expect("生成に失敗");
        let content = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(content.contains("[general]"));
        assert!(content.contains("language"));
        assert!(content.contains("[rules]"));
        // 生成された内容自体が正しくパースできることも確認する。
        let raw: RawConfig = toml::from_str(&content).expect("生成された設定のパースに失敗");
        assert_eq!(raw.general.language.as_deref(), Some("en"));
    }

    #[test]
    fn generate_default_config_file_does_not_overwrite_existing_file() {
        let tmp = std::env::temp_dir().join("dat_linter_test_existing_config.toml");
        std::fs::write(&tmp, "# custom content\n[rules]\ninclude=[\"x\"]\n").unwrap();
        let result = generate_default_config_file(&tmp);
        let content_after = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(result.is_err(), "既存ファイルへの上書きはエラーになるべき");
        assert!(content_after.contains("custom content"));
    }
}
