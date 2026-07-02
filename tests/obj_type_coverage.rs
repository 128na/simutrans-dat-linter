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
use dat_linter::registry::{RuleSet, SUPPORTED_OBJ_TYPES};
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
