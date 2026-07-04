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

/// 1つの診断codeの情報。`dat_linter.toml`の`[rules] include/exclude`に
/// そのまま書ける文字列が`code`。`why_ja`/`why_en`（なぜNGか）と
/// `fix_ja`/`fix_en`（どう直すか）は`dat_linter describe <code>`が表示する説明文
/// （第10弾項目6）。
#[derive(Debug, Clone, Copy)]
pub struct CodeInfo {
    pub code: &'static str,
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
    CodeInfo {
        code: "no-bridge-image-specified",
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
    // --- lint: src/rules/building.rs ---
    CodeInfo {
        code: "parsed-pairs",
        source: CodeSource::Lint,
        why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。読み込んだ\
            key=valueの総数を示すだけで、問題を示すものではありません",
        why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv). \
            It just reports the total number of key=value pairs loaded and does not indicate a \
            problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "raw-type-waytype",
        source: CodeSource::Lint,
        why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。type/waytypeの\
            生の値を示すだけで、問題を示すものではありません",
        why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
            that reports the raw type/waytype values. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "obsolete-type",
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
    CodeInfo {
        code: "unknown-type",
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
    CodeInfo {
        code: "type-waytype-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。type/waytypeの組み合わせが正常であることを\
            示すだけで、問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming that the type/waytype \
            combination is valid. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "generic-extension",
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
    CodeInfo {
        code: "obsolete-keyword",
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
    CodeInfo {
        code: "dims-resolved",
        source: CodeSource::Lint,
        why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。Dims=から\
            解決されたsize_x/size_y/layoutsを示すだけで、問題を示すものではありません",
        why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
            that reports size_x/size_y/layouts resolved from Dims=. It does not indicate a \
            problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "zero-size",
        source: CodeSource::Lint,
        why_ja: "Dims=から解決されたsize_x*size_yが0です。building_writer.ccは\
            \"Cannot create a building with zero size\"としてdbg->fatalでFATAL ERRORにします",
        why_en: "size_x*size_y resolved from Dims= is 0. building_writer.cc treats this as a \
            FATAL ERROR (\"Cannot create a building with zero size\") via dbg->fatal",
        fix_ja: "Dims=に0以外の正の整数（例: Dims=1,1）を指定してください",
        fix_en: "Specify a positive non-zero integer for Dims= (e.g. Dims=1,1)",
    },
    CodeInfo {
        code: "dims-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。Dimsのサイズが正常であることを示すだけで、\
            問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming that Dims resolves to a \
            valid size. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "raw-cursor-icon",
        source: CodeSource::Lint,
        why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。cursor/iconの\
            生の値を示すだけで、問題を示すものではありません",
        why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
            that reports the raw cursor/icon values. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "cursor-icon-not-applicable",
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
    CodeInfo {
        code: "missing-cursor-icon",
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
    CodeInfo {
        code: "tile-key-lookup",
        source: CodeSource::Lint,
        why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。各タイルの\
            front/backimageキー探索の詳細を示すだけで、問題を示すものではありません",
        why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
            detailing the front/backimage key lookup for each tile. It does not indicate a \
            problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "missing-tile-image",
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
    CodeInfo {
        code: "tile-image-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。そのタイルにfront/backimageのいずれかが\
            定義されていることを示すだけで、問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming that a front/backimage \
            is defined for that tile. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "frontimage-height",
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
    // --- lint: src/rules/citycar.rs, pedestrian.rs (共有code) ---
    CodeInfo {
        code: "image-omitted",
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
    // --- lint: src/rules/common.rs ---
    CodeInfo {
        code: "duplicate-key",
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
    CodeInfo {
        code: "missing-waytype",
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
    CodeInfo {
        code: "unknown-waytype",
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
    CodeInfo {
        code: "waytype-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。waytypeの値が既知であることを示すだけで、\
            問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming the waytype value is \
            known. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "image-ref-empty-sentinel",
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
    CodeInfo {
        code: "image-ref-resolved",
        source: CodeSource::Lint,
        why_ja: "デバッグ用の情報表示です（Diagnostic::debug、-vvでのみ表示）。画像参照の値から\
            解決されたファイル名・パスを示すだけで、問題を示すものではありません",
        why_en: "A debug-level informational message (Diagnostic::debug, shown only with -vv) \
            that reports the filename/path resolved from an image reference value. It does not \
            indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "missing-image-file",
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
    CodeInfo {
        code: "image-size-not-multiple-of-128",
        source: CodeSource::Lint,
        why_ja: "参照画像の幅または高さが128の倍数ではありません。image_writer.ccの\
            block_load()は幅/高さが128（pak128のimg_size）の倍数でないと読み込み失敗を返し、\
            write_obj側がobj_pak_exception_tをthrowしてpak生成全体を中断させます",
        why_en: "The referenced image's width or height is not a multiple of 128. image_writer.\
            cc's block_load() fails to load images whose width/height is not a multiple of 128 \
            (pak128's img_size), and write_obj throws obj_pak_exception_t, aborting pak \
            generation entirely",
        fix_ja: "画像を128x128単位（128, 256, 384...）のサイズにリサイズ・パディングしてください",
        fix_en: "Resize or pad the image to a multiple of 128x128 (128, 256, 384, ...)",
    },
    CodeInfo {
        code: "image-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。参照画像が存在し、サイズも128の倍数で\
            あることを示すだけで、問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming that the referenced \
            image exists and its size is a multiple of 128. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "unreadable-image",
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
    // --- lint: src/rules/crossing.rs ---
    CodeInfo {
        code: "crossing-identical-waytypes",
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
    CodeInfo {
        code: "crossing-missing-speed",
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
    CodeInfo {
        code: "crossing-missing-openimage",
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
    // --- lint: src/rules/factory.rs ---
    CodeInfo {
        code: "factory-type-override",
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
    CodeInfo {
        code: "factory-missing-mapcolor",
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
    CodeInfo {
        code: "factory-mapcolor-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。mapcolorが255以外の値に設定されていることを\
            示すだけで、問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming mapcolor is set to a \
            value other than 255. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "factory-output-capacity-too-small",
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
    CodeInfo {
        code: "factory-smoketile-without-offset",
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
    CodeInfo {
        code: "factory-probability-clamped",
        source: CodeSource::Lint,
        why_ja: "probability_to_spawnまたはexpand_probabilityが10000以上です。\
            factory_writer.ccはこの値を\"too large, set to 10,000\"というメッセージを出力した上で\
            サイレントに10000へクランプします",
        why_en: "probability_to_spawn or expand_probability is 10000 or greater. factory_writer.\
            cc prints \"too large, set to 10,000\" and silently clamps the value to 10000",
        fix_ja: "値を10000未満に修正してください",
        fix_en: "Set the value below 10000",
    },
    CodeInfo {
        code: "factory-productivity-zero",
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
    // --- lint: src/rules/groundobj.rs ---
    CodeInfo {
        code: "waytype-omitted",
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
    CodeInfo {
        code: "missing-season-image",
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
    CodeInfo {
        code: "no-images",
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
    // --- lint: src/rules/roadsign.rs ---
    CodeInfo {
        code: "roadsign-image-count-not-multiple-of-4",
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
    CodeInfo {
        code: "roadsign-image-missing",
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
    // --- lint: src/rules/tree.rs ---
    CodeInfo {
        code: "missing-age-season-image",
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
    // --- lint: src/rules/vehicle.rs ---
    CodeInfo {
        code: "engine-type-skipped",
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
    CodeInfo {
        code: "unknown-engine-type",
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
    CodeInfo {
        code: "incomplete-8-direction-images",
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
    CodeInfo {
        code: "freightimage-count-mismatch",
        source: CodeSource::Lint,
        why_ja: "非indexedのfreightimage[<dir>]の個数がemptyimageの個数と一致しません。\
            vehicle_writer.ccは両者が完全一致することを要求し、不一致はFATAL ERRORにします",
        why_en: "The count of non-indexed freightimage[<dir>] entries does not match the count \
            of emptyimage entries. vehicle_writer.cc requires these to match exactly; a mismatch \
            is a FATAL ERROR",
        fix_ja: "freightimage[<dir>]の個数がemptyimageの個数と一致するよう追加・削除してください",
        fix_en: "Add or remove freightimage[<dir>] entries so the count matches emptyimage",
    },
    CodeInfo {
        code: "missing-indexed-freightimage",
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
    CodeInfo {
        code: "missing-freightimagetype",
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
    CodeInfo {
        code: "extra-freightimagetype",
        source: CodeSource::Lint,
        why_ja: "freightimagetype[N]が使用範囲（0..freight_image_type）より1つ多いindexで\
            定義されています。makeobjはFATALにはしませんが警告を出します（超過定義）",
        why_en: "freightimagetype[N] is defined one index beyond the used range \
            (0..freight_image_type). makeobj does not treat this as FATAL, but warns about the \
            excess definition",
        fix_ja: "使用していない超過分のfreightimagetype[N]を削除してください",
        fix_en: "Remove the unused excess freightimagetype[N] entry",
    },
    CodeInfo {
        code: "power-gear-mismatch",
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
    // --- lint: src/rules/way.rs ---
    CodeInfo {
        code: "missing-base-image",
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
    CodeInfo {
        code: "base-image-ok",
        source: CodeSource::Lint,
        why_ja: "情報表示です（Diagnostic::info）。image[-]またはimage[-][0]のいずれかが\
            定義されていることを示すだけで、問題ではありません",
        why_en: "An informational message (Diagnostic::info) confirming either image[-] or \
            image[-][0] is defined. It does not indicate a problem",
        fix_ja: "対応不要です",
        fix_en: "No action needed",
    },
    CodeInfo {
        code: "clip-below-out-of-range",
        source: CodeSource::Lint,
        why_ja: "clip_belowが0/1以外の値です。way_writer.ccはobj.get_int_clamped(\"clip_below\", \
            1, 0, 1)を呼ぶため、範囲外の値はdbg->warningを出した上で黙って0か1にクランプされます",
        why_en: "clip_below is not 0 or 1. way_writer.cc calls obj.get_int_clamped(\"clip_below\",\
             1, 0, 1), so an out-of-range value is warned about and silently clamped to 0 or 1",
        fix_ja: "clip_belowに0または1を指定してください",
        fix_en: "Specify 0 or 1 for clip_below",
    },
    // --- fmt: src/main.rs (fmt_one_file) ---
    CodeInfo {
        code: "fmt-reorder-applied",
        source: CodeSource::Fmt,
        // 第11弾: 専用の[fmt] reorder設定を廃止し、reorder自体をこのcodeで表現する
        // ([rules] include/excludeの仕組みに統合)。main.rs::fmt_one_fileがreorderを
        // 実際に適用したタイミングでDiagnostic::info("fmt-reorder-applied", ...)を発行する。
        why_ja: "fmtが慣習的な順序へキーを並び替える機能を表すcodeです。デフォルトで有効\
            （`--no-reorder`未指定・このcodeが`[rules] exclude`に無い場合）で、`fmt`実行時に\
            実際に並び替えを適用したことを示す情報表示（Diagnostic::info）として発行されます。\
            エラーや警告ではありません",
        why_en: "This code represents fmt's key-reordering feature itself. It is enabled by \
            default (unless --no-reorder is passed or this code is listed in [rules] exclude), \
            and is emitted as an informational message (Diagnostic::info) when `fmt` actually \
            applies reordering. It is not an error or warning",
        fix_ja: "恒久的に無効化したい場合は`[rules] exclude`にこのcode\
            （\"fmt-reorder-applied\"）を追加してください。1回の実行だけ無効化したい場合は\
            `--no-reorder`フラグを使ってください（`--no-reorder`はconfig設定より常に優先されます）",
        fix_en: "To permanently disable reordering, add this code (\"fmt-reorder-applied\") to \
            [rules] exclude. To disable it for a single invocation only, use the --no-reorder \
            flag (--no-reorder always takes priority over the config setting)",
    },
    // --- fmt: src/formatter/mod.rs ---
    CodeInfo {
        code: "fmt-leading-space-line",
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
    CodeInfo {
        code: "fmt-malformed-line",
        source: CodeSource::Fmt,
        why_ja: "行に`=`が含まれていません（区切り行`-`・コメント`#`・行頭スペース行を除く）。\
            makeobjはこの行を\"No data in ...\"としてdbg->warningを出した上で無視します",
        why_en: "The line contains no `=` (excluding separator lines starting with `-`, comments \
            starting with `#`, and leading-space lines). makeobj warns \"No data in ...\" and \
            ignores this line",
        fix_ja: "key=value形式に修正するか、意図しない行であれば削除してください",
        fix_en: "Fix the line to key=value form, or remove it if it was not intended",
    },
    CodeInfo {
        code: "fmt-reorder-unsupported-obj",
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
    CodeInfo {
        code: "fmt-reorder-lines-dropped",
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
    // --- analyze: src/couplings.rs ---
    CodeInfo {
        code: "read-dir-failed",
        source: CodeSource::Analyze,
        why_ja: "`analyze --kind coupling`が指定されたディレクトリを読めませんでした\
            （存在しない・権限が無い等）",
        why_en: "`analyze --kind coupling` could not read the specified directory (it does not \
            exist, permission denied, etc.)",
        fix_ja: "ディレクトリのパス・存在・アクセス権限を確認してください",
        fix_en: "Check the directory path, its existence, and access permissions",
    },
    CodeInfo {
        code: "read-failed",
        source: CodeSource::Analyze,
        why_ja: "ディレクトリ内の.datファイルの読み込み・パースに失敗しました",
        why_en: "Reading or parsing a .dat file within the directory failed",
        fix_ja: "該当ファイルの内容・エンコーディングを確認してください",
        fix_en: "Check the content and encoding of the affected file",
    },
    CodeInfo {
        code: "missing-name",
        source: CodeSource::Analyze,
        why_ja: "obj=vehicleのレコードにnameがありません。connect解析（constraint参照の\
            突合）はnameを車両の識別子として使うため、nameが無い車両は解析対象から除外されます",
        why_en: "An obj=vehicle record has no name. The coupling analysis uses name as the \
            vehicle's identifier for matching constraint references, so a vehicle without a \
            name is excluded from the analysis",
        fix_ja: "name=にこの車両の一意な識別子を指定してください",
        fix_en: "Specify a unique identifier for this vehicle in name=",
    },
    CodeInfo {
        code: "dangling-vehicle-constraint",
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
    CodeInfo {
        code: "unsatisfiable-constraint",
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
];
