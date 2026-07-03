//! Simutrans `.dat` の静的検証・フォーマット・連結制約解析を行うライブラリ。
//!
//! CLI 入口は `src/main.rs`。各モジュールを公開し、統合テスト（`tests/`）や
//! 外部クレートからルール・フォーマッタ・連結制約解析を直接呼べるようにしている。

pub mod config;
pub mod couplings;
pub mod diagnostics;
pub mod formatter;
pub mod i18n;
pub mod param_expansion;
pub mod parser;
pub mod registry;
pub mod rules;
