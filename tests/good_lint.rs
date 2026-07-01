//! `rules::check_good` の統合テスト。testdata/ の正常系1件で、
//! 期待する診断が出ない（＝空のRuleセット）ことを確認する。
//! `tests/way_lint.rs`と同じ形式に揃えている。
//!
//! `good_writer.cc`にはmakeobj時点でfatal/warningになる分岐が一つも無いため
//! （`src/rules/good.rs`冒頭のREJECTEDコメント参照）、異常系テストは無い。
//! `check_good`自体は`all()`が空のRuleSetを返すラッパーであることの確認と、
//! 将来ルールが追加された際にこのファイル構成をそのまま再利用できることが目的。

use dat_linter::diagnostics::Severity;
use dat_linter::parser::DatFile;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// 指定した good dat を検査し、(severity, code) の一覧を返す。
fn check(file: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata_dir();
    let path = dir.join(file);
    let dat = DatFile::parse(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    rules::check_good(&dat, &dir)
        .into_iter()
        .map(|d| (d.severity, d.code))
        .collect()
}

#[test]
fn valid_good_has_no_errors_or_warnings() {
    let diags = check("good_valid.dat");
    assert!(diags.is_empty(), "予期しない診断: {diags:?}");
}
