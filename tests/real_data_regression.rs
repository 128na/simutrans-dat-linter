//! `refs/linter_test`（このリポジトリ外、`simutrans_addon`ワークスペースの参照アドオン
//! 置き場）にある実データを`testdata/real_data/`へコピーした回帰テスト。
//!
//! 対象は3件（メインセッションでの事前調査結果を参照）:
//! - `JpClassicTerminal.dat`: `type=station`/`extension_building=1`という**本物の
//!   obsolete**構文（`building_writer.cc:185-204`の`dbg->fatal`で裏付け済み）。
//!   誤検知ではなく正しい判定なので、期待する診断は「エラーが出ること」。
//! - `tunnelz.dat`: 4つの`obj=tunnel`が`-`区切りで連結された、エラーの出ない
//!   正常系の実データ。`parse_all`の複数レコード処理の回帰も兼ねる。
//! - `KSN-128op_Rail-yard_0001.dat` / `KSN-128op-OTRP_Rail-yard_0001.dat`:
//!   `BackImage[0,1,2,3,4,5,6,7][...]=...<$0>`のようなパラメータ展開構文
//!   （`param_expansion`モジュールが実装したもの）を実際に使うファイル。
//!   展開が無ければ`missing-tile-image`を誤検知していたことをこのテストで固定する。
//!
//! ## フィクスチャの扱い（重要）
//! `testdata/real_data/` はサードパーティ製の実アドオンデータ
//! （`refs/linter_test/`からのローカルコピー）のため、`.gitignore`で意図的に
//! git管理対象外にしている。つまりCI・他の開発者のクローンにはこのディレクトリが
//! **存在しない**のが前提。各テストは冒頭で`skip_if_fixtures_missing!()`を呼び、
//! フィクスチャが無い場合は理由を`eprintln!`してテストをスキップ（早期return）する
//! ことで、フィクスチャの有無に関わらず`cargo test`全体がFAILにならないようにする。
//! フィクスチャがこのマシン上に存在する間は、これまで通り実データに対する
//! 実際の検証が走る。

use dat_linter::codes::DiagnosticCode;
use dat_linter::config::LintConfig;
use dat_linter::diagnostics::Severity;
use dat_linter::i18n::Language;
use dat_linter::parser::DatFile;
use dat_linter::registry::RuleSet;
use dat_linter::rules;
use std::path::{Path, PathBuf};

fn real_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/real_data")
}

/// `testdata/real_data/`（および指定した`file`）が存在しない場合は`true`を返す。
/// このディレクトリはサードパーティ実データのため`.gitignore`対象で、
/// CI・他クローンには存在しないのが前提。
fn fixture_missing(file: &str) -> bool {
    !real_data_dir().join(file).is_file()
}

/// テスト冒頭で呼ぶスキップガード。フィクスチャファイルが見つからない場合は
/// 理由を`eprintln!`した上でこのテスト関数を早期returnさせる（FAILにしない）。
macro_rules! skip_if_fixture_missing {
    ($file:expr) => {
        if fixture_missing($file) {
            eprintln!(
                "SKIP: testdata/real_data/{} が見つからないためこのテストをスキップします \
                 （このディレクトリはサードパーティ実データのため.gitignore対象で、\
                 ローカル検証専用です。取得したい場合は refs/linter_test/ からコピーしてください）",
                $file
            );
            return;
        }
    };
}

/// 指定ファイルの**全レコード**を検査し、レコードごとの`(severity, code)`一覧を返す。
/// `main.rs::lint_one_file_counts`と同じロジック
/// （`check_duplicate_keys` + `RuleSet::for_obj_type`）をテスト用に再現している。
fn check_all_records(file: &str) -> Vec<Vec<(Severity, &'static str)>> {
    let dir = real_data_dir();
    let path = dir.join(file);
    let records =
        DatFile::parse_all(&path).unwrap_or_else(|e| panic!("{file} のパースに失敗: {e}"));
    records
        .iter()
        .map(|dat| {
            let obj_type = dat.get("obj").unwrap_or("").to_string();
            let mut diags = rules::check_duplicate_keys(dat, Language::default());
            if let Some(rule_set) = RuleSet::for_obj_type(&obj_type, dat) {
                let ctx = dat_linter::registry::RuleContext {
                    dat,
                    dat_dir: &dir,
                    language: Language::default(),
                };
                diags.extend(rule_set.run(&ctx));
            }
            diags
                .into_iter()
                .map(|d| (d.severity, d.code.as_str()))
                .collect()
        })
        .collect()
}

fn count_errors(records: &[Vec<(Severity, &str)>]) -> usize {
    records
        .iter()
        .flatten()
        .filter(|(s, _)| *s == Severity::Error)
        .count()
}

// --- JpClassicTerminal.dat: 本物のobsolete構文（誤検知ではない） -----------------

#[test]
fn jp_classic_terminal_has_three_records() {
    skip_if_fixture_missing!("JpClassicTerminal.dat");
    let dir = real_data_dir();
    let records = DatFile::parse_all(&dir.join("JpClassicTerminal.dat")).expect("パースに失敗");
    assert_eq!(records.len(), 3, "`-`区切りの3ステージが解決できていない");
}

#[test]
fn jp_classic_terminal_obsolete_type_is_correctly_flagged() {
    skip_if_fixture_missing!("JpClassicTerminal.dat");
    // 調査済み: type=station は building_writer.cc:185-204 の dbg->fatal で
    // 裏付けられた本物のobsoleteであり、誤検知ではない。全3レコードで検出されるべき。
    let records = check_all_records("JpClassicTerminal.dat");
    assert_eq!(records.len(), 3);
    for (i, diags) in records.iter().enumerate() {
        assert!(
            diags
                .iter()
                .any(|(s, c)| *s == Severity::Error && *c == "obsolete-type"),
            "レコード{i}でobsolete-typeが検出されるべき: {diags:?}"
        );
    }
}

#[test]
fn jp_classic_terminal_obsolete_keyword_is_correctly_flagged() {
    skip_if_fixture_missing!("JpClassicTerminal.dat");
    // extension_building=1 も同様にobsolete（obsolete-keyword）。
    let records = check_all_records("JpClassicTerminal.dat");
    for (i, diags) in records.iter().enumerate() {
        assert!(
            diags
                .iter()
                .any(|(s, c)| *s == Severity::Error && *c == "obsolete-keyword"),
            "レコード{i}でobsolete-keywordが検出されるべき: {diags:?}"
        );
    }
}

// --- tunnelz.dat: 正常系・複数obj=tunnel連結 ------------------------------------

#[test]
fn tunnelz_has_four_records() {
    skip_if_fixture_missing!("tunnelz.dat");
    let dir = real_data_dir();
    let records = DatFile::parse_all(&dir.join("tunnelz.dat")).expect("パースに失敗");
    assert_eq!(
        records.len(),
        4,
        "BGU/EPU/EFU/SSUの4つのtunnel定義が解決できていない"
    );
}

#[test]
fn tunnelz_has_no_errors_or_warnings() {
    skip_if_fixture_missing!("tunnelz.dat");
    let records = check_all_records("tunnelz.dat");
    assert_eq!(records.len(), 4);
    for (i, diags) in records.iter().enumerate() {
        let errors: Vec<_> = diags
            .iter()
            .filter(|(s, _)| *s == Severity::Error)
            .collect();
        let warnings: Vec<_> = diags
            .iter()
            .filter(|(s, _)| *s == Severity::Warning)
            .collect();
        assert!(
            errors.is_empty(),
            "レコード{i}で予期しないerror: {errors:?}"
        );
        assert!(
            warnings.is_empty(),
            "レコード{i}で予期しないwarning: {warnings:?}"
        );
    }
}

// --- KSN-128op_Rail-yard_0001.dat: パラメータ展開構文（機能Bの回帰） -------------

#[test]
fn ksn_rail_yard_has_three_records() {
    skip_if_fixture_missing!("KSN-128op_Rail-yard_0001.dat");
    let dir = real_data_dir();
    let records =
        DatFile::parse_all(&dir.join("KSN-128op_Rail-yard_0001.dat")).expect("パースに失敗");
    assert_eq!(
        records.len(),
        3,
        "0001/0002/0003の3ステージが解決できていない"
    );
}

#[test]
fn ksn_rail_yard_parameter_expansion_resolves_first_16_layouts() {
    skip_if_fixture_missing!("KSN-128op_Rail-yard_0001.dat");
    // このファイルのlayout 0-15 (BackImage[0,1,...,7]と[8,...,15]の2行がそれぞれ
    // 8個ずつに展開される)は実在する画像を指しており、パラメータ展開が正しく
    // 動作していればmissing-tile-imageは出ない。layout 16-47は`.dat`内で
    // `#`コメントアウトされている（実際に画像が無い）ため、そちらは
    // missing-tile-imageが出て正しい（本テストでは0-15のみを見る）。
    let dir = real_data_dir();
    let records =
        DatFile::parse_all(&dir.join("KSN-128op_Rail-yard_0001.dat")).expect("パースに失敗");
    for (i, dat) in records.iter().enumerate() {
        for layout in 0..16 {
            let back = dat.get(&format!("backimage[{layout}][0][0][0][0][0]"));
            let front = dat.get(&format!("frontimage[{layout}][0][0][0][0][0]"));
            assert!(
                back.is_some() && front.is_some(),
                "レコード{i} layout{layout}: パラメータ展開後のキーが見つからない \
                 (backimage={back:?}, frontimage={front:?})"
            );
        }
    }
}

#[test]
fn ksn_rail_yard_commented_out_layouts_16_to_47_are_missing_tile_image() {
    skip_if_fixture_missing!("KSN-128op_Rail-yard_0001.dat");
    // layout16-47は`.dat`側で`#`コメントアウトされている（意図的に未定義）ため、
    // missing-tile-imageが出るのが正しい挙動。パラメータ展開の実装が「常に全部
    // 埋める」ような過剰展開になっていないことの確認を兼ねる。
    let records = check_all_records("KSN-128op_Rail-yard_0001.dat");
    assert_eq!(records.len(), 3);
    // 3レコード x 32layout分(16..48) = 96件のmissing-tile-imageが期待値
    // (実装当時に実測した値。パラメータ展開が退行して0-15まで巻き込まれると
    // この数値が144に戻ってしまう＝退行検知になる)。
    let total_missing_tile = records
        .iter()
        .flatten()
        .filter(|(s, c)| *s == Severity::Error && *c == "missing-tile-image")
        .count();
    assert_eq!(
        total_missing_tile, 96,
        "missing-tile-imageの件数が想定と異なる(パラメータ展開の退行の可能性): {total_missing_tile}"
    );
}

#[test]
fn ksn_rail_yard_otrp_variant_has_no_errors() {
    skip_if_fixture_missing!("KSN-128op-OTRP_Rail-yard_0001.dat");
    // OTRPバリアントはlayout0-31が全てコメントアウトを外され、実在する画像
    // (0001/0002/0003.png)を参照している。パラメータ展開が正しく動作していれば
    // エラー0件になるはず。
    let records = check_all_records("KSN-128op-OTRP_Rail-yard_0001.dat");
    assert_eq!(records.len(), 3);
    let total_errors = count_errors(&records);
    assert_eq!(
        total_errors, 0,
        "OTRPバリアントはエラー0件のはずが{total_errors}件検出された: {records:?}"
    );
}

#[test]
fn ksn_rail_yard_dollar_n_arithmetic_reference_resolves_correct_tile_suffix() {
    skip_if_fixture_missing!("KSN-128op_Rail-yard_0001.dat");
    // BackImage[8,9,...,15][...]=KSN-128op_Rail-yard_0001.     2.<$-8>
    // という行は<$-8>が「$0(=layoutの実値, 8..15) - 8」に評価されるため、
    // layout8は".2.0"、layout15は".2.7"を指すはず（パーサーの算術展開が
    // 正しく動作しているかの直接的な確認）。
    let dir = real_data_dir();
    let records =
        DatFile::parse_all(&dir.join("KSN-128op_Rail-yard_0001.dat")).expect("パースに失敗");
    let dat = &records[0];

    // 元の値は`"KSN-128op_Rail-yard_0001.     2.<$-8>"`のように桁揃えの空白を
    // 含むため、末尾の空白除去後に評価結果のサフィックスだけを比較する。
    let back8 = dat
        .get("backimage[8][0][0][0][0][0]")
        .expect("backimage[8]が無い");
    assert!(
        back8.trim().ends_with("2.0"),
        "layout8の展開結果がサフィックス2.0で終わっていない: {back8:?}"
    );

    let back15 = dat
        .get("backimage[15][0][0][0][0][0]")
        .expect("backimage[15]が無い");
    assert!(
        back15.trim().ends_with("2.7"),
        "layout15の展開結果がサフィックス2.7で終わっていない: {back15:?}"
    );
}

// --- 設定ファイルによるinclude/exclude(機能C)がKSNの誤検知抑制ユースケースで
//     期待通り動くことの確認 ------------------------------------------------

#[test]
fn config_exclude_suppresses_missing_tile_image_diagnostics() {
    skip_if_fixture_missing!("KSN-128op_Rail-yard_0001.dat");
    let records = check_all_records("KSN-128op_Rail-yard_0001.dat");
    let config = {
        // LintConfig はファイル経由のロードのみ公開しているため、excludeのみの
        // TOMLを直接パースして同等の設定を作る。
        let toml_text = "[rules]\nexclude = [\"missing-tile-image\"]\n";
        let tmp = std::env::temp_dir().join("dat_linter_test_exclude_config.toml");
        std::fs::write(&tmp, toml_text).expect("一時設定ファイルの書き込みに失敗");
        let cfg = LintConfig::load_or_default(Some(&tmp)).expect("設定読み込みに失敗");
        let _ = std::fs::remove_file(&tmp);
        cfg
    };

    let filtered_error_count: usize = records
        .iter()
        .flatten()
        .filter(|(s, c)| {
            *s == Severity::Error
                && config.is_enabled(DiagnosticCode::from_str(c).expect("既知のcodeのはず"))
        })
        .count();
    assert_eq!(
        filtered_error_count, 0,
        "missing-tile-imageをexcludeしたのにerrorが残っている"
    );
}
