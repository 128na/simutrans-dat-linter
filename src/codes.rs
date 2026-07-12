//! `dat_linter list`（第9弾項目2）が表示する、全診断codeの一覧と、
//! `DiagnosticCode`という型安全なenumでの表現（第17弾・code smellレビュータスク13）。
//!
//! ## 設計（第17弾で`&'static str`からenumへ移行）
//! 以前は`Diagnostic.code`が`&'static str`のみで、`ALL_CODES`（当時は
//! `&[CodeInfo]`という静的配列）との整合性は`tests/codes_completeness.rs`の
//! 正規表現ソーススキャンという実行時テストでしか保証されていなかった
//! （テストを流し忘れるとドリフトに気付けない、`cargo build`は通ってしまう）。
//!
//! この問題を解消するため、全67種のcodeを`DiagnosticCode`という列挙型の
//! variantとして定義し、`as_str`・`info`（`CodeInfo`を返す）の2つを
//! **ワイルドカードarmを持たない網羅match**として実装した
//! （`registry::RuleSet::for_obj_type`・`formatter::order::order_for`と同じ、
//! このプロジェクトの既存規約）。これにより:
//! - 新しいvariantを`DiagnosticCode`に追加したのに`as_str`/`info`への
//!   arm追加を忘れると、**`cargo build`が非網羅match errorで失敗する**
//!   （実行時テストを流さなくても、コンパイルの時点でドリフトを検出できる）。
//! - `Diagnostic::error(code, ...)`のような呼び出しは`code`に
//!   `DiagnosticCode::MissingWaytype`のようなvariantを渡す形になり、
//!   文字列のtypoによる「存在しないcodeを指定してしまう」バグ自体が
//!   コンパイルエラーになる。
//!
//! 一方、`.dat_linter.toml`の`[rules] include/exclude`（ユーザーが手で書く
//! TOML）やCLIの`describe <code>`引数は、UXとして引き続き文字列で受け付ける
//! 必要があるため、`DiagnosticCode::from_str(&str) -> Option<Self>`
//! （`ALL`定数配列を線形探索するだけの薄い実装）を用意して文字列↔enumの
//! 相互変換を担う。`ALL`定数配列自体は手動保持（`strum`等のenum列挙クレートを
//! 新規依存に追加しない、このプロジェクトの既存の依存最小方針を踏襲）だが、
//! これが古くなった場合の検出は`tests/codes_completeness.rs`
//! （実ソースの`Diagnostic::x(DiagnosticCode::...)`呼び出しをスキャンし、
//! `ALL`と過不足なく一致するか確認する軽量な回帰テストとして残した）に委ねる。
//!
//! `source`は「どのサブコマンドが出すcodeか」を表す（`lint`のルール一つ一つに
//! 対応するファイル名まで露出する必要は無いため、obj種別非依存の粒度に留める）。
//!
//! ## `describe`（第10弾項目6）
//! 各codeには`why`（なぜNGか。makeobj/ゲームランタイムの実際の挙動を根拠とする）と
//! `how_to_fix`（どう直すか）の説明をJA/EN両方で追加した。`dat_linter describe <code>`
//! はこれを表示する。説明文は各ルールの実装（`src/rules/*.rs`のdocコメント・
//! `Diagnostic`メッセージそのもの）を直接読んで書いたもので、機械的なコピペや
//! 当て推量ではない（該当箇所は各`why`/`how_to_fix`のコメントで参照元ファイルを示す）。

use crate::i18n::Language;

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

/// 1つの診断codeの情報。`why_ja`/`why_en`（なぜNGか）と`fix_ja`/`fix_en`
/// （どう直すか）は`dat_linter describe <code>`が表示する説明文（第10弾項目6）。
/// `code`自体は`DiagnosticCode::info()`の戻り値の一部として持たせず、
/// 呼び出し元（`DiagnosticCode::info`）が`self`から`code`フィールドへ詰める
/// （`CodeInfo`単体では「どのcodeの情報か」を復元できないため、
/// `ALL_CODES`相当のイテレーション時は`(DiagnosticCode, CodeInfo)`のペアで
/// 扱う。`dat_linter list`/`describe`の実装（`src/commands/list.rs`・
/// `src/commands/describe.rs`）参照）。
#[derive(Debug, Clone, Copy)]
pub struct CodeInfo {
    pub code: DiagnosticCode,
    pub source: CodeSource,
    why_ja: &'static str,
    why_en: &'static str,
    fix_ja: &'static str,
    fix_en: &'static str,
}

impl CodeInfo {
    /// なぜこのcodeが検出されるのか（makeobj/ゲームランタイムの実際の挙動を根拠とする）。
    pub fn why(&self, lang: Language) -> &'static str {
        t_static(lang, self.why_ja, self.why_en)
    }

    /// どう直せばよいか。
    pub fn how_to_fix(&self, lang: Language) -> &'static str {
        t_static(lang, self.fix_ja, self.fix_en)
    }
}

/// `t!`マクロは`format!`（`String`を返す）前提のため、引数を持たない静的文字列の
/// 選択にはこの薄いヘルパーを使う（`&'static str`のまま返せる）。
fn t_static(lang: Language, ja: &'static str, en: &'static str) -> &'static str {
    match lang {
        Language::Japanese => ja,
        Language::English => en,
    }
}

/// 全診断code（67種）を表す型安全なenum。`Diagnostic::error/warning/info/debug`の
/// `code`引数はこの型を取る。variant名は元のkebab-case文字列（`as_str()`が返す値）を
/// PascalCaseにしたもの（例: `"missing-waytype"` → `MissingWaytype`）。
///
/// 新しいvariantを追加した場合、`as_str`・`info`（共にワイルドカードarmを
/// 持たない網羅match）と`ALL`定数配列の更新が必要。前2つを忘れると
/// `cargo build`が失敗する。`ALL`の更新漏れは`tests/codes_completeness.rs`が検出する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    // --- lint: src/rules/bridge.rs ---
    ClampedValueOutOfRange,
    NoBridgeImageSpecified,
    // --- lint: src/rules/building.rs ---
    ParsedPairs,
    RawTypeWaytype,
    ObsoleteType,
    UnknownType,
    TypeWaytypeOk,
    GenericExtension,
    ObsoleteKeyword,
    DimsResolved,
    ZeroSize,
    DimsOk,
    RawCursorIcon,
    CursorIconNotApplicable,
    MissingCursorIcon,
    TileKeyLookup,
    MissingTileImage,
    TileImageOk,
    FrontimageHeight,
    BooleanStyleFieldNotZeroOrOne,
    // --- lint: src/rules/citycar.rs, pedestrian.rs (共有code) ---
    ImageOmitted,
    // --- lint: src/rules/common.rs ---
    DuplicateKey,
    MissingWaytype,
    UnknownWaytype,
    WaytypeOk,
    ImageRefEmptySentinel,
    ImageRefResolved,
    MissingImageFile,
    ImageSizeNotMultipleOf128,
    ImageCoordinateOutOfBounds,
    ImageOk,
    UnreadableImage,
    DateIndexOverflow,
    NameForbiddenFilenameCharacter,
    NarrowIntOverflow,
    EmbeddedNulInStringField,
    UnknownSkinName,
    // --- lint: src/rules/crossing.rs ---
    CrossingIdenticalWaytypes,
    CrossingMissingSpeed,
    CrossingMissingOpenimage,
    // --- lint: src/rules/factory.rs ---
    FactoryTypeOverride,
    FactoryMissingMapcolor,
    FactoryMapcolorOk,
    FactoryOutputCapacityTooSmall,
    FactorySmoketileWithoutOffset,
    FactoryProbabilityClamped,
    FactoryProductivityZero,
    // --- lint: src/rules/groundobj.rs ---
    WaytypeOmitted,
    MissingSeasonImage,
    NoImages,
    // --- lint: src/rules/roadsign.rs ---
    RoadsignImageCountNotMultipleOf4,
    RoadsignImageMissing,
    // --- lint: src/rules/tree.rs ---
    MissingAgeSeasonImage,
    // --- lint: src/rules/vehicle.rs ---
    EngineTypeSkipped,
    UnknownEngineType,
    Incomplete8DirectionImages,
    FreightimageCountMismatch,
    MissingIndexedFreightimage,
    MissingFreightimagetype,
    ExtraFreightimagetype,
    PowerGearMismatch,
    // --- lint: src/rules/way.rs ---
    MissingBaseImage,
    BaseImageOk,
    ClipBelowOutOfRange,
    // --- fmt: src/commands/fmt.rs — 機能トグル専用code。
    // 実ソースでDiagnostic::x()として発行されることは無い
    // （理由はALL_CODES内の該当CodeInfoのコメント参照）。
    FmtReorderApplied,
    // --- fmt: src/formatter/mod.rs ---
    FmtLeadingSpaceLine,
    FmtMalformedLine,
    FmtReorderUnsupportedObj,
    FmtReorderLinesDropped,
    // --- analyze: src/couplings.rs ---
    ReadDirFailed,
    ReadFailed,
    MissingName,
    DanglingVehicleConstraint,
    UnsatisfiableConstraint,
    // --- lint: src/commands/lint.rs — `--format json`専用。テキストモードでは
    // 対応するメッセージがeprintln!されるだけでDiagnosticとしては構築されない
    // （obj=は未対応です等）が、JSON出力ではdiagnostics配列内の1エントリとして
    // 構造化する必要があるため新設した。
    FileReadFailed,
    UnsupportedObjType,
}

impl DiagnosticCode {
    /// `dat_linter.toml`の`[rules] include/exclude`にそのまま書ける文字列表現。
    /// **ワイルドカードarmを持たない網羅match**（このプロジェクトの規約）。
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticCode::ClampedValueOutOfRange => "clamped-value-out-of-range",
            DiagnosticCode::NoBridgeImageSpecified => "no-bridge-image-specified",
            DiagnosticCode::ParsedPairs => "parsed-pairs",
            DiagnosticCode::RawTypeWaytype => "raw-type-waytype",
            DiagnosticCode::ObsoleteType => "obsolete-type",
            DiagnosticCode::UnknownType => "unknown-type",
            DiagnosticCode::TypeWaytypeOk => "type-waytype-ok",
            DiagnosticCode::GenericExtension => "generic-extension",
            DiagnosticCode::ObsoleteKeyword => "obsolete-keyword",
            DiagnosticCode::DimsResolved => "dims-resolved",
            DiagnosticCode::ZeroSize => "zero-size",
            DiagnosticCode::DimsOk => "dims-ok",
            DiagnosticCode::RawCursorIcon => "raw-cursor-icon",
            DiagnosticCode::CursorIconNotApplicable => "cursor-icon-not-applicable",
            DiagnosticCode::MissingCursorIcon => "missing-cursor-icon",
            DiagnosticCode::TileKeyLookup => "tile-key-lookup",
            DiagnosticCode::MissingTileImage => "missing-tile-image",
            DiagnosticCode::TileImageOk => "tile-image-ok",
            DiagnosticCode::FrontimageHeight => "frontimage-height",
            DiagnosticCode::BooleanStyleFieldNotZeroOrOne => "boolean-style-field-not-zero-or-one",
            DiagnosticCode::ImageOmitted => "image-omitted",
            DiagnosticCode::DuplicateKey => "duplicate-key",
            DiagnosticCode::MissingWaytype => "missing-waytype",
            DiagnosticCode::UnknownWaytype => "unknown-waytype",
            DiagnosticCode::WaytypeOk => "waytype-ok",
            DiagnosticCode::ImageRefEmptySentinel => "image-ref-empty-sentinel",
            DiagnosticCode::ImageRefResolved => "image-ref-resolved",
            DiagnosticCode::MissingImageFile => "missing-image-file",
            DiagnosticCode::ImageSizeNotMultipleOf128 => "image-size-not-multiple-of-128",
            DiagnosticCode::ImageCoordinateOutOfBounds => "image-coordinate-out-of-bounds",
            DiagnosticCode::ImageOk => "image-ok",
            DiagnosticCode::UnreadableImage => "unreadable-image",
            DiagnosticCode::DateIndexOverflow => "date-index-overflow",
            DiagnosticCode::NameForbiddenFilenameCharacter => "name-forbidden-filename-character",
            DiagnosticCode::NarrowIntOverflow => "narrow-int-overflow",
            DiagnosticCode::EmbeddedNulInStringField => "embedded-nul-in-string-field",
            DiagnosticCode::UnknownSkinName => "unknown-skin-name",
            DiagnosticCode::CrossingIdenticalWaytypes => "crossing-identical-waytypes",
            DiagnosticCode::CrossingMissingSpeed => "crossing-missing-speed",
            DiagnosticCode::CrossingMissingOpenimage => "crossing-missing-openimage",
            DiagnosticCode::FactoryTypeOverride => "factory-type-override",
            DiagnosticCode::FactoryMissingMapcolor => "factory-missing-mapcolor",
            DiagnosticCode::FactoryMapcolorOk => "factory-mapcolor-ok",
            DiagnosticCode::FactoryOutputCapacityTooSmall => "factory-output-capacity-too-small",
            DiagnosticCode::FactorySmoketileWithoutOffset => "factory-smoketile-without-offset",
            DiagnosticCode::FactoryProbabilityClamped => "factory-probability-clamped",
            DiagnosticCode::FactoryProductivityZero => "factory-productivity-zero",
            DiagnosticCode::WaytypeOmitted => "waytype-omitted",
            DiagnosticCode::MissingSeasonImage => "missing-season-image",
            DiagnosticCode::NoImages => "no-images",
            DiagnosticCode::RoadsignImageCountNotMultipleOf4 => {
                "roadsign-image-count-not-multiple-of-4"
            }
            DiagnosticCode::RoadsignImageMissing => "roadsign-image-missing",
            DiagnosticCode::MissingAgeSeasonImage => "missing-age-season-image",
            DiagnosticCode::EngineTypeSkipped => "engine-type-skipped",
            DiagnosticCode::UnknownEngineType => "unknown-engine-type",
            DiagnosticCode::Incomplete8DirectionImages => "incomplete-8-direction-images",
            DiagnosticCode::FreightimageCountMismatch => "freightimage-count-mismatch",
            DiagnosticCode::MissingIndexedFreightimage => "missing-indexed-freightimage",
            DiagnosticCode::MissingFreightimagetype => "missing-freightimagetype",
            DiagnosticCode::ExtraFreightimagetype => "extra-freightimagetype",
            DiagnosticCode::PowerGearMismatch => "power-gear-mismatch",
            DiagnosticCode::MissingBaseImage => "missing-base-image",
            DiagnosticCode::BaseImageOk => "base-image-ok",
            DiagnosticCode::ClipBelowOutOfRange => "clip-below-out-of-range",
            DiagnosticCode::FmtReorderApplied => "fmt-reorder-applied",
            DiagnosticCode::FmtLeadingSpaceLine => "fmt-leading-space-line",
            DiagnosticCode::FmtMalformedLine => "fmt-malformed-line",
            DiagnosticCode::FmtReorderUnsupportedObj => "fmt-reorder-unsupported-obj",
            DiagnosticCode::FmtReorderLinesDropped => "fmt-reorder-lines-dropped",
            DiagnosticCode::ReadDirFailed => "read-dir-failed",
            DiagnosticCode::ReadFailed => "read-failed",
            DiagnosticCode::MissingName => "missing-name",
            DiagnosticCode::DanglingVehicleConstraint => "dangling-vehicle-constraint",
            DiagnosticCode::UnsatisfiableConstraint => "unsatisfiable-constraint",
            DiagnosticCode::FileReadFailed => "file-read-failed",
            DiagnosticCode::UnsupportedObjType => "unsupported-obj-type",
        }
    }

    /// `as_str()`の逆変換。`dat_linter.toml`の`[rules] include/exclude`や
    /// `describe <code>`引数など、ユーザーが文字列でcodeを指定するUXのために
    /// 用意する（`ALL`定数配列の線形探索。67件程度なのでハッシュテーブル化は
    /// 過剰設計と判断）。
    ///
    /// 名前は`std::str::FromStr`トレイトと紛らわしいが、`registry::ObjType::from_str`・
    /// `i18n::Language::from_str`と同じ理由でこのシグネチャ（inherent method、
    /// `&str` -> `Option<Self>`）を踏襲するため`clippy::should_implement_trait`を
    /// 明示的に許容する。
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        ALL.iter().find(|c| c.as_str() == s).copied()
    }

    /// このcodeの`CodeInfo`（source・why・how_to_fix）を返す。
    /// **ワイルドカードarmを持たない網羅match**（このプロジェクトの規約）。
    /// 新しいvariantを追加してこのmatchへのarm追加を忘れると`cargo build`が
    /// 非網羅match errorで失敗する（第17弾の本丸。以前はこの整合性を
    /// `tests/codes_completeness.rs`の実行時テストでしか保証できなかった）。
    pub fn info(&self) -> CodeInfo {
        match self {
            DiagnosticCode::ClampedValueOutOfRange => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                // bridge.rs ClampedRangeRule / way.rs ClipBelowRangeRule 共通の根拠。
                // tabfileobj_t::get_int_clamped()（tabfile.cc:201-212）は範囲外の値を
                // dbg->warningを出した上で黙って範囲内にクランプする（FATALにはしない）。
                why_ja: "bridgeの数値フィールド（pillar_distance/pillar_asymmetric/max_lenght/max_length/\
                    max_height/axle_load/clip_below/intro_year/intro_month/retire_year/retire_month）が\
                    許容範囲外です。makeobjのtabfileobj_t::get_int_clamped()はFATALにはしませんが、\
                    警告を出した上で値を黙って範囲内にクランプします。指定した値と実際にpakへ\
                    書き込まれる値が一致しなくなります",
                why_en: "A bridge numeric field (pillar_distance/pillar_asymmetric/max_lenght/max_length/\
                    max_height/axle_load/clip_below/intro_year/intro_month/retire_year/retire_month) is \
                    out of its allowed range. makeobj's tabfileobj_t::get_int_clamped() does not treat \
                    this as FATAL, but warns and silently clamps the value into range, so the value \
                    actually written to the pak differs from what you specified",
                fix_ja: "各フィールドの許容範囲内（例: intro_month/retire_monthは1..12、pillar_asymmetric/\
                    clip_belowは0..1）に収まるよう値を修正してください。警告文が示す範囲を確認してください",
                fix_en: "Set the value within the field's allowed range (e.g. intro_month/retire_month is \
                    1..12, pillar_asymmetric/clip_below is 0..1). Check the range shown in the warning text",
            },
            DiagnosticCode::NoBridgeImageSpecified => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "front{name}[{dir}]（季節ありなら末尾に[season]も付く）の値が2文字以下\
                    （空文字列や\"-\"を含む）です。bridge_writer.cc（write_bridge_images）は\
                    front側の値がこの条件を満たすと\"No ... specified (might still work)\"という\
                    警告を出します。FATALにはならず、橋が完全に描画されないわけではありませんが、\
                    通常は前景画像の指定漏れを示します",
                why_en: "The value of front{name}[{dir}] (with a trailing [season] if seasonal) is 2 \
                    characters or fewer (including empty or \"-\"). bridge_writer.cc's write_bridge_images \
                    warns \"No ... specified (might still work)\" when the front-side value meets this \
                    condition. This is not FATAL and the bridge may still render, but it usually indicates \
                    a missing foreground image",
                fix_ja: "その方向・季節の前景（front）画像を指定するか、意図的に省略する場合は\
                    そのままで構いません（\"might still work\"の通り、必須ではありません）",
                fix_en: "Specify the foreground (front) image for that direction/season, or leave it as-is \
                    if the omission is intentional (as the message says, this is not required)",
            },
            DiagnosticCode::ParsedPairs => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。読み込んだ\
                    key=valueの総数を示すだけで、問題を示すものではありません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv). \
                    It just reports the total number of key=value pairs loaded and does not indicate a \
                    problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::RawTypeWaytype => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。type/waytypeの\
                    生の値を示すだけで、問題を示すものではありません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    that reports the raw type/waytype values. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::ObsoleteType => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "type=station/railstop/monorailstop/busstop/carstop/airport/wharf/hall/post/shedは\
                    obsoleteです。building_writer.ccはこれらをdbg->fatalでFATAL ERRORにします",
                why_en: "type=station/railstop/monorailstop/busstop/carstop/airport/wharf/hall/post/shed \
                    is obsolete. building_writer.cc treats these as a FATAL ERROR via dbg->fatal",
                fix_ja: "type=stop または type=extension に変更し、waytype=（track/road/water等）を\
                    明示的に指定してください",
                fix_en: "Change to type=stop or type=extension and explicitly specify waytype= (track/\
                    road/water, etc.)",
            },
            DiagnosticCode::UnknownType => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "typeがmakeobjの認識する既知値（res/com/ind/cur/mon/tow/hq/habour/harbour/dock/\
                    fac/stop/extension/depot/any/空文字列）のいずれとも一致しません。\
                    building_writer.ccはこの場合dbg->fatalでFATAL ERRORにします",
                why_en: "type does not match any value makeobj recognizes (res/com/ind/cur/mon/tow/hq/\
                    habour/harbour/dock/fac/stop/extension/depot/any/empty). building_writer.cc treats \
                    this as a FATAL ERROR via dbg->fatal",
                fix_ja: "typeの綴りを確認し、既知値のいずれかに修正してください",
                fix_en: "Check the spelling of type and correct it to one of the known values",
            },
            DiagnosticCode::TypeWaytypeOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。type/waytypeの組み合わせが正常であることを\
                    示すだけで、問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming that the type/waytype \
                    combination is valid. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::GenericExtension => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "type=extensionでwaytypeが未指定です。building_writer.ccはこれを\
                    「全waytypeに適合する汎用拡張」として解釈しますが、意図せず省略していると\
                    想定外の駅拡張になる可能性があります",
                why_en: "type=extension has no waytype specified. building_writer.cc interprets this as \
                    a \"generic extension that fits any waytype\", but omitting it unintentionally can \
                    result in an unexpected station extension",
                fix_ja: "汎用拡張として意図している場合はそのままで構いません。特定waytype専用の\
                    拡張建物にしたい場合はwaytype=を明示的に指定してください",
                fix_en: "If a generic extension is intended, leave it as-is. If you want the extension \
                    building to be specific to a waytype, explicitly specify waytype=",
            },
            DiagnosticCode::ObsoleteKeyword => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "extension_buildingキーはobsoleteです。building_writer.ccはこれを\
                    dbg->fatalでFATAL ERRORにします",
                why_en: "The extension_building key is obsolete. building_writer.cc treats this as a \
                    FATAL ERROR via dbg->fatal",
                fix_ja: "extension_buildingを削除し、代わりにtype=stop または type=extension と \
                    waytype= を指定してください",
                fix_en: "Remove extension_building and instead specify type=stop or type=extension with \
                    waytype=",
            },
            DiagnosticCode::DimsResolved => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。Dims=から\
                    解決されたsize_x/size_y/layoutsを示すだけで、問題を示すものではありません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    that reports size_x/size_y/layouts resolved from Dims=. It does not indicate a \
                    problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::ZeroSize => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "Dims=から解決されたsize_x*size_yが0です。building_writer.ccは\
                    \"Cannot create a building with zero size\"としてdbg->fatalでFATAL ERRORにします",
                why_en: "size_x*size_y resolved from Dims= is 0. building_writer.cc treats this as a \
                    FATAL ERROR (\"Cannot create a building with zero size\") via dbg->fatal",
                fix_ja: "Dims=に0以外の正の整数（例: Dims=1,1）を指定してください",
                fix_en: "Specify a positive non-zero integer for Dims= (e.g. Dims=1,1)",
            },
            DiagnosticCode::DimsOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。Dimsのサイズが正常であることを示すだけで、\
                    問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming that Dims resolves to a \
                    valid size. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::RawCursorIcon => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。cursor/iconの\
                    生の値を示すだけで、問題を示すものではありません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    that reports the raw cursor/icon values. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::CursorIconNotApplicable => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。cursor/iconが両方とも未指定ですが、\
                    type=res/com/ind/cur/mon/tow（または obj=factory）はプレイヤーが選ぶビルドメニューに\
                    そもそも現れない種別（都市成長や特殊建造物として自動配置される）と判断できるため、\
                    missing-cursor-iconのような問題ではありません",
                why_en: "An informational message (Diagnostic::info). cursor/icon are both unspecified, \
                    but type=res/com/ind/cur/mon/tow (or obj=factory) is a category that never appears in \
                    the player-facing build menu (placed automatically by city growth or as a special \
                    building), so unlike missing-cursor-icon this is not a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::MissingCursorIcon => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "cursorとiconが両方とも未指定です。type=stop/extension/depot/dock/habour/harbour/hq\
                    （hausbauer_t::fill_menu()がstation_buildingリストから読む種別）はcursorが実在しないと\
                    ビルドツール自体が生成されず、makeobjはエラーを出さずにpak化しますが、\
                    ゲーム内のビルドメニューに一切表示されなくなります",
                why_en: "Both cursor and icon are unspecified. For type=stop/extension/depot/dock/habour/\
                    harbour/hq (the categories hausbauer_t::fill_menu() reads from the station_building \
                    list), no build tool is generated without a cursor. makeobj builds without error, but \
                    the object will never appear in the in-game build menu",
                fix_ja: "cursor=とicon=に画像参照（アイコン用128x128画像等）を指定してください",
                fix_en: "Specify image references for cursor= and icon= (e.g. a 128x128 icon image)",
            },
            DiagnosticCode::TileKeyLookup => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。各タイルの\
                    front/backimageキー探索の詳細を示すだけで、問題を示すものではありません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    detailing the front/backimage key lookup for each tile. It does not indicate a \
                    problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::MissingTileImage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "あるlayout/タイル座標についてfrontimage/backimageのいずれも定義されていません。\
                    makeobjはエラーを出さずに空画像のタイルを生成しますが、ゲーム内でそのタイルが\
                    透明（何も描画されない）になります",
                why_en: "Neither frontimage nor backimage is defined for a given layout/tile coordinate. \
                    makeobj generates an empty tile without error, but that tile renders as transparent \
                    (nothing drawn) in-game",
                fix_ja: "対象のlayout/タイル座標に frontimage[l][y][x][0][0]= または \
                    backimage[l][y][x][0][0]= を指定してください。意図的に空にする場合は\"-\"を\
                    指定してください（image-ref-empty-sentinel参照）",
                fix_en: "Specify frontimage[l][y][x][0][0]= or backimage[l][y][x][0][0]= for that layout/\
                    tile coordinate. If the tile is intentionally empty, use \"-\" (see \
                    image-ref-empty-sentinel)",
            },
            DiagnosticCode::TileImageOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。そのタイルにfront/backimageのいずれかが\
                    定義されていることを示すだけで、問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming that a front/backimage \
                    is defined for that tile. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::FrontimageHeight => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "frontimage[l][y][x][h][phase]のh（高さ添字）が0以外です。building_writer.ccは\
                    hが0のみ有効とし、それ以外はエラーログを出します（処理は継続しますが意図しない\
                    構文の可能性が高いです）",
                why_en: "The h (height index) in frontimage[l][y][x][h][phase] is non-zero. \
                    building_writer.cc only accepts h=0 and logs an error otherwise (processing \
                    continues, but this likely indicates an unintended syntax)",
                fix_ja: "frontimageのh添字は常に0にしてください（frontimage[l][y][x][0][phase]の形式）",
                fix_en: "Always use 0 for the h index of frontimage (i.e. frontimage[l][y][x][0][phase])",
            },
            DiagnosticCode::BooleanStyleFieldNotZeroOrOne => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "スタイルノート（機能的な不具合ではありません）。noinfo/noconstruction/\
                    needs_ground/extension_building/enables_pax/enables_post/enables_wareは\
                    いずれも`obj.get_int(key, 0) > 0`という比較でフラグ化されるだけなので、\
                    0/1以外の正の値（例: NoInfo=999）も1と全く同じに動作します。既に正しく\
                    動作していますが、0/1のつもりで書いた値であれば入力ミスの可能性があります",
                why_en: "A style note (not a functional bug). noinfo/noconstruction/needs_ground/\
                    extension_building/enables_pax/enables_post/enables_ware are converted to flags via \
                    `obj.get_int(key, 0) > 0`, so any positive value other than 0/1 (e.g. NoInfo=999) \
                    behaves identically to 1. This already works correctly, but if 0 or 1 was intended, \
                    it may indicate an authoring mistake",
                fix_ja: "意図を明確にするため、値を0または1に修正することを推奨します\
                    （修正しなくても動作は変わりません）",
                fix_en: "Consider changing the value to 0 or 1 to make the intent clear (behavior is \
                    unchanged either way)",
            },
            DiagnosticCode::ImageOmitted => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。citycar/pedestrianの\
                    8方向画像image[<dir>]の1方向が省略されていることを示すだけです。\
                    citycar_writer.cc/pedestrian_writer.ccはこの省略を無条件に許容し（各方向を\
                    独立に省略できる）、FATALにはなりません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    noting that one of citycar/pedestrian's 8 directional image[<dir>] entries is \
                    omitted. citycar_writer.cc/pedestrian_writer.cc unconditionally allows this (each \
                    direction can be omitted independently) and it is not FATAL",
                fix_ja: "対応不要です。意図的に省略している場合はそのままで構いません",
                fix_en: "No action needed. If the omission is intentional, leave it as-is",
            },
            DiagnosticCode::DuplicateKey => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "同じキーが複数回定義されています。makeobjのtabfileobj_t::put()は既存キーを\
                    上書きしません（先勝ち、tabfile.h:45）。つまり後から書いた値は無視され、\
                    最初に書いた値だけが使われます。意図しない値の上書き忘れの可能性が高いです",
                why_en: "The same key is defined more than once. makeobj's tabfileobj_t::put() does not \
                    overwrite existing keys (first-write-wins, tabfile.h:45), so the later value is \
                    silently ignored and only the first value takes effect. This often indicates an \
                    unintended duplicate that was meant to replace the earlier value",
                fix_ja: "重複しているキーのうち不要な方を削除するか、意図した値が最初の行に来るよう\
                    修正してください",
                fix_en: "Remove the unnecessary duplicate, or make sure the intended value is on the \
                    first occurrence of the key",
            },
            DiagnosticCode::MissingWaytype => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "waytypeフィールドが未指定（空文字列）です。get_waytype()はNULLではなく\
                    空文字列を渡されてもSTRICMPが既知13種のいずれにも一致しないためdbg->fatalで\
                    FATAL ERRORになります（way/bridge/tunnel/roadsign/vehicle/way-object/crossing/\
                    type=stop・depotのbuildingで共有される検証）",
                why_en: "The waytype field is unspecified (empty string). get_waytype() receives an \
                    empty string (not NULL) and, since STRICMP does not match any of the 13 known \
                    values, this becomes a FATAL ERROR via dbg->fatal (shared validation across way/\
                    bridge/tunnel/roadsign/vehicle/way-object/crossing/building with type=stop or depot)",
                fix_ja: "waytype=に既知の値（none/road/track/electrified_track/maglev_track/\
                    monorail_track/narrowgauge_track/water/air/schiene_tram/tram_track/power/decoration）\
                    のいずれかを指定してください",
                fix_en: "Specify a known value for waytype= (none/road/track/electrified_track/\
                    maglev_track/monorail_track/narrowgauge_track/water/air/schiene_tram/tram_track/\
                    power/decoration)",
            },
            DiagnosticCode::UnknownWaytype => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "waytypeの値がmakeobjの既知13種のいずれとも一致しません。get_waytype()は\
                    STRICMPで一致しない値をdbg->fatalでFATAL ERRORにします",
                why_en: "The waytype value does not match any of makeobj's 13 known values. get_waytype() \
                    treats a non-matching value as a FATAL ERROR via dbg->fatal",
                fix_ja: "waytypeの綴りを確認し、既知値（none/road/track/electrified_track/maglev_track/\
                    monorail_track/narrowgauge_track/water/air/schiene_tram/tram_track/power/decoration）\
                    のいずれかに修正してください",
                fix_en: "Check the spelling of waytype and correct it to one of the known values (none/\
                    road/track/electrified_track/maglev_track/monorail_track/narrowgauge_track/water/\
                    air/schiene_tram/tram_track/power/decoration)",
            },
            DiagnosticCode::WaytypeOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。waytypeの値が既知であることを示すだけで、\
                    問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming the waytype value is \
                    known. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::ImageRefEmptySentinel => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。画像参照の値が\"-\"（画像なしセンチネル）です。\
                    image_writer_t::write_objはこの値に対してファイル解決を試みず、意図的に空画像として\
                    扱います。エラーではありません",
                why_en: "An informational message (Diagnostic::info). The image reference value is \"-\" \
                    (the empty-image sentinel). image_writer_t::write_obj does not attempt file \
                    resolution for this value and treats it as an intentionally empty image. This is not \
                    an error",
                fix_ja: "対応不要です。意図的に画像を省略している場合の正常な書き方です",
                fix_en: "No action needed. This is the correct way to intentionally omit an image",
            },
            DiagnosticCode::ImageRefResolved => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。画像参照の値から\
                    解決されたファイル名・パスを示すだけで、問題を示すものではありません",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    that reports the filename/path resolved from an image reference value. It does not \
                    indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::MissingImageFile => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "画像参照が解決したファイルが.datと同じディレクトリに見つかりません。\
                    image_writer_t::write_objはファイルを開けないとpak生成全体を例外で中断させます",
                why_en: "The file resolved from an image reference was not found next to the .dat file. \
                    image_writer_t::write_obj throws an exception when it cannot open the file, \
                    aborting pak generation entirely",
                fix_ja: "参照しているファイル名・拡張子・配置ディレクトリを確認し、実在するPNGファイルを\
                    指すよう修正してください",
                fix_en: "Check the referenced filename, extension, and directory, and make sure it \
                    points to an existing PNG file",
            },
            DiagnosticCode::ImageSizeNotMultipleOf128 => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "参照画像の幅または高さが画像タイルサイズ（既定128、`.dat`の`cell_size=`や\
                    `dat_linter.toml`の`[tile_size]`・`--tile-size`で変更可能）の倍数ではありません。\
                    image_writer.ccのblock_load()は幅/高さがimg_sizeの倍数でないと読み込み失敗を返し、\
                    write_obj側がobj_pak_exception_tをthrowしてpak生成全体を中断させます",
                why_en: "The referenced image's width or height is not a multiple of the image tile \
                    size (128 by default; configurable via the .dat's cell_size=, dat_linter.toml's \
                    [tile_size], or --tile-size). image_writer.cc's block_load() fails to load images \
                    whose width/height is not a multiple of img_size, and write_obj throws \
                    obj_pak_exception_t, aborting pak generation entirely",
                fix_ja: "画像をタイルサイズ単位（例: 128の場合は128, 256, 384...）にリサイズ・\
                    パディングしてください",
                fix_en: "Resize or pad the image to a multiple of the tile size (e.g. 128, 256, 384, \
                    ... for a 128 tile size)",
            },
            DiagnosticCode::ImageCoordinateOutOfBounds => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "画像参照で指定された行/列（例: \"foo.334.0\"のrow=334, col=0）が、実際の\
                    画像ファイルのタイル数（幅/img_size, 高さ/img_size）を超えています。\
                    image_writer.ccのwrite_objは`col >= width/img_size || row >= height/img_size`\
                    のとき\"invalid image number in ...\"としてobj_pak_exception_tをthrowし、\
                    pak生成全体を中断させます。1引数省略形（\"foo.334\"のようにcolを省略した形式）でも、\
                    実際のタイル数を使ってrow/colへ展開し直した後の値で同じ判定が行われます",
                why_en: "The row/column specified in an image reference (e.g. row=334, col=0 in \
                    \"foo.334.0\") exceeds the image file's actual tile grid (width/img_size, \
                    height/img_size). image_writer.cc's write_obj throws obj_pak_exception_t \
                    (\"invalid image number in ...\") when `col >= width/img_size || \
                    row >= height/img_size`, aborting pak generation entirely. The same check applies \
                    to the 1-argument shorthand form (e.g. \"foo.334\", omitting col), after it is \
                    expanded into row/col using the image's actual tile count",
                fix_ja: "参照している行/列番号を確認し、画像の実際のタイルグリッド範囲内\
                    （0..幅/img_size, 0..高さ/img_size）に収まるよう修正するか、画像をより大きな\
                    タイル数を持つファイルに差し替えてください",
                fix_en: "Check the referenced row/column number and correct it to fit within the \
                    image's actual tile grid (0..width/img_size, 0..height/img_size), or replace the \
                    image with one that has enough tiles",
            },
            DiagnosticCode::ImageOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。参照画像が存在し、サイズも画像タイルサイズの\
                    倍数であることを示すだけで、問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming that the referenced \
                    image exists and its size is a multiple of the image tile size. It does not \
                    indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::UnreadableImage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "参照ファイルは存在しますが、画像として読み込めません（破損している、\
                    PNG以外の形式である等）。image_writer_t::write_objは読み込み失敗時に\
                    例外をthrowしてpak生成全体を中断させます",
                why_en: "The referenced file exists but cannot be read as an image (corrupted, not a \
                    valid PNG, etc.). image_writer_t::write_obj throws an exception on read failure, \
                    aborting pak generation entirely",
                fix_ja: "ファイルが正しいPNG形式であること・破損していないことを確認してください",
                fix_en: "Verify the file is a valid, uncorrupted PNG",
            },
            DiagnosticCode::DateIndexOverflow => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "静的解析ルール（makeobjの`dbg->fatal`/`dbg->warning`を直接ミラーするものでは\
                    ない。PowerGearMismatch/FactoryProductivityZeroと同種）。intro_year/intro_month\
                    （またはretire_year/retire_month、buildingのみpreservation_year/\
                    preservation_monthも対象）から`year*12+month-1`で計算される日付インデックスが\
                    格納先のuint16の範囲(0..65535)外です。makeobj（building/citycar/crossing/\
                    pedestrian/roadsign/tunnel/vehicle/way/way-objectの各writer）はこの計算結果を\
                    無条件にuint16へ代入するため、範囲外の値は2の補数による切り詰めで\
                    無関係な日付へ静かにラップアラウンドします。makeobj自体はこれを検証せず、\
                    警告も出しません（bridgeは`get_int_clamped`で既に緩和されているため対象外）",
                why_en: "A static-analysis rule (not a direct mirror of makeobj's dbg->fatal/dbg->warning; \
                    same category as PowerGearMismatch/FactoryProductivityZero). The date index computed \
                    as `year*12+month-1` from intro_year/intro_month (or retire_year/retire_month; \
                    building also has preservation_year/preservation_month) is outside the uint16 range \
                    (0..65535) it is stored in. makeobj (the building/citycar/crossing/pedestrian/\
                    roadsign/tunnel/vehicle/way/way-object writers) unconditionally assigns this result \
                    to a uint16, so an out-of-range value silently wraps around (two's-complement \
                    truncation) into an unrelated bogus date. makeobj itself does not validate this and \
                    emits no warning (bridge is excluded because it already mitigates this via \
                    get_int_clamped)",
                fix_ja: "year/monthの値を確認し、year*12+month-1が0..65535の範囲に収まるよう\
                    修正してください（monthは通常1..12、負のyearや極端に大きいyearを指定していないか\
                    確認してください）",
                fix_en: "Check the year/month values and adjust them so that year*12+month-1 falls \
                    within 0..65535 (month is normally 1..12; check for a negative year or an \
                    excessively large year)",
            },
            DiagnosticCode::NameForbiddenFilenameCharacter => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "nameの値に、Windows/Unixのファイル名として使えない文字\
                    （\\ / : * ? \" < > | や制御文字）・Windows予約デバイス名\
                    （CON/PRN/AUX/NUL/COM1-9/LPT1-9、拡張子を除いた完全一致）・\
                    末尾のドットや空白が含まれています。root_writer_t::write()の\
                    separate出力（root_writer.cc:89、`obj.get(\"obj\")+\".\"+obj.get(\"name\")+\
                    \".pak\"`を`fopen`）と、root_writer_t::uncopy()（マージ済みpakを\
                    個別ファイルへ分割する操作、root_writer.cc:467、`writer+\".\"+node_name+\
                    \".pak\"`を`fopen`）の2箇所が、`name=`の値をサニタイズせず\
                    そのままファイルパスへ組み込みます。該当文字を含むとこれらの\
                    `fopen`が失敗し、`write()`側は紛らわしいことに実際に失敗した\
                    パスではなく元のCLI出力先引数をエラーメッセージに表示するため、\
                    ビルド/分割の失敗原因が非常に分かりにくくなります",
                why_en: "The value of name= contains characters not allowed in Windows/Unix \
                    filenames (\\ / : * ? \" < > | or control characters), a reserved Windows \
                    device name (CON/PRN/AUX/NUL/COM1-9/LPT1-9, exact match ignoring any \
                    extension), or a trailing dot/space. Two places in root_writer_t build a \
                    filesystem path directly from name= without sanitizing it and then fopen() \
                    it: the separate-file output mode of write() (root_writer.cc:89, \
                    `obj.get(\"obj\")+\".\"+obj.get(\"name\")+\".pak\"`), and uncopy() (splitting \
                    a previously-merged pak back into per-object files, root_writer.cc:467, \
                    `writer+\".\"+node_name+\".pak\"`). When name= contains such a character, \
                    these fopen() calls fail, and confusingly, write()'s error message reports \
                    the original CLI output-directory argument rather than the actual failed \
                    per-object path, making build/split failures very hard to diagnose",
                fix_ja: "name=の値から該当文字・予約デバイス名・末尾のドット/空白を\
                    取り除いてください",
                fix_en: "Remove the offending character, reserved device name, or trailing \
                    dot/space from the value of name=",
            },
            DiagnosticCode::NarrowIntOverflow => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "静的解析ルール（makeobjの`dbg->fatal`/`dbg->warning`を直接ミラーする\
                    ものではない。DateIndexOverflow/PowerGearMismatchと同種）。この数値\
                    フィールドは`tabfileobj_t::get_int()`/`get_int64()`（範囲チェック無しの\
                    無条件フォールバック）で読まれた後、その値より狭いC++整数型\
                    （`node.write_uint8`/`write_uint16`等）へ無条件に代入されて書き込まれます。\
                    範囲外の値を指定すると、C++の暗黙変換（2の補数での切り詰め）によって\
                    指定した値と全く異なる値が静かにpakへ書き込まれます。makeobj自体はこれを\
                    検証しません",
                why_en: "A static-analysis rule (not a direct mirror of makeobj's dbg->fatal/\
                    dbg->warning; same category as DateIndexOverflow/PowerGearMismatch). This \
                    numeric field is read via tabfileobj_t::get_int()/get_int64() (an \
                    unconditional fallback with no range check), then unconditionally assigned \
                    to a narrower C++ integer type (node.write_uint8/write_uint16/etc.) at the \
                    point it is written. An out-of-range value is silently truncated by C++'s \
                    implicit conversion (two's-complement narrowing), so a value completely \
                    different from what was specified is written to the pak with no warning. \
                    makeobj itself does not validate this",
                fix_ja: "指摘された値の範囲を確認し、格納先の整数型が表現できる範囲内\
                    （警告文中のbit幅・符号から算出される範囲）に収まるよう値を修正してください",
                fix_en: "Check the reported value range and adjust it to fit within what the \
                    storage integer type can represent (the range implied by the bit width and \
                    signedness shown in the warning)",
            },
            DiagnosticCode::EmbeddedNulInStringField => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "name/copyrightフィールドの値にNULバイト(\\0)が含まれています。\
                    obj_writer_t::write_name_and_copyright経由で呼ばれるtext_writer_t::write_obj\
                    （text_writer.cc:18、`strlen(text)`）は文字列長をNULバイトで判定するC文字列\
                    処理のため、NULバイトより後ろの内容が警告無く静かに切り捨てられます",
                why_en: "The value of the name/copyright field contains an embedded NUL byte (\\0). \
                    text_writer_t::write_obj (text_writer.cc:18, `strlen(text)`), called via \
                    obj_writer_t::write_name_and_copyright, determines the string length using C \
                    string semantics based on the NUL byte, so any content after the NUL byte is \
                    silently truncated with no warning",
                fix_ja: "name/copyrightの値からNULバイトを取り除いてください（テキストエディタで\
                    意図せず紛れ込んだ制御文字である可能性が高いです）",
                fix_en: "Remove the NUL byte from the name/copyright value (it is likely an \
                    accidentally embedded control character from a text editor)",
            },
            DiagnosticCode::UnknownSkinName => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "obj=symbol/obj=miscのトップレベルname=が、ゲーム本体が認識する既知の\
                    特殊スキン名のいずれとも一致しません。skinverwaltung_t::register_desc()\
                    （simskin.cc:195-227）はこの場合\"Spurious object '%s' loaded (will not be \
                    referenced anyway)!\"というdbg->warningを出します（obj=cursor/obj=menuは\
                    一致しなくてもextra_cursor_obj/extra_menu_objへ登録されるだけで警告が\
                    出ないため対象外）。pak自体は正常に読み込まれますが、このオブジェクトは\
                    ゲームのどのロジックからも一切参照されません",
                why_en: "The top-level name= of an obj=symbol/obj=misc entry does not match any \
                    of the known special-purpose skin names recognized by the game. \
                    skinverwaltung_t::register_desc() (simskin.cc:195-227) emits the warning \
                    \"Spurious object '%s' loaded (will not be referenced anyway)!\" in this case \
                    (obj=cursor/obj=menu are excluded because a non-matching name is simply \
                    registered into extra_cursor_obj/extra_menu_obj with no warning). The pak \
                    still loads successfully, but this object is never referenced by any game \
                    logic",
                fix_ja: "name=の綴りを確認し、意図した特殊スキン名（`dat_linter describe`や\
                    エディタの補完候補一覧を参照）と完全に一致させてください。単に独自の名前で\
                    画像を管理したい場合は、その画像がどのゲーム機能にも紐づかないことを\
                    理解した上で使ってください",
                fix_en: "Check the spelling of name= and make it match one of the intended \
                    special-purpose skin names exactly (see `dat_linter describe` or your \
                    editor's completion suggestions). If you intend to manage the image under \
                    your own arbitrary name, be aware that it will not be tied to any game \
                    feature",
            },
            DiagnosticCode::CrossingIdenticalWaytypes => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "waytype[0]とwaytype[1]が解決後の値として同一です（別名同士、例えば\
                    schiene_tramとtram_trackも含む）。crossing_writer.ccは\
                    \"Identical ways (...) cannot cross (check waytypes)!\"としてdbg->fatalで\
                    FATAL ERRORにします",
                why_en: "waytype[0] and waytype[1] resolve to the same way type (including aliases such \
                    as schiene_tram and tram_track). crossing_writer.cc treats this as a FATAL ERROR \
                    (\"Identical ways (...) cannot cross (check waytypes)!\") via dbg->fatal",
                fix_ja: "waytype[0]とwaytype[1]に異なる種類のwayを指定してください",
                fix_en: "Specify two different way types for waytype[0] and waytype[1]",
            },
            DiagnosticCode::CrossingMissingSpeed => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "speed[0]とspeed[1]のどちらか一方でも0（未指定含む）です。\
                    crossing_writer.ccは\"A maxspeed MUST be given for both ways!\"として\
                    dbg->fatalでFATAL ERRORにします",
                why_en: "Either speed[0] or speed[1] is 0 (including unspecified). crossing_writer.cc \
                    treats this as a FATAL ERROR (\"A maxspeed MUST be given for both ways!\") via \
                    dbg->fatal",
                fix_ja: "speed[0]とspeed[1]の両方に0以外の値（最高速度）を指定してください",
                fix_en: "Specify a non-zero value (max speed) for both speed[0] and speed[1]",
            },
            DiagnosticCode::CrossingMissingOpenimage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "openimage[ns][0]とopenimage[ew][0]のどちらか一方でも未指定（0枚）です。\
                    crossing_writer.ccのコメント\"these must exists!\"の通り両方とも最低1枚必須で、\
                    片方でも0枚だと\"Missing images (at least one openimage!...)\"として\
                    dbg->fatalでFATAL ERRORにします",
                why_en: "Either openimage[ns][0] or openimage[ew][0] is unspecified (0 images). As \
                    crossing_writer.cc's comment \"these must exists!\" indicates, both require at \
                    least one image; if either has 0, this becomes a FATAL ERROR (\"Missing images (at \
                    least one openimage!...)\") via dbg->fatal",
                fix_ja: "openimage[ns][0]とopenimage[ew][0]の両方に最低1枚ずつ画像を指定してください",
                fix_en: "Specify at least one image for both openimage[ns][0] and openimage[ew][0]",
            },
            DiagnosticCode::FactoryTypeOverride => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "obj=factoryでtypeが明示的に指定されています。factory_writer.ccの\
                    obj.put(\"type\",\"fac\")はtabfileobj_t::put()の先勝ち仕様により既存のtypeキーを\
                    上書きできません。building_writer_t::write_objは明示された値のまま分岐するため、\
                    obsolete型ならFATAL ERROR、fac以外の既知型（res/com/ind等）ならfactoryとして\
                    機能しない建物が黙って生成されます",
                why_en: "type= is explicitly set for obj=factory. factory_writer.cc's \
                    obj.put(\"type\",\"fac\") cannot overwrite an existing type key due to \
                    tabfileobj_t::put()'s first-write-wins behavior. building_writer_t::write_obj then \
                    branches on the explicit value, so an obsolete type becomes a FATAL ERROR, and any \
                    other known type (res/com/ind, etc.) silently produces a building that does not \
                    function as a factory",
                fix_ja: "obj=factoryのtypeキーを削除してください（factory_writer.ccが自動的にfacへ\
                    設定します）",
                fix_en: "Remove the type key from obj=factory (factory_writer.cc sets it to fac \
                    automatically)",
            },
            DiagnosticCode::FactoryMissingMapcolor => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "mapcolorが未指定（または255）です。factory_writer.ccはmapcolorがデフォルト値\
                    255のままだと\"missing an identification color! (mapcolor)\"として\
                    dbg->fatalでFATAL ERRORにします",
                why_en: "mapcolor is unspecified (or 255). factory_writer.cc treats mapcolor staying at \
                    the default 255 as a FATAL ERROR (\"missing an identification color! (mapcolor)\") \
                    via dbg->fatal",
                fix_ja: "mapcolor=に255以外の値（マップ上での識別色）を指定してください",
                fix_en: "Specify a value other than 255 for mapcolor= (the identification color shown on \
                    the map)",
            },
            DiagnosticCode::FactoryMapcolorOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。mapcolorが255以外の値に設定されていることを\
                    示すだけで、問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming mapcolor is set to a \
                    value other than 255. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::FactoryOutputCapacityTooSmall => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "outputcapacity[N]が11未満です。factory_writer.ccは\
                    \"Factory outputcapacity must be larger than 10!\"としてエラーログを出します\
                    （dbg->error、非FATAL。処理は継続しますが意図しない値の可能性が高いです）",
                why_en: "outputcapacity[N] is less than 11. factory_writer.cc logs an error (\"Factory \
                    outputcapacity must be larger than 10!\") via dbg->error (non-FATAL; processing \
                    continues, but this likely indicates an unintended value)",
                fix_ja: "outputcapacity[N]に11以上の値を指定してください",
                fix_en: "Specify a value of 11 or greater for outputcapacity[N]",
            },
            DiagnosticCode::FactorySmoketileWithoutOffset => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "smoketile[N]（インデックス形式）が定義されているのに対応するsmokeoffset[N]が\
                    未指定です。factory_writer.ccは\"... defined but not ...!\"としてエラーログを\
                    出します（dbg->error、非FATAL）",
                why_en: "smoketile[N] (indexed form) is defined but the corresponding smokeoffset[N] is \
                    missing. factory_writer.cc logs an error (\"... defined but not ...!\") via \
                    dbg->error (non-FATAL)",
                fix_ja: "対応するsmokeoffset[N]（煙のオフセット座標）を指定してください",
                fix_en: "Specify the corresponding smokeoffset[N] (the smoke's offset coordinates)",
            },
            DiagnosticCode::FactoryProbabilityClamped => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "probability_to_spawnまたはexpand_probabilityが10000以上です。\
                    factory_writer.ccはこの値を\"too large, set to 10,000\"というメッセージを出力した上で\
                    サイレントに10000へクランプします",
                why_en: "probability_to_spawn or expand_probability is 10000 or greater. factory_writer.\
                    cc prints \"too large, set to 10,000\" and silently clamps the value to 10000",
                fix_ja: "値を10000未満に修正してください",
                fix_en: "Set the value below 10000",
            },
            DiagnosticCode::FactoryProductivityZero => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "productivity=0です。makeobj自体はこの値を検証しませんが、ゲームランタイム\
                    （simfab.cc）はfactory配置時に無条件でupdate_scaled_pax_demand()/\
                    update_scaled_mail_demand()を呼び、productivityを分母とした整数除算を行います。\
                    この値がゼロだとゼロ除算（未定義動作、通常はクラッシュ）になります",
                why_en: "productivity=0. makeobj itself does not validate this value, but the game \
                    runtime (simfab.cc) unconditionally calls update_scaled_pax_demand()/\
                    update_scaled_mail_demand() when a factory is placed, dividing by productivity. If \
                    this value is zero, that becomes a division by zero (undefined behavior, usually a \
                    crash)",
                fix_ja: "productivityに1以上の値を指定してください",
                fix_en: "Specify a value of 1 or greater for productivity",
            },
            DiagnosticCode::WaytypeOmitted => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。obj=ground_objではwaytypeが未指定でも\
                    ignore_wtにサイレントフォールバックしFATALになりません（他の大半のobj種別と異なる\
                    非対称な挙動）。問題ではありません",
                why_en: "An informational message (Diagnostic::info). For obj=ground_obj, an \
                    unspecified waytype silently falls back to ignore_wt and does not cause a FATAL \
                    ERROR (unlike most other obj types). This is not a problem",
                fix_ja: "対応不要です。特定waytype専用にしたい場合のみwaytype=を指定してください",
                fix_en: "No action needed. Specify waytype= only if you want it restricted to a \
                    particular waytype",
            },
            DiagnosticCode::MissingSeasonImage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "季節ごとの画像（image[<phase>][<season>]または移動物のimage[<dir>][<season>]）が\
                    一部欠けています。groundobj_writer.ccは season 0が定義済みのphaseで後続seasonが\
                    欠けている場合（固定物）、または移動物で8方向×全seasonのいずれかが欠けている場合、\
                    \"Season image for season N missing!\"としてdbg->fatalでFATAL ERRORにします",
                why_en: "A per-season image (image[<phase>][<season>] for fixed objects, or \
                    image[<dir>][<season>] for moving objects) is missing. groundobj_writer.cc treats a \
                    missing later-season image (when season 0 is defined, for fixed objects) or any \
                    missing image among the 8 directions x all seasons (for moving objects) as a FATAL \
                    ERROR (\"Season image for season N missing!\") via dbg->fatal",
                fix_ja: "そのseasonの画像を指定するか、seasons=を減らして必要な季節数を減らしてください",
                fix_en: "Specify the missing season's image, or reduce seasons= to require fewer season \
                    images",
            },
            DiagnosticCode::NoImages => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。固定物（speed=0）のimage[0][0]が未指定です。\
                    groundobj_writer.ccはこれをFATALにしません（画像0枚のground_objも許容されますが、\
                    ゲーム内では何も描画されません）",
                why_en: "An informational message (Diagnostic::info). image[0][0] is unspecified for a \
                    fixed object (speed=0). groundobj_writer.cc does not treat this as FATAL (a \
                    ground_obj with zero images is allowed, but nothing renders in-game)",
                fix_ja: "描画したい場合はimage[0][0]=に画像を指定してください。意図的に無描画の\
                    オブジェクトにする場合は対応不要です",
                fix_en: "If you want it to render, specify an image for image[0][0]=. If an invisible \
                    object is intentional, no action is needed",
            },
            DiagnosticCode::RoadsignImageCountNotMultipleOf4 => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "numbered構文（image[0]が非空）で、画像枚数が4の倍数ではありません。\
                    roadsign_writer.ccは\"image count is N but must be multiple of 4!\"として\
                    dbg->fatalでFATAL ERRORにします",
                why_en: "In the numbered syntax (image[0] is present), the image count is not a \
                    multiple of 4. roadsign_writer.cc treats this as a FATAL ERROR (\"image count is N \
                    but must be multiple of 4!\") via dbg->fatal",
                fix_ja: "image[N]の連番を4の倍数枚（4, 8, 12...）になるよう追加または削除してください",
                fix_en: "Add or remove image[N] entries so the total count is a multiple of 4 (4, 8, \
                    12, ...)",
            },
            DiagnosticCode::RoadsignImageMissing => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "2D構文（image[0]が空）で、state=0の全方向（および私有地標識ならstate=1も）の\
                    画像が必須ですが、途中の方向だけ欠けています。roadsign_writer.ccは\
                    \"... is missing!\"としてdbg->fatalでFATAL ERRORにします",
                why_en: "In the 2D syntax (image[0] is empty), all directions for state=0 (and state=1 \
                    for private-road signs) are required, but one is missing partway through. \
                    roadsign_writer.cc treats this as a FATAL ERROR (\"... is missing!\") via dbg->fatal",
                fix_ja: "そのstate・方向の画像image[<dir>][<state>]=を指定してください",
                fix_en: "Specify the image for that state/direction: image[<dir>][<state>]=",
            },
            DiagnosticCode::MissingAgeSeasonImage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "age（0..4固定5段階）×season（0..seasons-1）の画像image[<age>][<season>]の\
                    いずれかが未指定です。tree_writer.ccは全組み合わせを無条件に必須とし、\
                    1つでも欠けると\"Missing ...!\"としてdbg->fatalでFATAL ERRORにします",
                why_en: "One of the image[<age>][<season>] entries (age 0..4 fixed, season 0..seasons-1) \
                    is missing. tree_writer.cc unconditionally requires every combination; if even one \
                    is missing, this becomes a FATAL ERROR (\"Missing ...!\") via dbg->fatal",
                fix_ja: "全age(0-4)×全season(0..seasons-1)の組み合わせについてimage[<age>][<season>]=を\
                    指定してください",
                fix_en: "Specify image[<age>][<season>]= for every combination of age (0-4) and season \
                    (0..seasons-1)",
            },
            DiagnosticCode::EngineTypeSkipped => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。\
                    waytype=electrified_trackのためengine_typeが無条件にelectricとして扱われ、\
                    実際の値は読まれないことを示すだけです",
                why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
                    noting that since waytype=electrified_track, engine_type is unconditionally treated \
                    as electric and the actual value is not read",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::UnknownEngineType => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "engine_typeが既知値（diesel/electric/steam/bio/sail/fuel_cell/hydrogene/battery/\
                    unknown）以外の値です。vehicle_writer.ccのget_engine_type()はfatal/errorを出さず、\
                    黙ってdieselにフォールバックします。typoの可能性が高いです\
                    （engine_typeが完全に未指定の場合は無動力車両の慣習として警告対象外）",
                why_en: "engine_type is not one of the known values (diesel/electric/steam/bio/sail/\
                    fuel_cell/hydrogene/battery/unknown). vehicle_writer.cc's get_engine_type() does not \
                    emit fatal/error, but silently falls back to diesel. This likely indicates a typo \
                    (a completely unspecified engine_type is exempt, as it is a common convention for \
                    unpowered vehicles such as freight cars)",
                fix_ja: "engine_typeの綴りを確認し、既知値のいずれかに修正するか、無動力車両であれば\
                    キー自体を削除してください",
                fix_en: "Check the spelling of engine_type and correct it to a known value, or remove \
                    the key entirely if the vehicle is unpowered",
            },
            DiagnosticCode::Incomplete8DirectionImages => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "emptyimage[<dir>]（8方向）のうちn/e/ne/nwのいずれかが定義されているのに、\
                    連続して定義された方向の数が8未満です。vehicle_writer.ccは8方向全てが揃っているか、\
                    4方向以下で止まっているかのどちらかを要求し、それ以外はFATAL ERRORにします",
                why_en: "One of the n/e/ne/nw direction images (emptyimage[<dir>]) is defined, but fewer \
                    than 8 consecutive directions are defined overall. vehicle_writer.cc requires either \
                    all 8 directions or stopping at 4 or fewer; anything else is a FATAL ERROR",
                fix_ja: "8方向全てのemptyimage[<dir>]を定義するか、4方向（s/w/sw/se）以下で止めてください",
                fix_en: "Define emptyimage[<dir>] for all 8 directions, or stop at 4 or fewer (s/w/sw/se)",
            },
            DiagnosticCode::FreightimageCountMismatch => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "非indexedのfreightimage[<dir>]の個数がemptyimageの個数と一致しません。\
                    vehicle_writer.ccは両者が完全一致することを要求し、不一致はFATAL ERRORにします",
                why_en: "The count of non-indexed freightimage[<dir>] entries does not match the count \
                    of emptyimage entries. vehicle_writer.cc requires these to match exactly; a mismatch \
                    is a FATAL ERROR",
                fix_ja: "freightimage[<dir>]の個数がemptyimageの個数と一致するよう追加・削除してください",
                fix_en: "Add or remove freightimage[<dir>] entries so the count matches emptyimage",
            },
            DiagnosticCode::MissingIndexedFreightimage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "indexed形式（freightimage[0][s]が定義済み）で、emptyimageが定義された方向×\
                    freight typeの組み合わせのfreightimage[N][<dir>]が欠けています。vehicle_writer.ccは\
                    この欠落をFATAL ERRORにします",
                why_en: "In indexed form (freightimage[0][s] is defined), freightimage[N][<dir>] is \
                    missing for a direction (where emptyimage is defined) x freight-type combination. \
                    vehicle_writer.cc treats this as a FATAL ERROR",
                fix_ja: "全ての方向×freight typeの組み合わせについてfreightimage[N][<dir>]=を\
                    指定してください",
                fix_en: "Specify freightimage[N][<dir>]= for every direction x freight-type combination",
            },
            DiagnosticCode::MissingFreightimagetype => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "freight_image_type個のindexed freightimageが使われているのに、対応する\
                    freightimagetype[i]（goodへのxref）が欠けています。vehicle_writer.ccはこの欠落を\
                    FATAL ERRORにします",
                why_en: "Indexed freightimage entries are in use (freight_image_type entries), but the \
                    corresponding freightimagetype[i] (an xref to a good) is missing. vehicle_writer.cc \
                    treats this as a FATAL ERROR",
                fix_ja: "各indexに対応するfreightimagetype[i]=に貨物種別（good）を指定してください",
                fix_en: "Specify the good (freight type) for freightimagetype[i]= at each index",
            },
            DiagnosticCode::ExtraFreightimagetype => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "freightimagetype[N]が使用範囲（0..freight_image_type）より1つ多いindexで\
                    定義されています。makeobjはFATALにはしませんが警告を出します（超過定義）",
                why_en: "freightimagetype[N] is defined one index beyond the used range \
                    (0..freight_image_type). makeobj does not treat this as FATAL, but warns about the \
                    excess definition",
                fix_ja: "使用していない超過分のfreightimagetype[N]を削除してください",
                fix_en: "Remove the unused excess freightimagetype[N] entry",
            },
            DiagnosticCode::PowerGearMismatch => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "静的解析ルール（makeobjではなくゲームランタイム simconvoi.cc が根拠）。\
                    power>0を宣言していますが、gear（変換後 gear*64/100）が0になるため、\
                    編成内でのこの車両の実効出力寄与が常に0になります。makeobj自体はこの組み合わせを\
                    検証しません",
                why_en: "A static-analysis rule (based on the game runtime simconvoi.cc, not makeobj). \
                    power>0 is declared, but gear (after conversion, gear*64/100) becomes 0, so this \
                    vehicle's effective power contribution in a convoy is always 0. makeobj itself does \
                    not validate this combination",
                fix_ja: "gearの値を大きくする（2以上でgear*64/100が非ゼロになります）か、\
                    意図的に無出力車両にする場合はpowerを0にしてください",
                fix_en: "Increase gear (2 or more makes gear*64/100 non-zero), or set power to 0 if a \
                    non-powered vehicle is intended",
            },
            DiagnosticCode::MissingBaseImage => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "image[-]（直進画像）とimage[-][0]（冬季season 0版）の両方が未指定です。\
                    way_writer.ccはどちらか一方でも定義されていれば良いとしますが、両方欠落している場合は\
                    \"image with label image[-] missing\"としてdbg->fatalでFATAL ERRORにします",
                why_en: "Both image[-] (straight-track image) and image[-][0] (winter season 0 variant) \
                    are unspecified. way_writer.cc accepts either one being defined, but if both are \
                    missing, this becomes a FATAL ERROR (\"image with label image[-] missing\") via \
                    dbg->fatal",
                fix_ja: "image[-]=（直進画像）またはimage[-][0]=（冬季版）のいずれかに画像を\
                    指定してください",
                fix_en: "Specify an image for either image[-]= (straight-track image) or image[-][0]= \
                    (winter variant)",
            },
            DiagnosticCode::BaseImageOk => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "情報表示です（Diagnostic::info）。image[-]またはimage[-][0]のいずれかが\
                    定義されていることを示すだけで、問題ではありません",
                why_en: "An informational message (Diagnostic::info) confirming either image[-] or \
                    image[-][0] is defined. It does not indicate a problem",
                fix_ja: "対応不要です",
                fix_en: "No action needed",
            },
            DiagnosticCode::ClipBelowOutOfRange => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "clip_belowが0/1以外の値です。way_writer.ccはobj.get_int_clamped(\"clip_below\", \
                    1, 0, 1)を呼ぶため、範囲外の値はdbg->warningを出した上で黙って0か1にクランプされます",
                why_en: "clip_below is not 0 or 1. way_writer.cc calls obj.get_int_clamped(\"clip_below\",\
                     1, 0, 1), so an out-of-range value is warned about and silently clamped to 0 or 1",
                fix_ja: "clip_belowに0または1を指定してください",
                fix_en: "Specify 0 or 1 for clip_below",
            },
            DiagnosticCode::FmtReorderApplied => CodeInfo {
                code: *self,
                source: CodeSource::Fmt,
                // 第11弾: 専用の[fmt] reorder設定を廃止し、reorder機能の有効/無効自体を
                // このcodeで表現する（[rules] include/excludeの仕組みに統合）。
                // 第12弾: 当初は実際にreorderを適用するたびDiagnostic::info(...)を発行して
                // いたが、これにより問題の無い通常のfmt実行が毎回1行stderrへ出力される
                // 副作用があった（「指摘が無ければ完全silent」というlint/analyzeと同じ方針に
                // 反する）ため撤回した。このcodeは診断メッセージとして表示されることは無く、
                // `config.is_enabled(DiagnosticCode::FmtReorderApplied)`という設定判定の
                // ためだけに存在する（`dat_linter list`/`describe`から参照できるように
                // するための純粋な設定キー名としての登録）。
                why_ja: "fmtが慣習的な順序へキーを並び替える機能そのものを表すcodeです。\
                    診断メッセージとして表示されることはありません。デフォルトで有効\
                    （`--no-reorder`未指定・このcodeが`[rules] exclude`に無い場合）で、\
                    `[rules] include/exclude`を通じてreorder機能自体の有効/無効を\
                    切り替えるためだけに使う設定キー名です",
                why_en: "This code represents fmt's key-reordering feature itself. It is never shown \
                    as a diagnostic message. It is enabled by default (unless --no-reorder is passed or \
                    this code is listed in [rules] exclude), and exists purely as a setting-key name used \
                    via [rules] include/exclude to toggle the reordering feature on or off",
                fix_ja: "恒久的に無効化したい場合は`[rules] exclude`にこのcode\
                    （\"fmt-reorder-applied\"）を追加してください。1回の実行だけ無効化したい場合は\
                    `--no-reorder`フラグを使ってください（`--no-reorder`はconfig設定より常に優先されます）。\
                    このcode自体を「修正」する必要はありません（診断ではないため）",
                fix_en: "To permanently disable reordering, add this code (\"fmt-reorder-applied\") to \
                    [rules] exclude. To disable it for a single invocation only, use the --no-reorder \
                    flag (--no-reorder always takes priority over the config setting). There is nothing \
                    to \"fix\" about this code itself (it is not a diagnostic)",
            },
            DiagnosticCode::FmtLeadingSpaceLine => CodeInfo {
                code: *self,
                source: CodeSource::Fmt,
                why_ja: "行頭がスペースで始まっています。実際のmakeobjのtabfile_t::read_line()は\
                    `*dest=='#' || *dest==' '`の間スキップし続けるため、この行はコメントとして\
                    無視され、key=valueとして読み込まれません",
                why_en: "The line starts with a space. makeobj's tabfile_t::read_line() skips while \
                    `*dest=='#' || *dest==' '`, so this line is treated as a comment and ignored — it is \
                    never read as a key=value pair",
                fix_ja: "行頭のスペースを削除してください（または、コメントとして意図している場合は\
                    `#`で始めてください）",
                fix_en: "Remove the leading space (or start the line with `#` if a comment is intended)",
            },
            DiagnosticCode::FmtMalformedLine => CodeInfo {
                code: *self,
                source: CodeSource::Fmt,
                why_ja: "行に`=`が含まれていません（区切り行`-`・コメント`#`・行頭スペース行を除く）。\
                    makeobjはこの行を\"No data in ...\"としてdbg->warningを出した上で無視します",
                why_en: "The line contains no `=` (excluding separator lines starting with `-`, comments \
                    starting with `#`, and leading-space lines). makeobj warns \"No data in ...\" and \
                    ignores this line",
                fix_ja: "key=value形式に修正するか、意図しない行であれば削除してください",
                fix_en: "Fix the line to key=value form, or remove it if it was not intended",
            },
            DiagnosticCode::FmtReorderUnsupportedObj => CodeInfo {
                code: *self,
                source: CodeSource::Fmt,
                why_ja: "`--reorder`（デフォルト有効）が、このobj=の値に対応する並び順仕様を\
                    持っていません。dat_linter自体の制約であり、makeobjのエラーではありません。\
                    並び替えを行わず元の行順のまま出力します",
                why_en: "`--reorder` (enabled by default) has no ordering spec registered for this \
                    obj= value. This is a limitation of dat_linter itself, not a makeobj error. Output \
                    uses the original line order without reordering",
                fix_ja: "対応不要です（`--no-reorder`を指定すればこの警告自体を出さずに常に元の順序を\
                    保持できます）。このobj種別の並び替えテーブルを追加したい場合は\
                    src/formatter/order.rsの拡張が必要です",
                fix_en: "No action needed (passing `--no-reorder` suppresses this warning entirely by \
                    always preserving the original order). To add reordering support for this obj type, \
                    extend src/formatter/order.rs",
            },
            DiagnosticCode::FmtReorderLinesDropped => CodeInfo {
                code: *self,
                source: CodeSource::Fmt,
                why_ja: "`--reorder`実行時、コメント/行頭スペース行/不正行の一部が、並び替え後の\
                    出力上で一意な位置に紐づけられないため出力から削除されました\
                    （直後のkey=value行に紐づくコメントは保持されますが、紐づけ先が無いものは\
                    削除対象になります）",
                why_en: "During `--reorder`, some comment/leading-space/malformed lines could not be \
                    tied to a well-defined position in the reordered output and were dropped (a comment \
                    immediately followed by a key=value line is preserved and moves with it, but ones \
                    with no such anchor are dropped)",
                fix_ja: "削除されたくないコメント等がある場合は、`--no-reorder`を使うか、\
                    コメントを対応するkey=value行の直前に移動してから`--reorder`してください",
                fix_en: "If you don't want certain comments dropped, use `--no-reorder`, or move the \
                    comment immediately above its corresponding key=value line before running \
                    `--reorder`",
            },
            DiagnosticCode::ReadDirFailed => CodeInfo {
                code: *self,
                source: CodeSource::Analyze,
                why_ja: "`analyze --kind coupling`が指定されたディレクトリを読めませんでした\
                    （存在しない・権限が無い等）",
                why_en: "`analyze --kind coupling` could not read the specified directory (it does not \
                    exist, permission denied, etc.)",
                fix_ja: "ディレクトリのパス・存在・アクセス権限を確認してください",
                fix_en: "Check the directory path, its existence, and access permissions",
            },
            DiagnosticCode::ReadFailed => CodeInfo {
                code: *self,
                source: CodeSource::Analyze,
                why_ja: "ディレクトリ内の.datファイルの読み込み・パースに失敗しました",
                why_en: "Reading or parsing a .dat file within the directory failed",
                fix_ja: "該当ファイルの内容・エンコーディングを確認してください",
                fix_en: "Check the content and encoding of the affected file",
            },
            DiagnosticCode::MissingName => CodeInfo {
                code: *self,
                source: CodeSource::Analyze,
                why_ja: "obj=vehicleのレコードにnameがありません。connect解析（constraint参照の\
                    突合）はnameを車両の識別子として使うため、nameが無い車両は解析対象から除外されます",
                why_en: "An obj=vehicle record has no name. The coupling analysis uses name as the \
                    vehicle's identifier for matching constraint references, so a vehicle without a \
                    name is excluded from the analysis",
                fix_ja: "name=にこの車両の一意な識別子を指定してください",
                fix_en: "Specify a unique identifier for this vehicle in name=",
            },
            DiagnosticCode::DanglingVehicleConstraint => CodeInfo {
                code: *self,
                source: CodeSource::Analyze,
                why_ja: "constraint[prev]/constraint[next]が参照する車両名が、解析対象ディレクトリ内に\
                    存在しません。makeobjは参照の実在性を検証しない（xref_writer.ccの解決はゲーム\
                    読み込み時まで遅延される）ため、この不整合はゲームがパークセットを読み込むまで\
                    気づけません",
                why_en: "The vehicle name referenced by constraint[prev]/constraint[next] does not exist \
                    in the analyzed directory. makeobj does not validate reference existence \
                    (xref_writer.cc's resolution is deferred until the game loads the pakset), so this \
                    inconsistency goes unnoticed until the game actually loads it",
                fix_ja: "参照している車両名の綴りを確認するか、参照先の車両を同じディレクトリに\
                    追加してください",
                fix_en: "Check the spelling of the referenced vehicle name, or add the referenced \
                    vehicle to the same directory",
            },
            DiagnosticCode::UnsatisfiableConstraint => CodeInfo {
                code: *self,
                source: CodeSource::Analyze,
                why_ja: "この車両を含む、constraint[prev]/constraint[next]を満たす有限な編成が\
                    1つも組み立てられません（自身および参照車両の制約だけでは、先頭になれる車両から\
                    末尾になれる車両まで到達できません）。ゲーム内で編成を組もうとしても\
                    永遠に成立しない可能性があります",
                why_en: "No finite consist containing this vehicle can be assembled while satisfying \
                    constraint[prev]/constraint[next] (the constraints of this vehicle and its \
                    referenced vehicles alone cannot reach from a vehicle that can be first to one that \
                    can be last). Attempting to build a consist in-game may never succeed",
                fix_ja: "constraint[prev]/constraint[next]の連鎖を見直し、\"none\"（先頭/末尾でよい）へ\
                    到達できる経路が存在するよう修正してください",
                fix_en: "Review the constraint[prev]/constraint[next] chain and ensure a path exists \
                    that can reach \"none\" (allowed to be first/last)",
            },
            DiagnosticCode::FileReadFailed => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "`.dat`ファイル自体の読み込みに失敗しました（存在しない・権限が無い等、\
                    `DatFile::parse_all`のI/Oエラー）。makeobj自身のエラーではなく、\
                    dat_linterのファイル読み込み層のエラーです。`--format json`実行時のみ、\
                    テキストモードのエラーメッセージと同じ状況をdiagnostics配列内の\
                    1エントリとして構造化するために使われます",
                why_en: "Reading the .dat file itself failed (it does not exist, permission denied, \
                    etc. — an I/O error from DatFile::parse_all). This is not a makeobj error but a \
                    dat_linter file-reading error. It is only used under --format json, to represent \
                    the same situation as the text-mode error message as a structured entry in the \
                    diagnostics array",
                fix_ja: "ファイルパス・存在・アクセス権限、および文字エンコーディング（UTF-8または\
                    Shift-JIS）を確認してください",
                fix_en: "Check the file path, its existence, access permissions, and character \
                    encoding (UTF-8 or Shift-JIS)",
            },
            DiagnosticCode::UnsupportedObjType => CodeInfo {
                code: *self,
                source: CodeSource::Lint,
                why_ja: "`.dat`ファイルの`obj=`の値が、dat_linterが検証をサポートするobj種別\
                    （`dat_linter lint -h`に列挙される22種）のいずれにも一致しません\
                    （`obj=`自体が欠落しているファイル・レコードも含む）。`--format json`実行時のみ、\
                    テキストモードの\"obj=... は未対応です\"メッセージと同じ状況を\
                    diagnostics配列内の1エントリとして構造化するために使われます",
                why_en: "The .dat file's obj= value does not match any obj type dat_linter supports \
                    validating (the 22 types listed in `dat_linter lint -h`), including files/records \
                    where obj= itself is missing. It is only used under --format json, to represent \
                    the same situation as the text-mode \"obj=... is not supported\" message as a \
                    structured entry in the diagnostics array",
                fix_ja: "obj=の値の綴りを確認するか、`dat_linter lint -h`で対応obj種別の一覧を\
                    確認してください",
                fix_en: "Check the spelling of obj=, or see `dat_linter lint -h` for the list of \
                    supported obj types",
            },
        }
    }
}

/// 全`DiagnosticCode`のvariant一覧（`dat_linter list`が表示する内容の基礎）。
/// 同じcodeが複数のobj種別モジュールで共有される場合（例:
/// `missing-waytype`はbuilding.rs内の分岐とcommon.rs経由の両方から出る）でも
/// 一意のvariantとしては1つのみ列挙する（重複表示しない）。
///
/// `tests/codes_completeness.rs`が実ソースとの整合性（過不足）を保証するため、
/// 新しいvariantを追加した際はこの配列にも追記すること
/// （`as_str`/`info`の網羅matchはコンパイル時に強制されるが、この配列自体は
/// 手動保持のため、追記漏れがあると`from_str`・`dat_linter list`から
/// そのcodeが見えなくなる。ここだけは実行時テストに委ねている）。
pub const ALL: &[DiagnosticCode] = &[
    DiagnosticCode::ClampedValueOutOfRange,
    DiagnosticCode::NoBridgeImageSpecified,
    DiagnosticCode::ParsedPairs,
    DiagnosticCode::RawTypeWaytype,
    DiagnosticCode::ObsoleteType,
    DiagnosticCode::UnknownType,
    DiagnosticCode::TypeWaytypeOk,
    DiagnosticCode::GenericExtension,
    DiagnosticCode::ObsoleteKeyword,
    DiagnosticCode::DimsResolved,
    DiagnosticCode::ZeroSize,
    DiagnosticCode::DimsOk,
    DiagnosticCode::RawCursorIcon,
    DiagnosticCode::CursorIconNotApplicable,
    DiagnosticCode::MissingCursorIcon,
    DiagnosticCode::TileKeyLookup,
    DiagnosticCode::MissingTileImage,
    DiagnosticCode::TileImageOk,
    DiagnosticCode::FrontimageHeight,
    DiagnosticCode::BooleanStyleFieldNotZeroOrOne,
    DiagnosticCode::ImageOmitted,
    DiagnosticCode::DuplicateKey,
    DiagnosticCode::MissingWaytype,
    DiagnosticCode::UnknownWaytype,
    DiagnosticCode::WaytypeOk,
    DiagnosticCode::ImageRefEmptySentinel,
    DiagnosticCode::ImageRefResolved,
    DiagnosticCode::MissingImageFile,
    DiagnosticCode::ImageSizeNotMultipleOf128,
    DiagnosticCode::ImageCoordinateOutOfBounds,
    DiagnosticCode::ImageOk,
    DiagnosticCode::UnreadableImage,
    DiagnosticCode::DateIndexOverflow,
    DiagnosticCode::NameForbiddenFilenameCharacter,
    DiagnosticCode::NarrowIntOverflow,
    DiagnosticCode::EmbeddedNulInStringField,
    DiagnosticCode::UnknownSkinName,
    DiagnosticCode::CrossingIdenticalWaytypes,
    DiagnosticCode::CrossingMissingSpeed,
    DiagnosticCode::CrossingMissingOpenimage,
    DiagnosticCode::FactoryTypeOverride,
    DiagnosticCode::FactoryMissingMapcolor,
    DiagnosticCode::FactoryMapcolorOk,
    DiagnosticCode::FactoryOutputCapacityTooSmall,
    DiagnosticCode::FactorySmoketileWithoutOffset,
    DiagnosticCode::FactoryProbabilityClamped,
    DiagnosticCode::FactoryProductivityZero,
    DiagnosticCode::WaytypeOmitted,
    DiagnosticCode::MissingSeasonImage,
    DiagnosticCode::NoImages,
    DiagnosticCode::RoadsignImageCountNotMultipleOf4,
    DiagnosticCode::RoadsignImageMissing,
    DiagnosticCode::MissingAgeSeasonImage,
    DiagnosticCode::EngineTypeSkipped,
    DiagnosticCode::UnknownEngineType,
    DiagnosticCode::Incomplete8DirectionImages,
    DiagnosticCode::FreightimageCountMismatch,
    DiagnosticCode::MissingIndexedFreightimage,
    DiagnosticCode::MissingFreightimagetype,
    DiagnosticCode::ExtraFreightimagetype,
    DiagnosticCode::PowerGearMismatch,
    DiagnosticCode::MissingBaseImage,
    DiagnosticCode::BaseImageOk,
    DiagnosticCode::ClipBelowOutOfRange,
    DiagnosticCode::FmtReorderApplied,
    DiagnosticCode::FmtLeadingSpaceLine,
    DiagnosticCode::FmtMalformedLine,
    DiagnosticCode::FmtReorderUnsupportedObj,
    DiagnosticCode::FmtReorderLinesDropped,
    DiagnosticCode::ReadDirFailed,
    DiagnosticCode::ReadFailed,
    DiagnosticCode::MissingName,
    DiagnosticCode::DanglingVehicleConstraint,
    DiagnosticCode::UnsatisfiableConstraint,
    DiagnosticCode::FileReadFailed,
    DiagnosticCode::UnsupportedObjType,
];

/// 後方互換用のエイリアス。以前は`ALL_CODES: &[CodeInfo]`という名前の静的配列
/// だったが、`CodeInfo`単体が`code`フィールドを持つようになったため、
/// `ALL`（`DiagnosticCode`一覧）から`.info()`で導出できるようにした。
/// `dat_linter list`/`describe`はこちらを使う。
pub fn all_codes() -> impl Iterator<Item = CodeInfo> {
    ALL.iter().map(|c| c.info())
}
