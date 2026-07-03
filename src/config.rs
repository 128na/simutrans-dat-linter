//! `lint`サブコマンドの診断ルールinclude/exclude設定（TOML）。
//!
//! ## 配置場所・探索順
//! - `--config <path>` が明示された場合はそのパスのみを読む（存在しなければエラー）。
//! - 明示が無い場合、カレントディレクトリ直下の `dat_linter.toml` を自動探索する。
//!   存在しなければ「設定ファイル無し」として全ルールを有効にする（エラーにしない。
//!   このツールを設定ファイル無しでこれまで通り使い続けられることを優先する）。
//!
//! ## スキーマ
//! ```toml
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

use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 自動探索時に見るファイル名。
const DEFAULT_CONFIG_FILENAME: &str = "dat_linter.toml";

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    rules: RawRulesConfig,
}

#[derive(Debug, Default, Deserialize)]
struct RawRulesConfig {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

/// 読み込み・正規化済みの設定。`is_enabled`で`Diagnostic.code`ごとに
/// 有効/無効を判定する。
#[derive(Debug, Default)]
pub struct LintConfig {
    include: HashSet<String>,
    exclude: HashSet<String>,
}

impl LintConfig {
    /// 全ルールが有効な設定（設定ファイル無しの状態と同義）。
    pub fn all_enabled() -> Self {
        LintConfig {
            include: HashSet::new(),
            exclude: HashSet::new(),
        }
    }

    /// `--config`指定または自動探索の結果に応じて設定を読み込む。
    ///
    /// - `explicit_path`が`Some`: そのパスを読む。存在しない・パースエラーは`Err`。
    /// - `explicit_path`が`None`: カレントディレクトリの`dat_linter.toml`を探す。
    ///   存在しなければ`Ok(LintConfig::all_enabled())`（エラーにしない）。
    pub fn load_or_default(explicit_path: Option<&Path>) -> Result<Self, String> {
        match explicit_path {
            Some(path) => Self::load_from(path),
            None => {
                let default_path = PathBuf::from(DEFAULT_CONFIG_FILENAME);
                if default_path.is_file() {
                    Self::load_from(&default_path)
                } else {
                    Ok(Self::all_enabled())
                }
            }
        }
    }

    fn load_from(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} を読み込めません ({e})", path.display()))?;
        let raw: RawConfig = toml::from_str(&text)
            .map_err(|e| format!("{} のTOML解析に失敗しました ({e})", path.display()))?;
        Ok(LintConfig {
            include: raw.rules.include.into_iter().collect(),
            exclude: raw.rules.exclude.into_iter().collect(),
        })
    }

    /// この`code`の診断を出力すべきか。
    /// `include`が空なら常に候補入り、非空なら`include`に含まれる場合のみ候補入り。
    /// その後`exclude`に含まれていればどちらの場合も無効化する。
    pub fn is_enabled(&self, code: &str) -> bool {
        let included = self.include.is_empty() || self.include.contains(code);
        included && !self.exclude.contains(code)
    }
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
    fn empty_include_means_all_enabled_by_default() {
        let raw = RawConfig::default();
        let cfg = LintConfig {
            include: raw.rules.include.into_iter().collect(),
            exclude: raw.rules.exclude.into_iter().collect(),
        };
        assert!(cfg.is_enabled("obsolete-type"));
    }

    #[test]
    fn non_empty_include_restricts_to_listed_codes() {
        let cfg = LintConfig {
            include: ["obsolete-type".to_string()].into_iter().collect(),
            exclude: HashSet::new(),
        };
        assert!(cfg.is_enabled("obsolete-type"));
        assert!(!cfg.is_enabled("missing-waytype"));
    }

    #[test]
    fn exclude_wins_even_if_also_included() {
        let cfg = LintConfig {
            include: ["obsolete-type".to_string()].into_iter().collect(),
            exclude: ["obsolete-type".to_string()].into_iter().collect(),
        };
        assert!(!cfg.is_enabled("obsolete-type"));
    }

    #[test]
    fn exclude_only_removes_from_default_all_enabled_set() {
        let cfg = LintConfig {
            include: HashSet::new(),
            exclude: ["duplicate-key".to_string()].into_iter().collect(),
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
        };
        assert!(cfg.is_enabled("obsolete-type"));
        assert!(!cfg.is_enabled("missing-waytype"));
        assert!(!cfg.is_enabled("unrelated-code"));
    }
}
