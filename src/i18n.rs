//! 診断メッセージ・CLIの短いヘルプ文言の日本語/英語切り替え。
//!
//! ## 設計
//! `Diagnostic::error/warning/info/debug`は`message: impl Into<String>`を
//! 受け取るだけの薄いbuilderで、`code`（診断の一意なID）自体は翻訳対象ではない
//! （`config.rs`のinclude/excludeや各テストは`code`文字列でマッチしており、
//! この文字列は変更しない）。翻訳するのは`message`本文のみ。
//!
//! 各`Rule::check`実装は`ctx.language`（`RuleContext`経由）を見て、
//! [`t!`]マクロでJA/EN両方のフォーマット文字列を1箇所に並べて書く。
//! `format!`の名前付きキャプチャ（`{type_name}`等）はJA/EN共通のスコープの
//! 変数を参照するため、多くの呼び出し箇所は「文字列リテラル部分だけ」を
//! 訳せばよい（引数の順序・型を揃える必要が無い）。
//!
//! ```ignore
//! use crate::i18n::{t, Language};
//! let msg = t!(lang,
//!     ja: "type={type_name} は obsolete です",
//!     en: "type={type_name} is obsolete",
//! );
//! ```

/// 出力言語。`Copy`にしているのは`RuleContext`をイミュータブルに使い回す
/// 既存の設計（`Rule::check(&self, ctx: &RuleContext)`）と相性が良いため
/// （`Language`は1バイトのenumで、参照を持ち回すよりコピーの方が単純）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// デフォルト。設定ファイルが無い場合・`[general] language`未指定の場合はこちら
    /// （ユーザー確認済みの決定事項。既存の日本語固定挙動から変更されている点に注意）。
    #[default]
    English,
    Japanese,
}

impl Language {
    /// config.rsの`[general] language`文字列からの変換。
    /// 未知の値・空文字列は`None`（呼び出し側でデフォルト`English`にフォールバックする）。
    ///
    /// 名前は`std::str::FromStr`トレイトと紛らわしいが、`registry::ObjType::from_str`と
    /// 同じ理由でこのシグネチャ（inherent method、`&str` -> `Option<Language>`）を
    /// 踏襲するため`clippy::should_implement_trait`を明示的に許容する。
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Language> {
        match s.to_ascii_lowercase().as_str() {
            "en" | "english" => Some(Language::English),
            "ja" | "japanese" | "jp" => Some(Language::Japanese),
            _ => None,
        }
    }

    pub fn as_config_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Japanese => "ja",
        }
    }
}

/// JA/EN2つのフォーマット文字列を1箇所に書き、`lang`に応じて`format!`する
/// マクロ。呼び出し箇所は`t!(lang, ja: "...", en: "...")`
/// （引数を補間する場合は`t!(lang, ja: "{x}...", en: "...{x}", x = expr)`のように
/// `format!`と同じ名前付き引数を末尾に追加できる）。
///
/// 実体は単なる`match`+`format!`の展開であり、実行時コストは通常の
/// `format!`呼び出しと同じ（言語ごとの文字列テーブルや`HashMap`引きは行わない）。
///
/// `#[macro_export]`によりクレートルート（`dat_linter::t`）に配置される
/// （`main.rs`は`dat_linter`を外部クレートとして参照するバイナリのため、
/// `pub use`による通常のモジュール内re-exportでは`main.rs`から見えない。
/// `macro_export`はマクロに限った特別な公開規則で、モジュールパスに関係なく
/// クレートルート直下に置かれる）。
#[macro_export]
macro_rules! t {
    ($lang:expr, ja: $ja:expr, en: $en:expr $(,)?) => {
        match $lang {
            $crate::i18n::Language::Japanese => format!($ja),
            $crate::i18n::Language::English => format!($en),
        }
    };
    ($lang:expr, ja: $ja:expr, en: $en:expr, $($rest:tt)*) => {
        match $lang {
            $crate::i18n::Language::Japanese => format!($ja, $($rest)*),
            $crate::i18n::Language::English => format!($en, $($rest)*),
        }
    };
}

// `#[macro_export]`はマクロをクレートルートに公開するが、同一クレート内
// （このi18nモジュール自身やrules/*.rs）から`crate::i18n::t`という通常の
// モジュールパスでも使えるよう、ルートに公開されたマクロをこのモジュールへ
// re-exportしておく。
pub use crate::t;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_recognizes_en_and_ja() {
        assert_eq!(Language::from_str("en"), Some(Language::English));
        assert_eq!(Language::from_str("EN"), Some(Language::English));
        assert_eq!(Language::from_str("english"), Some(Language::English));
        assert_eq!(Language::from_str("ja"), Some(Language::Japanese));
        assert_eq!(Language::from_str("JP"), Some(Language::Japanese));
        assert_eq!(Language::from_str("japanese"), Some(Language::Japanese));
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert_eq!(Language::from_str(""), None);
        assert_eq!(Language::from_str("fr"), None);
    }

    #[test]
    fn default_is_english() {
        assert_eq!(Language::default(), Language::English);
    }

    #[test]
    fn t_macro_selects_by_language_without_args() {
        let ja = t!(Language::Japanese, ja: "こんにちは", en: "hello");
        let en = t!(Language::English, ja: "こんにちは", en: "hello");
        assert_eq!(ja, "こんにちは");
        assert_eq!(en, "hello");
    }

    #[test]
    fn t_macro_selects_by_language_with_named_args() {
        let x = 42;
        let ja = t!(Language::Japanese, ja: "値は{x}です", en: "value is {x}", x = x);
        let en = t!(Language::English, ja: "値は{x}です", en: "value is {x}", x = x);
        assert_eq!(ja, "値は42です");
        assert_eq!(en, "value is 42");
    }
}
