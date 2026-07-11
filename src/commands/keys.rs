//! `keys`サブコマンドの実行ロジック。
//!
//! obj種別ごとに有効なキー一覧（`dat_linter::rules::keys::keys_for`）と、
//! 特定キーが取りうる既知の値一覧（`dat_linter::rules::keys::known_values`）を
//! 表示する。VSCode拡張のシンタックスハイライト・スニペット機能の「唯一の正」
//! データソースにするため新設したコマンド（`lint --format json`と同じ設計で
//! `--format json`を持つ）。

use crate::cli::{KeysArgs, KeysFormat};
use dat_linter::i18n::{Language, t};
use dat_linter::registry::{ObjType, SUPPORTED_OBJ_TYPES};
use dat_linter::rules::keys::{keys_for, known_values};
use std::process::ExitCode;

pub fn run_keys(args: &KeysArgs, language: Language) -> ExitCode {
    if args.format == KeysFormat::Json {
        return run_keys_json();
    }
    run_keys_text(language)
}

fn run_keys_text(language: Language) -> ExitCode {
    for &obj in SUPPORTED_OBJ_TYPES {
        // `SUPPORTED_OBJ_TYPES`は`ObjType`から導出された唯一の正であり、
        // `tests/obj_type_coverage.rs`がこの往復（`from_str`が`None`にならない
        // こと）を保証しているため、ここでの`.unwrap()`は呼び出し側のtypo等の
        // プログラミングミスのみを検出する（`common::check_via_dispatch`と
        // 同じ考え方）。
        let obj_type = ObjType::from_str(obj).unwrap_or_else(|| {
            panic!(
                "SUPPORTED_OBJ_TYPES に含まれる \"{obj}\" が ObjType::from_str で None になりました"
            )
        });
        let keys = keys_for(obj_type);
        println!("{:<12} {}", obj, keys.join(", "));
    }

    println!();
    println!(
        "{}",
        t!(language,
            ja: "既知の値一覧:",
            en: "Known values:",
        )
    );
    for (key, values) in known_values() {
        println!("{:<12} {}", key, values.join(", "));
    }

    ExitCode::SUCCESS
}

/// `--format json`のトップレベル出力スキーマ。
/// `{ "obj_types": [{ "obj_type": "building", "keys": [...] }, ...],
///    "known_values": { "waytype": [...], "direction": [...] } }`。
#[derive(serde::Serialize)]
struct JsonKeysOutput {
    obj_types: Vec<JsonObjTypeKeys>,
    known_values: std::collections::BTreeMap<&'static str, Vec<&'static str>>,
}

#[derive(serde::Serialize)]
struct JsonObjTypeKeys {
    obj_type: &'static str,
    keys: Vec<&'static str>,
}

fn run_keys_json() -> ExitCode {
    let obj_types = SUPPORTED_OBJ_TYPES
        .iter()
        .map(|&obj| {
            let obj_type = ObjType::from_str(obj).unwrap_or_else(|| {
                panic!("SUPPORTED_OBJ_TYPES に含まれる \"{obj}\" が ObjType::from_str で None になりました")
            });
            JsonObjTypeKeys {
                obj_type: obj,
                keys: keys_for(obj_type),
            }
        })
        .collect();

    let output = JsonKeysOutput {
        obj_types,
        known_values: known_values()
            .into_iter()
            .map(|(k, v)| (k, v.to_vec()))
            .collect(),
    };

    match serde_json::to_string(&output) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            // `lint --format json`と同じ方針: シリアライズ失敗はバグ
            // （`JsonKeysOutput`は`&'static str`/`Vec<&'static str>`のみで
            // 構成され、通常は失敗し得ない）だが、壊れたJSONをstdoutへ
            // 流すよりは安全なためstderrへ報告する。
            eprintln!("failed to serialize JSON keys output: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
