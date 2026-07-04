//! `dat_linter list`（第9弾項目2）が表示する、全`Diagnostic.code`の一覧。
//!
//! ## 設計
//! `Rule::check`は実際の`RuleContext`（`DatFile`の中身次第で分岐が変わる）を
//! 要求するため、「全ルールを実行して出現したcodeを集める」という完全に動的な
//! 収集は、あらゆる分岐を通す大量の合成`.dat`データを用意しない限り現実的でない
//! （すでに62種の診断codeが`src/rules/*.rs`・`src/couplings.rs`・
//! `src/formatter/mod.rs`に散らばっており、多くが特定のフィールド値の組み合わせ
//! でしか到達しない分岐に対応する）。
//!
//! そのため、このモジュールでは`ALL_CODES`という静的な一覧を保持しつつ、
//! `tests/codes_completeness.rs`で実際のソースファイル（`src/rules/*.rs`・
//! `src/couplings.rs`・`src/formatter/mod.rs`）を正規表現で走査し、
//! `Diagnostic::error/warning/info/debug("code", ...)`の形で実際に使われている
//! 全codeがこの一覧に過不足なく含まれることをテストで保証する（ここが
//! ドリフト防止の要）。ルールを追加・削除した際にこの一覧の更新を忘れると
//! そのテストが落ちる。
//!
//! `source`は「どのサブシステムが出すcodeか」を表す（`lint`のルール一つ一つに
//! 対応するファイル名まで露出する必要は無いため、obj種別非依存の粒度に留める）。

/// `code`がどのサブコマンド／サブシステム由来かを示す粗い分類。
/// `dat_linter list`はこの値でグループ化して表示する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeSource {
    /// `lint`（各obj種別の`Rule`実装、`src/rules/*.rs`）が出すcode。
    Lint,
    /// `fmt`（`src/formatter/mod.rs`）が出すcode。
    Fmt,
    /// `analyze --kind coupling`（`src/couplings.rs`）が出すcode。
    Analyze,
}

impl CodeSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CodeSource::Lint => "lint",
            CodeSource::Fmt => "fmt",
            CodeSource::Analyze => "analyze",
        }
    }
}

/// 1つの診断codeの情報。`dat_linter.toml`の`[rules] include/exclude`に
/// そのまま書ける文字列が`code`。
#[derive(Debug, Clone, Copy)]
pub struct CodeInfo {
    pub code: &'static str,
    pub source: CodeSource,
}

/// 全`Diagnostic.code`の一覧（`dat_linter list`が表示する内容そのもの）。
/// 同じcodeが複数のobj種別モジュールで共有される場合（例:
/// `missing-waytype`はbuilding.rs内の分岐とcommon.rs経由の両方から出る）でも
/// 一意のcode文字列としては1エントリのみ列挙する（重複表示しない）。
///
/// `tests/codes_completeness.rs`が実ソースとの整合性を保証するため、
/// ここに列挙されるcodeを追加・削除する際は特別な追加作業は不要
/// （そのテストが自動的に過不足を検出する）。
pub const ALL_CODES: &[CodeInfo] = &[
    // --- lint: src/rules/bridge.rs ---
    CodeInfo {
        code: "clamped-value-out-of-range",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "no-bridge-image-specified",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/building.rs ---
    CodeInfo {
        code: "parsed-pairs",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "raw-type-waytype",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "obsolete-type",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "unknown-type",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "type-waytype-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "generic-extension",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "obsolete-keyword",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "dims-resolved",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "zero-size",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "dims-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "raw-cursor-icon",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "cursor-icon-not-applicable",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-cursor-icon",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "tile-key-lookup",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-tile-image",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "tile-image-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "frontimage-height",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/citycar.rs, pedestrian.rs (共有code) ---
    CodeInfo {
        code: "image-omitted",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/common.rs ---
    CodeInfo {
        code: "duplicate-key",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-waytype",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "unknown-waytype",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "waytype-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "image-ref-empty-sentinel",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "image-ref-resolved",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-image-file",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "image-size-not-multiple-of-128",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "image-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "unreadable-image",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/crossing.rs ---
    CodeInfo {
        code: "crossing-identical-waytypes",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "crossing-missing-speed",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "crossing-missing-openimage",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/factory.rs ---
    CodeInfo {
        code: "factory-type-override",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "factory-missing-mapcolor",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "factory-mapcolor-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "factory-output-capacity-too-small",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "factory-smoketile-without-offset",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "factory-probability-clamped",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "factory-productivity-zero",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/groundobj.rs ---
    CodeInfo {
        code: "waytype-omitted",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-season-image",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "no-images",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/roadsign.rs ---
    CodeInfo {
        code: "roadsign-image-count-not-multiple-of-4",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "roadsign-image-missing",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/tree.rs ---
    CodeInfo {
        code: "missing-age-season-image",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/vehicle.rs ---
    CodeInfo {
        code: "engine-type-skipped",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "unknown-engine-type",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "incomplete-8-direction-images",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "freightimage-count-mismatch",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-indexed-freightimage",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "missing-freightimagetype",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "extra-freightimagetype",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "power-gear-mismatch",
        source: CodeSource::Lint,
    },
    // --- lint: src/rules/way.rs ---
    CodeInfo {
        code: "missing-base-image",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "base-image-ok",
        source: CodeSource::Lint,
    },
    CodeInfo {
        code: "clip-below-out-of-range",
        source: CodeSource::Lint,
    },
    // --- fmt: src/formatter/mod.rs ---
    CodeInfo {
        code: "fmt-leading-space-line",
        source: CodeSource::Fmt,
    },
    CodeInfo {
        code: "fmt-malformed-line",
        source: CodeSource::Fmt,
    },
    CodeInfo {
        code: "fmt-reorder-unsupported-obj",
        source: CodeSource::Fmt,
    },
    CodeInfo {
        code: "fmt-reorder-lines-dropped",
        source: CodeSource::Fmt,
    },
    // --- analyze: src/couplings.rs ---
    CodeInfo {
        code: "read-dir-failed",
        source: CodeSource::Analyze,
    },
    CodeInfo {
        code: "read-failed",
        source: CodeSource::Analyze,
    },
    CodeInfo {
        code: "missing-name",
        source: CodeSource::Analyze,
    },
    CodeInfo {
        code: "dangling-vehicle-constraint",
        source: CodeSource::Analyze,
    },
    CodeInfo {
        code: "unsatisfiable-constraint",
        source: CodeSource::Analyze,
    },
];
