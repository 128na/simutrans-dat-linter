//! building/vehicle 両方のルールから参照される共通の定数・ヘルパー。
//! 検証根拠は `rules/mod.rs` 冒頭コメント参照。

use crate::codes::DiagnosticCode;
use crate::diagnostics::Diagnostic;
use crate::i18n::{Language, t};
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext, RuleSet};
use std::path::Path;

/// obj種別を問わず、パーサレベルで検出した重複キーを警告として出す。
/// `tabfileobj_t::put()`の実装（`if(objinfo.get(key).str) return false;`）と
/// `tabfile.h`のdocコメント「If keys are duplicated for one object, the first
/// value is used」により、実際のmakeobjは重複キーを**先勝ち**で無音に無視する。
/// makeobj自身はfatal/errorにしないためWarning止まりだが、意図しない値の
/// 上書き忘れである可能性が高いため検出する。
pub fn check_duplicate_keys(dat: &DatFile, lang: Language) -> Vec<Diagnostic> {
    dat.duplicates
        .iter()
        .map(|d| {
            Diagnostic::warning(
                DiagnosticCode::DuplicateKey,
                t!(lang,
                    ja: "キー \"{key}\" が複数回定義されています（{first}行目の値が採用され、\
                         {dup}行目は無視されます）。makeobjのtabfileobj_t::put()は既存キーを\
                         上書きしません（先勝ち、tabfile.h:45）",
                    en: "Key \"{key}\" is defined more than once (the value on line {first} \
                         is used, and line {dup} is ignored). makeobj's tabfileobj_t::put() \
                         does not overwrite existing keys (first-write-wins, tabfile.h:45)",
                    key = d.key,
                    first = d.first_line,
                    dup = d.duplicate_line,
                ),
            )
            .at(d.duplicate_line, d.key.clone())
        })
        .collect()
}

/// 各obj種別モジュールの`check_<objtype>`関数（22個、`tests/*_lint.rs`専用）が
/// 共有するディスパッチ経路。
///
/// 第15弾（code smellレビュー・タスク11）: 以前は各`check_<objtype>`が
/// 自モジュールの`all(dat)`/`all()`を直接呼んでいたため、本番の実行経路
/// （`src/commands/lint.rs`が使う`RuleSet::for_obj_type`という「obj=文字列から
/// RuleSetへのディスパッチ」）を一度も通らず、`for_obj_type`のmatch分岐に
/// 将来バグ（typo・登録漏れ）が入ってもテストが検出できないという問題があった。
/// この関数はテストの入口を本番と同じ`RuleSet::for_obj_type`経由に統一する
/// （`obj_type`はコンパイル時に分かっている既知の文字列リテラルを呼び出し元が
/// 渡す前提のため、`for_obj_type`が`None`を返すのは呼び出し側のtypo等の
/// プログラミングミスのみ。テストヘルパーとして`.unwrap()`で即座に落とすのが
/// 適切と判断した）。
pub fn check_via_dispatch(obj_type: &str, dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext {
        dat,
        dat_dir,
        language: Language::default(),
    };
    RuleSet::for_obj_type(obj_type, dat)
        .unwrap_or_else(|| panic!("RuleSet::for_obj_type({obj_type:?}) returned None"))
        .run(&ctx)
}

/// `get_waytype()`（get_waytype.cc）がSTRICMPで受理する既知waytype一覧。
/// これ以外の値、および欠落は `dbg->fatal("get_waytype()","invalid waytype \"%s\"\n", ...)`。
/// building・vehicle 両方の`get_waytype()`呼び出しがこの同じ関数を経由するため、
/// このリストは両obj種別で共有する（重複定義しない）。
pub const KNOWN_WAYTYPES: &[&str] = &[
    "none",
    "road",
    "track",
    "electrified_track",
    "maglev_track",
    "monorail_track",
    "narrowgauge_track",
    "water",
    "air",
    "schiene_tram",
    "tram_track",
    "power",
    "decoration",
];

/// pak128の画像タイルサイズ。このプロジェクトは現状pak128のみを対象とする
/// （image_writer.cc:270-275 `block_load()`: `if ((width%img_size!=0)||
/// (height%img_size!=0)) dbg->error(...)`で読み込み失敗を返し、
/// `write_obj`側（image_writer.cc:409-413）が`throw obj_pak_exception_t(...)`で
/// pak生成全体を中断させる。実質的にビルドを失敗させる意味でFATAL相当）。
///
/// REJECTED（第6弾で再調査、対応不要と判断）: pak128実データ全体への`lint`実行で
/// `base/misc_GUI_64/`配下のファイル（`wkz_icons.png`が3136x384等、128の倍数でない）
/// が`image-size-not-multiple-of-128`として大量に誤検知しているように見えたため、
/// `image_writer.cc`と`obj_writer.cc`を再調査した。結果:
/// - `img_size`は固定の128ではなく**実行時に決まるグローバル変数**
///   （`image_writer_t::img_size`、デフォルト64）で、`obj_writer_t::write()`
///   （obj_writer.cc:50）が`.dat`ごとに`obj.get_int("cell_size", default_image_size)`で
///   設定し直す。`default_image_size`自体は`makeobj pak<N>`のCLI引数
///   （`makeobj.cc:85-91`、`atoi(argv[0]+3)`）で決まる、つまり**「どのサイズで
///   ビルドするか」はコマンドライン引数次第**であり、`.dat`ファイル自体には
///   通常このサイズ情報を持たない（`cell_size=`フィールドで個別に上書きできるが、
///   pak128の実データにはこのフィールドを使う`.dat`が1件も無いことも確認した）
/// - `pak128/Makefile`（`DIRS64`/`DIRS128`の変数分け、`$(MAKEOBJ) PAK`と
///   `$(MAKEOBJ) verbose PAK128`の呼び分け）を確認したところ、
///   `base/misc_GUI_64`はpak128ビルドの対象**外**で、意図的に`PAK`
///   （デフォルトサイズ、実質pak64）でビルドされる別系統のアセットだった
/// - つまりこの誤検知は「本ツールが128チェックのロジックを誤っている」のではなく、
///   「pak128という1つのsubmodule内に、実際にはpak128としてビルドされない
///   pak64専用アセットが同居している」という、`.dat`ファイル単体からは
///   判別不可能な外部のビルド設定に起因するもの。`.dat`内に`cell_size=`が
///   無い以上、本ツール（1ファイルを見て検証する設計）に判別する手段が無い
///
/// 結論: 128チェック自体はpak128としてビルドする`.dat`に対しては正しく、
/// `misc_GUI_64`はそもそも本ツールの対象範囲（pak128）外のアセットである。
/// 誤検知ではないため`check_image_ref`のロジックは変更しない。
pub const PAK_TILE_SIZE: u32 = 128;

/// `vehicle_writer.cc`/`citycar_writer.cc:38`/`pedestrian_writer.cc:25-27`の
/// `dir_codes`配列そのもの（画像キーの添字となる8方向）。vehicle/citycar/pedestrianの
/// 3つのobj種別で全く同一の配列内容であることを確認済み（値・順序とも一致）。
pub const DIR_CODES: [&str; 8] = ["s", "w", "sw", "se", "n", "e", "ne", "nw"];

/// 画像参照（`icon=`, `frontimage[...]=`等）を検証する。ファイル存在確認と
/// サイズが`PAK_TILE_SIZE`の倍数か（`image_writer.cc`の該当fatalチェック）を見る。
/// building・vehicleの画像フィールドはどちらも同じ`layer.season.frame`形式の
/// サフィックスを持つため、両者から共有する。
/// waytypeを表すフィールドの必須性・既知値チェック。`get_waytype()`を無条件に
/// 呼ぶobj種別（way/bridge/tunnel/roadsign/vehicle/way-object/crossing）で
/// 共通のパターンを1箇所に集約する。`key`は`"waytype"`のような単純フィールド名
/// でも、`"own_waytype"`や`"waytype[0]"`のような別名・添字付きキーでもよい。
/// 呼び出し元ごとの`obj=`名はメッセージに含めず、`key`と値のみで表現する
/// （元の各obj種別別メッセージが持っていた「obj=way では」等の接頭辞はここでは
/// 付けない。これは表示テキストの差異であり、`missing-waytype`/`unknown-waytype`/
/// `waytype-ok`の3つの診断コードと重大度（Error/Error/Info）は元の実装と同じ）。
///
/// 根拠: `get_waytype()`（get_waytype.cc）はSTRICMPで`KNOWN_WAYTYPES`の
/// いずれにも一致しなければ`dbg->fatal("get_waytype()","invalid waytype
/// \"%s\"\n", ...)`で落とす。`tabfileobj_t::get()`はNULLを返さず欠落キーには
/// 空文字列を返す（tabfile.cc:48-56）ため、キー未指定も同じfatalパスに入る。
pub fn check_waytype_field(dat: &DatFile, key: &str, lang: Language) -> Vec<Diagnostic> {
    let waytype = dat.get(key).unwrap_or("").to_ascii_lowercase();
    if waytype.is_empty() {
        vec![Diagnostic::error(
            DiagnosticCode::MissingWaytype,
            t!(lang,
                ja: "{key} が必須です（get_waytype()は空文字列もFATAL ERRORにします）",
                en: "{key} is required (get_waytype() treats an empty string as a FATAL ERROR too)",
                key = key,
            ),
        )]
    } else if !KNOWN_WAYTYPES.contains(&waytype.as_str()) {
        vec![Diagnostic::error(
            DiagnosticCode::UnknownWaytype,
            t!(lang,
                ja: "{key}={waytype} は不正な値です（FATAL ERRORになります）",
                en: "{key}={waytype} is not a valid value (this becomes a FATAL ERROR)",
                key = key,
                waytype = waytype,
            ),
        )]
    } else {
        vec![Diagnostic::info(
            DiagnosticCode::WaytypeOk,
            t!(lang,
                ja: "{key}={waytype}",
                en: "{key}={waytype}",
                key = key,
                waytype = waytype,
            ),
        )]
    }
}

/// `tabfileobj_t::get_int_clamped(key, def, min, max)`（tabfile.cc:201-212）を
/// 模倣する共有チェック。値が`min..max`範囲外だと`dbg->warning(...)`を出して
/// 黙って範囲内にクランプする（FATALにはしない）という、makeobj側の実際の挙動を
/// 再現する。
///
/// 第19弾（code smellレビュー・タスク15）: `bridge.rs`の`ClampedRangeRule`
/// （複数フィールドをループで回す、`ClampedField`一覧を持つ実装）と`way.rs`の
/// `ClipBelowRangeRule`（`clip_below`単一フィールド専用の個別実装）が、
/// 「`dat.get(key)`→trim→空なら早期return→`i64`にparse→パース失敗なら早期
/// return→範囲外なら`Diagnostic::warning`」という同一のロジックをそれぞれ
/// 独立に実装していたため、ここに1本化した。診断code（`bridge.rs`は
/// `DiagnosticCode::ClampedValueOutOfRange`、`way.rs`は
/// `DiagnosticCode::ClipBelowOutOfRange`）はフィールドごとに異なるため、
/// 呼び出し元がパラメータとして渡す。
///
/// `tabfileobj_t::get_int()`は`strtol(value, NULL, 0)`を使う（tabfile.cc:
/// 183-198）。Rustの`str::parse`ほど厳密ではないが、10進の妥当な数値かどうかの
/// 判定には`str::parse`で十分近似できる（パース不能な値は`strtol`が0を返すため、
/// その場合も範囲外にならずクランプは発生しない扱いになる。`bridge.rs`/`way.rs`
/// 双方の元実装が採用していた近似をそのまま踏襲する）。
pub fn check_clamped_int_field(
    dat: &DatFile,
    key: &str,
    min: i64,
    max: i64,
    code: DiagnosticCode,
    lang: Language,
) -> Vec<Diagnostic> {
    let Some(raw) = dat.get(key) else {
        return Vec::new();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let Ok(value) = trimmed.parse::<i64>() else {
        return Vec::new();
    };
    if value < min || value > max {
        vec![Diagnostic::warning(
            code,
            t!(lang,
                ja: "{key}={value} は範囲{min}..{max}外です。makeobjはFATALにはしませんが警告を出し、\
                     値を範囲内にクランプします（tabfileobj_t::get_int_clamped()）",
                en: "{key}={value} is out of range {min}..{max}. makeobj does not treat this \
                     as FATAL, but warns and clamps the value into range (tabfileobj_t::get_int_clamped())",
                key = key,
                value = value,
                min = min,
                max = max,
            ),
        )]
    } else {
        Vec::new()
    }
}

/// `image_writer_t::write_obj`（image_writer.cc:348-364）の構文仕様コメント
/// （`"[> ]imagefilename_without_extension[...]"`）どおり、先頭の`'>'`1文字は
/// 「ズーム不可」フラグとして`an_imagekey[0]=='>'`判定で剥がされ
/// （`an_imagekey = an_imagekey.substr(1)`、image_writer.cc:357-359）、
/// **`'>'`の有無に関わらず無条件で**続けて`trim(an_imagekey)`
/// （image_writer.cc:364、`utils/simstring.cc`の`trim()`本体は半角スペース/タブの
/// 前後除去のみ）が呼ばれる。この処理はobj種別を問わず`image_writer_t::write_obj`を
/// 通る全ての画像キーに無条件で適用される共通の構文であり、特定のobj種別に
/// 限定されない（menuのマイルストーンで実例`Image[0]=> skins.0.4`
/// （aburch/simutrans-pak128.britain:gui/gui64/skins-64.dat）から`'>'`構文の
/// 見落としが発覚し、その調査の過程で`trim()`が`'>'`の有無に関係なく常に
/// 呼ばれる実装であることも判明した。剥がさず・trimせずに`check_image_ref`へ
/// 渡すと、`"> skins.0.4"`や`" station_icon.png.0.0"`のような文字列をそのまま
/// ファイル名として解決を試み、実在するファイルを「見つからない」と誤検知する。
/// Rustの`str::trim()`はUnicode空白全般を対象とし対象がC++の` `/`\t`限定より
/// 広いが、実用上の`.dat`ファイルでこの差が問題になることはない。
fn strip_zoomable_prefix_and_trim(value: &str) -> &str {
    value.strip_prefix('>').unwrap_or(value).trim()
}

/// 画像参照からファイル名を取り出す。`image_writer_t::write_obj`
/// （image_writer.cc:372-388）は次の2段階でファイル名の幹を取り出し、
/// 無条件で`".png"`を付与する:
///
/// 1. `int j = imagekey.rfind('/');` — 値全体から**最後の`'/'`**を探す
///    （無ければ`numkey`=値全体）。`numkey`はこの`'/'`より後ろの部分
///    （ディレクトリ接頭辞を除いたベース名+行列番号部分）になる
/// 2. `int i = numkey.find('.');` — **`numkey`（ディレクトリ接頭辞を除いた
///    部分）の中で**最初の`'.'`を探す。ここが値全体ではなく`numkey`基準である
///    点が重要で、`"../../icon_way3.1.0"`のようにディレクトリ接頭辞自体に
///    `'.'`を含む相対パス参照（`../`）では、値全体基準で最初の`'.'`を探すと
///    誤って`..`内の`'.'`にヒットしてしまう（第10弾で発見された実際の誤検知:
///    `iss/building/depot/depot.dat`の`icon=> ../../icon_way3.1.0`が
///    空文字列+`.png`に誤って解決されていた）
/// 3. `imagekey = inpath + imagekey.substr(0, imagekey.size()-numkey.size()-1) + ".png"`
///    — **元の値全体**（ディレクトリ接頭辞を含む）から、末尾の
///    `numkey.size()+1`文字（手順2で見つけた`'.'`より後ろの部分＋その`'.'`
///    自身）を取り除いた上で`.png`を付与する。つまりディレクトリ接頭辞は
///    保持されたまま、ベース名部分の最初の`'.'`だけを基準に切り詰められる
///    （`"../../icon_way3.1.0"` → `numkey`(1)="icon_way3.1.0" →
///    `numkey`内の最初の`.`直後="1.0"(3文字) → 元の値全体から末尾4文字
///    （"1.0"の3文字+直前の"."の1文字）を除去 → `"../../icon_way3"` +
///    `".png"` = `"../../icon_way3.png"`）
///
/// 1文字目の`'.'`の直後に続く文字列（`"foo.png.0.0"`のように慣習的に
/// `"png"`と書かれることが多いが、数字で始まらない限り何が書かれていても
/// 構わない）は行番号として`atoi()`され、非数値の先頭文字列は単に`0`になる
/// だけで実質無視される。つまり`"foo.png.0.0"`と`"foo.0.0"`は実際には
/// 全く同じ`"foo.png"`を指しており、`"png"`という文字列自体に構文上の
/// 意味は無い。参照が`".png"`を含まない場合（実際に配布されているpak128.japan系
/// アドオンでよく見る`"basename.col.row"`形式、例:
/// `icon=> JpClassicTerminal.4.0`）でも、makeobjと同じく`.png`を補って
/// 正しく解決できなければならない（以前の実装は`"最後の2つが数値なら
/// それより前を丸ごとファイル名とする"`というヒューリスティックで、
/// `.png`の補完が漏れていた）。
///
/// **`'/'`のみを区切りとして扱い、`'\'`（バックスラッシュ）は区切りとして
/// 扱わない**（`rfind('/')`のみを見るmakeobjに忠実。Windows的な直感に
/// 反するが、実データが`/`区切りで書かれている以上makeobjと同じ挙動に
/// 揃えるのが正しい）。
fn resolve_image_filename(value: &str) -> String {
    // 手順1: 最後の'/'より後ろ（無ければ値全体）を`numkey`とする。
    let numkey = match value.rfind('/') {
        Some(slash_idx) => &value[slash_idx + 1..],
        None => value,
    };
    // 手順2: `numkey`の中で最初の'.'を探す。無ければmakeobjはfatalにするが
    // （"no image number in %s"）、この関数はfilenameだけを返す設計のため、
    // 呼び出し元（check_image_ref）が"見つからない"エラーとして自然に
    // 検出できるよう値全体をそのまま返す（既存の`None`分岐の挙動を維持）。
    let Some(dot_idx_in_numkey) = numkey.find('.') else {
        return value.to_string();
    };
    // 手順3: 元の値全体から、`numkey`内で見つけた'.'より後ろの部分+その'.'
    // 自身（合計 numkey.len() - dot_idx_in_numkey 文字）を末尾から取り除く。
    let strip_len = numkey.len() - dot_idx_in_numkey;
    let stem = &value[..value.len() - strip_len];
    format!("{stem}.png")
}

/// `image_writer_t::write_obj`（image_writer.cc:348-453）は全obj種別が画像参照を
/// 書き込む際に必ず経由する共有関数で、366行目の
/// `if (imagekey != "-" && imagekey != "")`がファイル解決ロジック全体
/// （行370-450: row/col解析・パス構築・`block_load`によるファイル読み込み）を
/// 包んでいる。つまり値が`"-"`（または空文字列）の場合、makeobjはファイル名として
/// 解決しようとせず、451行目の`else`分岐で空画像ノードを書き込むのみ
/// （fatal/errorにはならない）。この判定は特定のobj種別に固有のものではなく
/// `image_writer_t::write_obj`という共有経路そのものの挙動であるため、
/// `check_image_ref`という共有関数の入口で一箇所だけ判定するのが正しい。
///
/// 第6弾（項目2）ではbuilding/factoryのタイル画像ルールに個別で`v != "-"`
/// ガードを追加していたが、第8弾でway-object（`iss/way-object/road/wall_1.dat`の
/// `backdiagonal[nw]=-`）で同じ誤検知が再発したことを受け、per-obj-typeの
/// 場当たり対応ではなくこの関数自体に統一した。呼び出し側の個別ガードは
/// 冗長になるが害はないため残っていても良いが、新規に追加する必要は無い。
pub fn check_image_ref(
    value: &str,
    dat_dir: &Path,
    context: &str,
    diags: &mut Vec<Diagnostic>,
    lang: Language,
) {
    let value = strip_zoomable_prefix_and_trim(value);
    if value == "-" {
        diags.push(Diagnostic::info(
            DiagnosticCode::ImageRefEmptySentinel,
            t!(lang,
                ja: "{context}: \"-\"（画像なしセンチネル）が指定されています。\
                     image_writer_t::write_obj（image_writer.cc:366）はファイル解決を\
                     試みず空画像として扱います",
                en: "{context}: \"-\" (empty-image sentinel) is specified. \
                     image_writer_t::write_obj (image_writer.cc:366) treats this as an \
                     intentionally empty image without attempting file resolution",
                context = context,
            ),
        ));
        return;
    }
    let filename = resolve_image_filename(value);

    let path = dat_dir.join(&filename);
    diags.push(Diagnostic::debug(
        DiagnosticCode::ImageRefResolved,
        format!(
            "{context}: \"{value}\" -> filename=\"{filename}\" path={}",
            path.display()
        ),
    ));

    if !path.is_file() {
        diags.push(Diagnostic::error(
            DiagnosticCode::MissingImageFile,
            t!(lang,
                ja: "{context}: 参照画像 {filename} が見つかりません ({p})",
                en: "{context}: Referenced image {filename} was not found ({p})",
                context = context,
                filename = filename,
                p = path.display(),
            ),
        ));
        return;
    }

    match image::open(&path) {
        Ok(img) => {
            let (w, h) = (img.width(), img.height());
            if w % PAK_TILE_SIZE != 0 || h % PAK_TILE_SIZE != 0 {
                diags.push(Diagnostic::error(
                    DiagnosticCode::ImageSizeNotMultipleOf128,
                    t!(lang,
                        ja: "{context}: {filename} のサイズが {w}x{h} です。makeobj pak128 は{tile}の倍数でないとエラーになります",
                        en: "{context}: {filename} has size {w}x{h}. makeobj pak128 requires dimensions to be a multiple of {tile}",
                        context = context,
                        filename = filename,
                        w = w,
                        h = h,
                        tile = PAK_TILE_SIZE,
                    ),
                ));
            } else {
                diags.push(Diagnostic::info(
                    DiagnosticCode::ImageOk,
                    format!("{context}: {filename} {w}x{h}"),
                ));
            }
        }
        Err(e) => {
            diags.push(Diagnostic::error(
                DiagnosticCode::UnreadableImage,
                t!(lang,
                    ja: "{context}: {filename} を画像として読み込めません ({e})",
                    en: "{context}: Failed to read {filename} as an image ({e})",
                    context = context,
                    filename = filename,
                    e = e,
                ),
            ));
        }
    }
}

/// `menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6つのobj種別で共有される
/// ルール実装。いずれも共通の基底クラス`skin_writer_t`（skin_writer.h:18-29）の
/// サブクラスであり、`get_type()`/`get_type_name()`の2つのオーバーライドのみを
/// 持ち`write_obj`は一切オーバーライドしないため、実際の書き込みロジックは
/// 全て基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものである
/// （各モジュール冒頭のdocコメント参照、詳細な根拠は`menu.rs`に記載）。
///
/// skin_writer.cc:21-35: `image[0]`, `image[1]`, ... と1次元・無制限に走査し、
/// 最初に欠落した（空文字列の）添字で走査全体を終了する（`"-"`センチネルは
/// 空文字列ではないため走査を止めない）。実際に画像を指す値（空文字列でも
/// `"-"`でもない値）についてのみ、`check_image_ref`でファイル存在・
/// サイズ（128の倍数か）を検証する（他の全obj種別と共有のパターン）。
pub struct AllImagesRule;
impl Rule for AllImagesRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        let mut i = 0u32;
        loop {
            let key = format!("image[{i}]");
            let value = dat.get(&key).unwrap_or("");
            if value.is_empty() {
                // skin_writer.cc:28-30: キー欠落（空文字列）で走査終了。
                break;
            }
            if value != "-" {
                check_image_ref(value, ctx.dat_dir, &key, &mut diags, ctx.language);
            }
            i += 1;
            // 安全弁: dat構文異常でiが際限なく増え続ける事態を避ける
            // （makeobj自身は無限ループ`for (;;i++)`だが、実用上十分大きい上限で
            // 打ち切る。menu/cursor/symbol/smoke/field/miscの安全弁と同じ考え方）。
            if i > 4096 {
                break;
            }
        }

        diags
    }
}

// `building`/`factory`で共有される`Dims=`解決・タイル画像・cursor/icon検証。
//
// 第14弾: `factory_writer_t::write_obj`は`building_writer_t::write_obj`を
// そのまま呼び出す（`factory.rs`冒頭のdocコメント参照）ため、`Dims`解決・
// タイル画像走査・cursor/icon検証の3つのRuleはbuilding.rsとfactory.rsに
// ほぼ同一のコードとして重複していた（code smellレビューで指摘）。
// この3つをここに1本化し、両モジュールから薄いラッパー経由で呼ぶ。
// cursor/icon省略時の扱いだけがbuilding/factoryで異なる（`CursorIconPolicy`
// 参照）ため、そこだけパラメータ化した。

/// `Dims=`を`(size_x, size_y, layouts)`へ解決する。診断は伴わない純粋な計算。
/// `DimsRule`（診断を出す）と`TileImageRule`（値だけ必要）の両方から呼ばれる。
pub fn resolve_dims(dat: &DatFile) -> (i64, i64, i64) {
    let ints = dat.get_ints("dims");
    let size_x = ints.first().copied().unwrap_or(1);
    let size_y = ints.get(1).copied().unwrap_or(1);
    let mut layouts = ints.get(2).copied().unwrap_or(0);
    if layouts == 0 {
        layouts = if size_x == size_y { 1 } else { 2 };
    }
    (size_x, size_y, layouts)
}

/// building_writer.cc:95-97: `size.x*size.y == 0`だと
/// `dbg->fatal("building_writer_t::write_obj", "Cannot create a building with
/// zero size (%i,%i)", ...)`。factoryはbuilding_writer_t::write_objを
/// そのまま呼ぶため同じ検証が適用される。
pub struct DimsRule;
impl Rule for DimsRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let ints = ctx.dat.get_ints("dims");
        let (size_x, size_y, layouts) = resolve_dims(ctx.dat);
        let mut diags = vec![Diagnostic::debug(
            DiagnosticCode::DimsResolved,
            format!("Dims={ints:?} -> size_x={size_x} size_y={size_y} layouts={layouts}"),
        )];

        if size_x * size_y == 0 {
            diags.push(Diagnostic::error(
                DiagnosticCode::ZeroSize,
                t!(ctx.language,
                    ja: "Dims のサイズが0です (size_x={size_x}, size_y={size_y})",
                    en: "Dims size is 0 (size_x={size_x}, size_y={size_y})",
                    size_x = size_x,
                    size_y = size_y,
                ),
            ));
        } else {
            diags.push(Diagnostic::info(
                DiagnosticCode::DimsOk,
                format!("size={size_x}x{size_y} layouts={layouts}"),
            ));
        }
        diags
    }
}

/// cursor/icon両方未指定の場合に`CursorIconRule`がどう扱うかの方針。
///
/// - `Building`: `type=`の値が`types_without_build_menu`（呼び出し元が渡す一覧）に
///   含まれる場合のみ`cursor-icon-not-applicable`（info）、それ以外は
///   `missing-cursor-icon`（error）。building.rsの`TYPES_WITHOUT_BUILD_MENU`
///   （res/com/ind/cur/mon/tow）がこの一覧に対応する。
/// - `AlwaysNotApplicable`: `type=`の値によらず常に`cursor-icon-not-applicable`
///   （info）。factory（`obj=factory`）はそもそもプレイヤーが選ぶビルドメニューに
///   現れないため（`factory.rs`の`CursorIconRule`旧実装のdocコメント参照）、
///   `type=`を見る必要が無い。
pub enum CursorIconPolicy {
    Building {
        types_without_build_menu: &'static [&'static str],
    },
    AlwaysNotApplicable,
}

/// building_writer.cc:372-380: `cursor`/`icon`がどちらも空文字列だと
/// `cursorskin_writer_t::instance()->write_obj`自体が呼ばれない（fatal/warning
/// にはならない）。cursor/icon省略時に「見送ってよいケース」かどうかの判定は
/// `policy`（`CursorIconPolicy`）に委ねる（building/factoryで判定基準が異なるため）。
pub struct CursorIconRule {
    pub policy: CursorIconPolicy,
}
impl Rule for CursorIconRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let cursor = ctx.dat.get("cursor").unwrap_or("");
        let icon = ctx.dat.get("icon").unwrap_or("");
        let mut diags = vec![Diagnostic::debug(
            DiagnosticCode::RawCursorIcon,
            format!("cursor=\"{cursor}\" icon=\"{icon}\""),
        )];

        if cursor.is_empty() && icon.is_empty() {
            let type_name = ctx.dat.get("type").unwrap_or("").to_ascii_lowercase();
            let not_applicable = match &self.policy {
                CursorIconPolicy::Building {
                    types_without_build_menu,
                } => types_without_build_menu.contains(&type_name.as_str()),
                CursorIconPolicy::AlwaysNotApplicable => true,
            };
            if not_applicable {
                // res/com/ind/cur/mon/tow（building）またはfactory全般は
                // プレイヤーが選ぶビルドメニューにそもそも現れないため、
                // cursor/icon未指定はerrorではない。ただし完全に無言にはせず、
                // 「対象外と判断した」ことが分かるようinfoで残す。
                diags.push(Diagnostic::info(
                    DiagnosticCode::CursorIconNotApplicable,
                    t!(ctx.language,
                        ja: "cursor と icon が両方とも未指定ですが、type={type_name} は\
                             プレイヤーが選ぶビルドメニューに現れない種別（都市成長や\
                             特殊建造物として自動配置される）のため問題ありません",
                        en: "Both cursor and icon are unspecified, but type={type_name} is a \
                             category that never appears in the player-facing build menu \
                             (placed automatically by city growth or as a special building), \
                             so this is not an issue",
                        type_name = type_name,
                    ),
                ));
                return diags;
            }
            diags.push(Diagnostic::error(
                DiagnosticCode::MissingCursorIcon,
                t!(ctx.language,
                    ja: "cursor と icon が両方とも未指定です。makeobjはエラーを出さずにビルドしますが、ゲーム内のビルドメニューに表示されません",
                    en: "Both cursor and icon are unspecified. makeobj builds without error, but the \
                         object will not appear in the in-game build menu",
                ),
            ));
            return diags;
        }

        if !icon.is_empty() {
            check_image_ref(icon, ctx.dat_dir, "icon", &mut diags, ctx.language);
        }
        if !cursor.is_empty() {
            check_image_ref(cursor, ctx.dat_dir, "cursor", &mut diags, ctx.language);
        }
        diags
    }
}

/// building_writer.cc:266-345: タイル画像の走査ロジック。`building`/`factory`共通
/// （factoryは`building_writer_t::write_obj`をそのまま呼ぶため）。
pub struct TileImageRule {
    pub size_x: i64,
    pub size_y: i64,
    pub layouts: i64,
}
impl Rule for TileImageRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let dat = ctx.dat;
        let dat_dir = ctx.dat_dir;

        for l in 0..self.layouts {
            // building_writer.cc: 奇数レイアウトは縦横を入れ替えて走査する
            let (w, h) = if l % 2 == 1 {
                (self.size_y, self.size_x)
            } else {
                (self.size_x, self.size_y)
            };
            for y in 0..h {
                for x in 0..w {
                    let front5 = format!("frontimage[{l}][{y}][{x}][0][0]");
                    let back5 = format!("backimage[{l}][{y}][{x}][0][0]");
                    let front6 = format!("frontimage[{l}][{y}][{x}][0][0][0]");
                    let back6 = format!("backimage[{l}][{y}][{x}][0][0][0]");

                    diags.push(Diagnostic::debug(
                        DiagnosticCode::TileKeyLookup,
                        format!("layout {l} tile ({x},{y}): {front5} / {back5} ({front6} / {back6} もfallback確認)"),
                    ));

                    let front = dat.get(&front5).or_else(|| dat.get(&front6));
                    let back = dat.get(&back5).or_else(|| dat.get(&back6));

                    if front.is_none() && back.is_none() {
                        diags.push(Diagnostic::error(
                            DiagnosticCode::MissingTileImage,
                            t!(ctx.language,
                                ja: "layout {l} tile ({x},{y}) に front/backimage が1枚もありません\
                                     （makeobjはエラーを出さず空画像のタイルを生成します）",
                                en: "layout {l} tile ({x},{y}) has no front/backimage \
                                     (makeobj generates an empty tile without error)",
                                l = l,
                                x = x,
                                y = y,
                            ),
                        ));
                    } else {
                        diags.push(Diagnostic::info(
                            DiagnosticCode::TileImageOk,
                            format!("layout {l} tile ({x},{y})"),
                        ));
                        // "-"（画像なしセンチネル）の判定は`check_image_ref`側に
                        // 一元化されている。building/factoryのタイル画像は実データ
                        // （pak128 factories/cotton_farm_w_fields.dat の
                        // `BackImage[0][0][0][0][0][0]=-`等）で実際にこの値が
                        // 使われることを確認済み（第6弾）。以前はここに`v != "-"`
                        // ガードを個別追加していたが、第8弾でway_obj.rsに同種の
                        // 誤検知が再発したことを受け、`check_image_ref`自身が
                        // `-`を判定するよう共通化したため、ここでの個別ガードは不要
                        // （`check_image_ref`冒頭のdocコメント参照）。
                        if let Some(v) = front {
                            check_image_ref(
                                v,
                                dat_dir,
                                &format!("frontimage[{l}][{y}][{x}]"),
                                &mut diags,
                                ctx.language,
                            );
                        }
                        if let Some(v) = back {
                            check_image_ref(
                                v,
                                dat_dir,
                                &format!("backimage[{l}][{y}][{x}]"),
                                &mut diags,
                                ctx.language,
                            );
                        }
                    }
                }
            }
        }

        // frontimage の高さ(h)は0のみ許可。h>0が定義されていないか確認する
        for key in dat.pairs.keys() {
            if let Some(rest) = key.strip_prefix("frontimage[") {
                let indices: Vec<&str> = rest.trim_end_matches(']').split("][").collect();
                // [l][y][x][h][phase] (+season)
                if let Some(h_str) = indices.get(3)
                    && h_str.parse::<i64>().unwrap_or(0) > 0
                {
                    diags.push(Diagnostic::error(
                        DiagnosticCode::FrontimageHeight,
                        t!(ctx.language,
                            ja: "{key} : frontimageの高さ(h)は0のみ有効です\
                                 （makeobjはエラーログを出すだけで処理を継続します）",
                            en: "{key}: frontimage height (h) must be 0 \
                                 (makeobj logs an error but continues processing)",
                            key = key,
                        ),
                    ));
                }
            }
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 第10弾: resolve_image_filenameのディレクトリ接頭辞対応の回帰テスト。
    // image_writer_t::write_obj（image_writer.cc:372-388）のアルゴリズムを
    // 正確に再現しているかを、`resolve_image_filename`関数単体で確認する。

    #[test]
    fn no_directory_prefix_resolves_as_before() {
        // ディレクトリ接頭辞が無い既存ケース。従来通り最初の'.'より前がファイル名。
        assert_eq!(resolve_image_filename("foo.png.0.0"), "foo.png");
        assert_eq!(resolve_image_filename("foo.0.0"), "foo.png");
    }

    #[test]
    fn relative_path_with_double_dot_resolves_correctly() {
        // 第10弾で発見された実際の誤検知の再現（iss/building/depot/depot.dat の
        // `icon=> ../../icon_way3.1.0`）。値全体基準で最初の'.'を探すと
        // ".."内の'.'に誤ってヒットし、ファイル名が空文字列になっていた。
        // 正しくは最後の'/'より後ろ（"icon_way3.1.0"）の中で最初の'.'を探し、
        // ディレクトリ接頭辞"../../"は保持したまま切り詰める必要がある。
        assert_eq!(
            resolve_image_filename("../../icon_way3.1.0"),
            "../../icon_way3.png"
        );
    }

    #[test]
    fn single_level_relative_path_resolves_correctly() {
        assert_eq!(resolve_image_filename("../icon.1.0"), "../icon.png");
    }

    #[test]
    fn subdirectory_path_resolves_correctly() {
        // 親ディレクトリへ遡らない、下位ディレクトリを指す参照でも同様に動作する
        // べき（rfind('/')は"最後の/"を探すだけで、".."かどうかは関知しない）。
        assert_eq!(
            resolve_image_filename("icons/station_icon.png.0.0"),
            "icons/station_icon.png"
        );
    }

    #[test]
    fn backslash_is_not_treated_as_a_separator() {
        // makeobj（image_writer.cc:372の`rfind('/')`）は'/'のみを区切りとして
        // 扱い、'\'は区切りとして扱わない。Windows的な直感に反するが、
        // makeobjに忠実に合わせる（`\`はただの通常の文字として扱われ、
        // 値全体の中で最初の'.'を探す従来の（ディレクトリ接頭辞が無い場合の）
        // 挙動と同じになる）。
        assert_eq!(resolve_image_filename("dir\\icon.1.0"), "dir\\icon.png");
    }
}
