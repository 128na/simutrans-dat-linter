//! `vehicle` モジュール（連結制約解析）の統合テスト。
//! testdata の ok / broken / dangling の3ディレクトリで挙動を確認する。

use dat_linter::couplings as vehicle;
use dat_linter::diagnostics::Severity;
use dat_linter::i18n::Language;
use std::path::{Path, PathBuf};

fn testdata(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(sub)
}

/// ディレクトリを読み込み、load/dangling/satisfiability の全診断コードを返す。
fn analyze(sub: &str) -> Vec<(Severity, &'static str)> {
    let dir = testdata(sub);
    let (vehicles, mut diags) = vehicle::load_vehicles(&dir, Language::default());
    diags.extend(vehicle::check_dangling_refs(&vehicles, Language::default()));
    diags.extend(vehicle::check_satisfiability(
        &vehicles,
        Language::default(),
    ));
    diags
        .into_iter()
        .map(|d| (d.severity, d.code.as_str()))
        .collect()
}

fn has_error(diags: &[(Severity, &str)], code: &str) -> bool {
    diags
        .iter()
        .any(|(s, c)| *s == Severity::Error && *c == code)
}

#[test]
fn ok_directory_has_no_errors() {
    let diags = analyze("couplings_ok");
    let errors: Vec<_> = diags
        .iter()
        .filter(|(s, _)| *s == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "予期しない error: {errors:?}");
}

#[test]
fn self_referencing_loop_is_unsatisfiable() {
    // A は自身を prev/next 両方に要求し続けるため、有限な編成が組めない。
    assert!(has_error(
        &analyze("couplings_broken"),
        "unsatisfiable-constraint"
    ));
}

#[test]
fn dangling_reference_is_detected() {
    // E は存在しない車両 "Ghost" を next に参照する。
    assert!(has_error(
        &analyze("couplings_dangling"),
        "dangling-vehicle-constraint"
    ));
}
