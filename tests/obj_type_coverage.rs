//! `registry::SUPPORTED_OBJ_TYPES`（唯一の正）と、実際のディスパッチ先である
//! `RuleSet::for_obj_type`のmatch arm・`formatter::order::order_for`のmatch arm
//! が同じobj種別集合をカバーしていることを検証する。
//!
//! コードレビューで指摘された「対応obj種別の一覧が`main.rs`のヘルプ文言・
//! エラーメッセージ・`registry.rs`・`formatter/order.rs`の4箇所に独立して
//! ハードコードされており、23種別目を追加する際に同期漏れが起きても
//! 誰にも気づかれない」というギャップを、テスト失敗として顕在化させるのが目的。
//! 個々のルール/フォーマット順の中身までは検証しない（それは各obj種別の
//! `tests/<obj>_lint.rs`/`tests/fmt.rs`の責務）。

use dat_linter::formatter::order::order_for;
use dat_linter::parser::DatFile;
use dat_linter::registry::{ObjType, RuleSet, SUPPORTED_OBJ_TYPES};
use dat_linter::rules::keys::{keys_for, known_values_per_obj_type};
use std::collections::BTreeMap;

/// キーが一切無い最小の`DatFile`。`RuleSet::for_obj_type`はディスパッチの
/// 可否だけを見るため、`all(dat)`の内部で`dat.get(...)`が空文字列/`None`を
/// 返す前提の初期化（`resolve_dims`等）が安全に完走できれば十分。
fn empty_dat() -> DatFile {
    DatFile {
        pairs: BTreeMap::new(),
        duplicates: Vec::new(),
    }
}

#[test]
fn all_supported_obj_types_have_rule_set_and_order() {
    let dat = empty_dat();
    let mut missing_rule_set = Vec::new();
    let mut missing_order = Vec::new();

    for &obj in SUPPORTED_OBJ_TYPES {
        if RuleSet::for_obj_type(obj, &dat).is_none() {
            missing_rule_set.push(obj);
        }
        if order_for(obj).is_none() {
            missing_order.push(obj);
        }
    }

    assert!(
        missing_rule_set.is_empty(),
        "registry::SUPPORTED_OBJ_TYPES に列挙されているが RuleSet::for_obj_type が \
         None を返す obj 種別があります: {missing_rule_set:?}"
    );
    assert!(
        missing_order.is_empty(),
        "registry::SUPPORTED_OBJ_TYPES に列挙されているが formatter::order::order_for \
         が None を返す obj 種別があります: {missing_order:?}"
    );
}

/// `registry::SUPPORTED_OBJ_TYPES`（文字列リスト）と`registry::ObjType`（enum）が
/// 機械的に同期していることを検証する。両者は独立管理ではなく、
/// `SUPPORTED_OBJ_TYPES`の各文字列が`ObjType::from_str`で漏れなく`ObjType`に
/// 変換でき、かつ`as_str`で元の文字列にラウンドトリップすることを保証する。
/// 件数についても、`SUPPORTED_OBJ_TYPES`の要素数と`ObjType`の全variant数
/// （下記`ALL_OBJ_TYPES`で列挙）が一致することを確認する。
#[test]
fn supported_obj_types_matches_obj_type_enum() {
    for &obj in SUPPORTED_OBJ_TYPES {
        let parsed = ObjType::from_str(obj);
        assert!(
            parsed.is_some(),
            "SUPPORTED_OBJ_TYPES に含まれる \"{obj}\" が ObjType::from_str で None になりました"
        );
        assert_eq!(
            parsed.unwrap().as_str(),
            obj,
            "ObjType::from_str(\"{obj}\").as_str() が元の文字列にラウンドトリップしません"
        );
    }

    const ALL_OBJ_TYPES: &[ObjType] = &[
        ObjType::Building,
        ObjType::Vehicle,
        ObjType::Way,
        ObjType::Good,
        ObjType::Bridge,
        ObjType::Tunnel,
        ObjType::Roadsign,
        ObjType::Crossing,
        ObjType::WayObject,
        ObjType::GroundObj,
        ObjType::Tree,
        ObjType::Citycar,
        ObjType::Pedestrian,
        ObjType::Factory,
        ObjType::Sound,
        ObjType::Ground,
        ObjType::Menu,
        ObjType::Cursor,
        ObjType::Symbol,
        ObjType::Smoke,
        ObjType::Field,
        ObjType::Misc,
    ];

    assert_eq!(
        SUPPORTED_OBJ_TYPES.len(),
        ALL_OBJ_TYPES.len(),
        "SUPPORTED_OBJ_TYPES の要素数と ObjType の全variant数が一致しません"
    );

    // 双方向の集合一致（重複や順序違いがあっても検出できるよう文字列集合で比較）。
    let from_list: std::collections::BTreeSet<&str> = SUPPORTED_OBJ_TYPES.iter().copied().collect();
    let from_enum: std::collections::BTreeSet<&str> =
        ALL_OBJ_TYPES.iter().map(|t| t.as_str()).collect();
    assert_eq!(
        from_list, from_enum,
        "SUPPORTED_OBJ_TYPES と ObjType の変換結果の集合が一致しません"
    );
}

/// `rules::keys::keys_for`（VSCode拡張のシンタックスハイライト・スニペット機能の
/// データソース）が、全`SUPPORTED_OBJ_TYPES`について空でない・`"obj"`・`"name"`と
/// `"copyright"`を含む・重複が無いことを検証する。`keys_for`自体のmatchは
/// ワイルドカードなしの網羅matchでコンパイル時に強制されるが、各obj種別の
/// 定数（`BUILDING_KEYS`等）の中身がこの3条件を満たすことまではコンパイラが
/// 保証しないため、このテストで確認する。
#[test]
fn keys_for_all_obj_types_are_well_formed() {
    for &obj in SUPPORTED_OBJ_TYPES {
        let obj_type =
            ObjType::from_str(obj).expect("SUPPORTED_OBJ_TYPESはObjTypeに変換できるはず");
        let keys = keys_for(obj_type);

        assert!(!keys.is_empty(), "obj={obj} の keys_for が空です");
        assert!(
            keys.contains(&"obj"),
            "obj={obj} の keys_for に \"obj\" が含まれていません"
        );
        assert!(
            keys.contains(&"name"),
            "obj={obj} の keys_for に \"name\" が含まれていません"
        );
        assert!(
            keys.contains(&"copyright"),
            "obj={obj} の keys_for に \"copyright\" が含まれていません"
        );

        let unique: std::collections::BTreeSet<&str> = keys.iter().copied().collect();
        assert_eq!(
            unique.len(),
            keys.len(),
            "obj={obj} の keys_for に重複したキーがあります: {keys:?}"
        );
    }
}

/// 第20弾（`known_values`拡張）: `rules::keys::known_values_per_obj_type`が返す
/// obj種別・キーごとの既知値一覧について、各エントリの`values`が空でない・
/// 重複が無いことを確認する。`type`/`location`/`climates`/`name`（skin系）いずれも
/// 手動キュレーションされた定数（`KNOWN_WAYTYPES`と同じ位置づけ、`rules/keys.rs`の
/// `known_values_per_obj_type`のdocコメント参照）であり、コンパイラは中身の正しさを
/// 保証しないため、このテストで空・重複だけは機械的に検出する。
#[test]
fn known_values_per_obj_type_entries_are_well_formed() {
    let entries = known_values_per_obj_type();

    // 期待するobj_type/keyの組が過不足なく列挙されていることも確認する
    // （うっかり削除・重複追加への回帰検知）。
    let expected_pairs: std::collections::BTreeSet<(&str, &str)> = [
        ("building", "type"),
        ("factory", "location"),
        ("building", "climates"),
        ("tree", "climates"),
        ("ground_obj", "climates"),
        ("factory", "climates"),
        ("menu", "name"),
        ("cursor", "name"),
        ("symbol", "name"),
        ("misc", "name"),
        ("ground", "name"),
    ]
    .into_iter()
    .collect();

    let actual_pairs: std::collections::BTreeSet<(&str, &str)> =
        entries.iter().map(|e| (e.obj_type, e.key)).collect();

    assert_eq!(
        actual_pairs, expected_pairs,
        "known_values_per_obj_type() の(obj_type, key)集合が期待と一致しません"
    );

    for entry in &entries {
        assert!(
            !entry.values.is_empty(),
            "({}, {}) の values が空です",
            entry.obj_type,
            entry.key
        );

        let unique: std::collections::BTreeSet<&str> = entry.values.iter().copied().collect();
        assert_eq!(
            unique.len(),
            entry.values.len(),
            "({}, {}) の values に重複があります: {:?}",
            entry.obj_type,
            entry.key,
            entry.values
        );

        // obj_typeはSUPPORTED_OBJ_TYPESに実在する文字列でなければならない
        // （typoの検出）。
        assert!(
            SUPPORTED_OBJ_TYPES.contains(&entry.obj_type),
            "obj_type={} がSUPPORTED_OBJ_TYPESに存在しません",
            entry.obj_type
        );
    }
}

/// `(building, type)`の既知値一覧が、現行有効値（例: "res"）とobsolete値
/// （例: "station"）の両方を含み、`""`（未指定を表す空文字列プレースホルダ）は
/// JSON上の値としては無意味なため除外されていることを確認する
/// （`rules/keys.rs::known_values_per_obj_type`のdocコメント参照）。
#[test]
fn building_type_known_values_include_valid_and_obsolete_but_not_empty_string() {
    let entries = known_values_per_obj_type();
    let building_type = entries
        .iter()
        .find(|e| e.obj_type == "building" && e.key == "type")
        .expect("(building, type) のエントリが見つかりません");

    assert!(
        building_type.values.contains(&"res"),
        "現行有効値\"res\"が含まれていません: {:?}",
        building_type.values
    );
    assert!(
        building_type.values.contains(&"station"),
        "obsolete値\"station\"が含まれていません: {:?}",
        building_type.values
    );
    assert!(
        !building_type.values.contains(&""),
        "空文字列プレースホルダが値一覧に含まれるべきではありません: {:?}",
        building_type.values
    );
}

/// `(cursor, name)`/`(symbol, name)`は、それぞれ固有の名前一覧と共有の
/// `FAKULTATIVE_SKIN_NAMES`（21件、`rules/common.rs`）を結合したものであることを
/// 確認する。両者が結合後の集合として`FAKULTATIVE_SKIN_NAMES`の代表的な値
/// （"TrainStop"）を含むこと、かつ固有名（cursorの"Builder"、symbolの"Waren"）が
/// 互いのobj_typeには含まれないことを確認する。
#[test]
fn cursor_and_symbol_name_values_share_fakultative_names_but_not_own_names() {
    let entries = known_values_per_obj_type();
    let cursor_names = &entries
        .iter()
        .find(|e| e.obj_type == "cursor" && e.key == "name")
        .expect("(cursor, name) のエントリが見つかりません")
        .values;
    let symbol_names = &entries
        .iter()
        .find(|e| e.obj_type == "symbol" && e.key == "name")
        .expect("(symbol, name) のエントリが見つかりません")
        .values;

    assert!(cursor_names.contains(&"TrainStop"));
    assert!(symbol_names.contains(&"TrainStop"));

    assert!(cursor_names.contains(&"Builder"));
    assert!(!symbol_names.contains(&"Builder"));

    assert!(symbol_names.contains(&"Waren"));
    assert!(!cursor_names.contains(&"Waren"));
}
