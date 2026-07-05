use crate::codes::DiagnosticCode;
use std::fmt;

/// 宣言順 = 重大度の高い順。Ord導出により `severity <= level` で
/// 「指定levelでも表示すべきか」を判定できる（Error=0が常に表示される）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// pak化に失敗する、またはゲーム内で正常に表示されない
    Error,
    /// 非推奨な項目、動作はするが設定が推奨される項目
    Warning,
    /// 正常な項目の簡易な出力
    Info,
    /// 詳細な監査ログ
    Debug,
}

impl Severity {
    /// CLIの -v 指定回数からこのlevelへの変換
    pub fn from_verbosity(count: u8) -> Self {
        match count {
            0 => Severity::Warning,
            1 => Severity::Info,
            _ => Severity::Debug,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warn"),
            Severity::Info => write!(f, "info"),
            Severity::Debug => write!(f, "debug"),
        }
    }
}

/// 診断が指す`.dat`内の位置。全ての診断がこれを持つわけではない
/// （Dimsサイズ0のようなファイル全体・複数キー由来の診断には自然な単一行が無い）。
#[derive(Debug)]
pub struct Location {
    pub line: usize,
    pub key: String,
}

/// 第9弾（項目3）で`formatter::ParsedDat::warnings`/`format_reordered`の
/// 戻り値型を`Vec<String>`からこの`Diagnostic`へ統一した（`couplings.rs`は
/// 元々この型を使っていた）。これにより`fmt`/`analyze`もlintと同じ
/// `LintConfig::is_enabled(code)`フィルタを適用でき、`code`一覧表示
/// （`dat_linter list`）にも同じデータ型で対応できる。`#[derive(Debug)]`は
/// テストの`assert!(..., "{warnings:?}")`のようなデバッグ出力のために必要。
///
/// 第17弾（code smellレビュー・タスク13）: `code`フィールドの型を
/// `&'static str`から`codes::DiagnosticCode`（enum）へ変更した。以前は
/// `Diagnostic::error("missing-waytype", ...)`のように任意の文字列を渡せて
/// しまい、`src/codes.rs::ALL_CODES`との整合性は実行時テスト
/// （`tests/codes_completeness.rs`の正規表現スキャン）でしか保証できなかった。
/// enum化により、存在しないcode文字列を指定してしまうミス自体が
/// コンパイルエラーになる。文字列表現が必要な箇所（`dat_linter.toml`の
/// include/exclude・`describe`引数・診断メッセージの表示）は
/// `DiagnosticCode::as_str()`で取得する。
#[derive(Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    pub location: Option<Location>,
}

impl Diagnostic {
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            location: None,
        }
    }

    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            code,
            message: message.into(),
            location: None,
        }
    }

    pub fn info(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Info,
            code,
            message: message.into(),
            location: None,
        }
    }

    pub fn debug(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Debug,
            code,
            message: message.into(),
            location: None,
        }
    }

    /// この診断が指す行・キーを付与する（builder）。
    pub fn at(mut self, line: usize, key: impl Into<String>) -> Self {
        self.location = Some(Location {
            line,
            key: key.into(),
        });
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code.as_str();
        match &self.location {
            Some(loc) => write!(
                f,
                "[{}] {} (line {}): {}",
                self.severity, code, loc.line, self.message
            ),
            None => write!(f, "[{}] {}: {}", self.severity, code, self.message),
        }
    }
}
