//! `obj=factory`（生産チェーンを持つ産業施設。inputgood/outputgoodで貨物を
//! 消費・生産する）の検証ルール。検証根拠は `rules/mod.rs` 冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/factory_writer.cc` / `factory_writer.h` /
//! `factory_desc.h` / `building_writer.cc`（factoryが直接呼び出す共有経路） /
//! `get_climate.cc` / `xref_writer.cc` / `dataobj/tabfile.cc`）を直接読んで確認した。
//! OTRP側の個別diffはまだ行っていない（citycar以降の全obj種別と同様）。
//!
//! ## `obj=`文字列について
//!
//! `factory_writer_t::get_type_name()`（`factory_writer.h:115`）は`return "factory";`
//! をそのまま返す。`factory_writer.h`には他にも`factory_field_class_writer_t`
//! （`"factory field class"`）・`factory_field_group_writer_t`（`"factory field"`）・
//! `factory_smoke_writer_t`（`"factory smoke"`）・`factory_product_writer_t`
//! （`"factory product"`）・`factory_supplier_writer_t`（`"factory supplier"`）という
//! 5つの補助writerクラスが定義されているが、これらは`.dat`の`obj=`欄に直接書く値では
//! なく（`register_writer(false)`＝トップレベルobj種別として登録されない）、
//! `factory_writer_t::write_obj`が生成するサブノード専用の内部クラスである。
//! GitHub code search（`repo:simutrans/pak128 obj=factory extension:dat`）で
//! `factories/bakery.dat`・`factories/open_coal_mine.dat`・
//! `factories/glass_factory.dat`・`factories/grain_farm_w_fields.dat`等、
//! 実在する多数の公開`.dat`ファイルで`obj=factory`（`Obj=factory`表記含む。
//! `tabfile_t`はキーを小文字化して読むため同じ値として扱われる）が使われている
//! ことを確認した。
//!
//! ## `factory_writer_t::write_obj`（factory_writer.cc:154-354）の構造
//!
//! factoryは他のobj種別と全く異なる構造を持つ。**buildingの`write_obj`を
//! そのまま呼び出す**（factory_writer.cc:223、
//! `building_writer_t::instance()->write_obj(fp, node, obj)`。同じ`tabfileobj_t&
//! obj`をそのまま渡す）ため、buildingが検証する`Dims`・タイル画像
//! （`{front|back}image[layout][y][x][h][phase][season]`）・`cursor`/`icon`・
//! `climates`・`level`等のフィールドは**factoryの`.dat`でもそのまま同じ形式で
//! 必要**になる。以下はfactory固有の追加フィールドについてのみ記述する
//! （building由来のフィールドは`rules/building.rs`参照）。
//!
//! - **`type`上書きの罠**（factory_writer.cc:220）: `obj.put("type", "fac")`を
//!   `building_writer_t::instance()->write_obj`呼び出しの直前に実行している。
//!   `tabfileobj_t::put()`（tabfile.cc:74-81）は`if(objinfo.get(key).str) return
//!   false;`という実装で、**既に`type`キーが存在する場合は何もせず`false`を返す
//!   （先勝ち）**。つまり:
//!   - `.dat`に`type=`を一切書いていない場合のみ、`put`が成功して`type=fac`が
//!     設定される（building_writer.cc:160-163の`fac`分岐に入り、
//!     `enables|=4`・`type=building_desc_t::factory`となる、これが期待される
//!     正常系）。
//!   - `.dat`に`type=`を明示的に書いていると（例えば`type=res`や、building側の
//!     obsolete判定に引っかかる`type=station`等）、`put`は静かに失敗し、
//!     building_writer_t::write_obj は**その明示的な値**を使って分岐する。
//!     `type=station`等のobsolete値なら`dbg->fatal`（building_writer.cc:193）で
//!     FATAL ERRORになるが、`type=res`のような**既知だが`fac`ではない値**では
//!     FATALにもならず、単に`building_desc_t::city_res`等として扱われて
//!     factoryとして機能しない建物が黙って生成される（`enables|=4`は
//!     `obj.get_int("enables_ware",0)>0`の場合のみ、building_writer.cc:213）。
//!     実際のpak128公開`.dat`（`bakery.dat`/`open_coal_mine.dat`/
//!     `glass_factory.dat`等）は例外なく`type=`を書いていないことを確認済みで、
//!     `type`を明示するのは非対称的で意図しないミスの可能性が高い。
//! - **`mapcolor`**（`obj.get_color("mapcolor", 255)`、factory_writer.cc:168）:
//!   `tabfileobj_t::get_color()`はMAKEOBJビルドでは`strtoul(value, NULL, 0)`
//!   （tabfile.cc:176-177、`#else`分岐、`MAKEOBJ`定義時にコンパイルされる方）を
//!   返すだけで、キー欠落時は`def`（255）にフォールバックする。factory_writer.cc:
//!   170-172で`color == 255`なら
//!   `dbg->fatal("Factory", "%s missing an identification color! (mapcolor)",
//!   obj_writer_t::last_name)`になる。255を明示的に指定した場合と未指定の場合は
//!   区別できない（ソースコード自体がこの2つを区別しない）。
//! - **`outputcapacity[N]`**（`obj.get_int(buf, 0)`、factory_writer.cc:262）:
//!   `outputgood[N]`が非空の行について、`cap<11`だと
//!   `dbg->error("factory_writer_t::write_obj()", "Factory outputcapacity must be
//!   larger than 10! (currently %i)", cap)`になる（factory_writer.cc:264-266）。
//!   `dbg->fatal`ではなく`dbg->error`（`log_t::error`、log.cc:245-296）で、
//!   ログにERRORとして出力されるがプログラムを中断せず処理を継続する
//!   （FATAL ERRORとは異なりpak生成自体は成功する）。とはいえ明示的な
//!   エラーメッセージを伴う観測可能な分岐であり、意図しない値である可能性が
//!   高いためWarningとして検出する。
//! - **`smoketile[N]`/`smokeoffset[N]`**（factory_writer.cc:289-309）:
//!   `smoketile[0]`が非空なら「インデックス形式」とみなし、i=0..3の4つを走査する
//!   （factory_writer.cc:292-304、`for(int i=0;i<4;i++)`）。各iについて
//!   `smoketile[i]`が空ならその時点でループ終了（fatalにならない）。
//!   `smoketile[i]`が非空なのに対応する`smokeoffset[i]`が空だと
//!   `dbg->error("factory_writer_t::write_obj", "%s defined but not %s!",
//!   str_smoketile, str_smokeoffset)`になる（factory_writer.cc:299-301。
//!   `outputcapacity`と同じ`dbg->error`、非fatal）。`smoketile[0]`が空の場合は
//!   非インデックス形式の`smoketile`/`smokeoffset`（単数形、添字なし）を読むだけで
//!   検証は無い（factory_writer.cc:306-309）。
//! - **`probability_to_spawn`**（`obj.get_int("probability_to_spawn", 10)`、
//!   factory_writer.cc:80）と**`expand_probability`**（`obj.get_int(
//!   "expand_probability", 0)`、factory_writer.cc:176）は、10000以上だと
//!   それぞれ`printf("probability_to_spawn too large, set to 10,000\n")`
//!   （factory_writer.cc:85-88）・`printf("expand_probability too large, set to
//!   10,000\n")`（factory_writer.cc:177-180）を出力してから10000にクランプされる。
//!   `tabfileobj_t::get_int_clamped()`ではなく素の`if`文とインラインの
//!   `printf`だが、**`printf`自体は無条件に到達可能**（この`if`は
//!   `tabfileobj_t::get()`のNULL/空文字列区別に依存しない単純な数値比較であり、
//!   tree/ground_objの`climates`警告が見送られた「到達しないelse分岐」問題とは
//!   異なる）。ただし`dbg->warning`ではないため、bridgeの`ClampedRangeRule`が
//!   前提とする「`get_int_clamped`呼び出しである」という基準そのものは
//!   満たさない。一方でpedestrianの`steps_per_frame`（`max()`によるインライン
//!   クランプで**メッセージ出力を一切伴わない**、REJECTED）とは異なり、
//!   こちらは実際に固定文字列のメッセージを標準出力へ出す、観測可能な
//!   クランプである。「到達可能」かつ「観測可能なメッセージを伴う」という
//!   2条件を満たすため、`get_int_clamped`由来のClampedRangeRuleとは区別しつつ
//!   別種のWarningルールとして採用する。
//! - **`inputgood[N]`/`inputsupplier[N]`/`inputcapacity[N]`/`inputfactor[N]`**
//!   （factory_writer.cc:231-249）・**`outputgood[N]`/`outputfactor[N]`**
//!   （factory_writer.cc:251-272、`outputcapacity[N]`は上記の通り別扱い）・
//!   **`smoke=`**（factory_writer.cc:224-229、`factory_smoke_writer_t`経由の
//!   xref）・**`fields`/`fields[N]`**（factory_writer.cc:44-77、`obj_field`型
//!   xref）はいずれも`xref_writer_t::write_obj`（xref_writer.cc:12-33）を
//!   経由する参照であり、makeobj時点では参照先の`good`/`field`オブジェクトが
//!   パークセット内に実在するかを検証しない（`fatal`引数はpakファイル内に
//!   書き込まれ、ゲーム読み込み時に解決される。vehicleの`freight=`・
//!   tunnelの`way=`と同じ理由）。ディレクトリ横断のレジストリが本ツールに
//!   無いため対象外（下記REJECTED参照）。
//! - `sound`（factory_writer.cc:199-216）: 数値文字列なら`atoi`でsound_idとして
//!   使われ、非数値ならLOAD_SOUND方式のファイル名として扱われるだけで、
//!   いずれの経路もfatal/warningを出さない（crossingの`sound`と同じパターン）。
//! - `productivity`/`range`/`distributionweight`/`pax_level`/`expand_minimum`/
//!   `expand_range`/`expand_times`/`electricity_boost`/`passenger_boost`/
//!   `mail_boost`/`electricity_amount`/`electricity_demand`/`passenger_demand`/
//!   `mail_demand`/`sound_interval`（factory_writer.cc:165-194）は全て
//!   `get_int`の無条件フォールバックのみで読まれ、`get_int_clamped`は
//!   一切使われていない（bridgeの`ClampedRangeRule`に相当する根拠が無い）。
//! - **`location`**（factory_writer.cc:156-164）: `land`/`water`/`city`/`river`/
//!   `shore`/`forest`のいずれにもSTRICMPで一致しない場合（未指定・誤字含む）は
//!   `dbg->warning`/`dbg->fatal`を一切伴わずに黙って`factory_desc_t::Land`へ
//!   フォールバックする（三項演算子チェーンの最終`:`が常に`Land`）。goodの
//!   waytype同様、observableな根拠が無いため見送り（下記REJECTED参照）。
//!
//! `building_writer_t::write_obj`が処理する`Dims`・タイル画像・`cursor`/`icon`は
//! factoryにも**そのまま同一の形式で**適用される（factory_writer.cc:223の
//! 直接呼び出しにより、building_writer.cc:73-383の全ロジックがfactoryの`.dat`にも
//! 及ぶ）。ただし`type`は上記の通りfactory側で強制的に`fac`に上書きされる
//! （成功した場合）ため、building.rsの`TypeWaytypeRule`/`ObsoleteKeywordRule`
//! （`type`の既知値チェック・`waytype`必須チェック）はfactoryには**そのままの
//! 形では適用できない**（factoryのwaytype必須ケース`type=stop`/`type=depot`には
//! 通常到達しない。ただし上記の`type`上書きの罠のとおり、`.dat`が明示的に
//! `type=`を書いた場合はbuilding側のobsolete/unknown-type FATALパスに入り得るため、
//! `TypeOverrideRule`として別途検出する）。`Dims`（`zero-size`）・
//! `cursor`/`icon`（`missing-cursor-icon`）・タイル画像（`missing-tile-image`）・
//! `frontimage`のh>0検証は、building.rsと全く同じロジックをfactory.rs内に
//! 再実装する（`building.rs`内の各Ruleは非`pub`のため、モジュール間で直接
//! 共有できない。way-object/ground_obj/tree/citycar/pedestrianが個別に
//! `check_image_ref`のみ共有しロジック自体は再実装してきた、このプロジェクトの
//! 既存スタイルを踏襲する）。
//!
//! REJECTED（候補として検討したが根拠不十分、またはmakeobj側にfatal/warning分岐が
//! 無いため実装しなかった）:
//! - `location`（factory_writer.cc:156-164）が既知の6値のいずれにも一致しない
//!   場合の警告: `dbg->warning`/`dbg->fatal`を伴わずに黙って`Land`へ
//!   フォールバックするだけ（三項演算子チェーンの最終elseが常に`Land`で、
//!   goodのwaytype省略やvehicleのengine_typeフォールバックと異なり、
//!   このフォールバック自体を示すメッセージ出力が一切無い）。
//! - `productivity`/`range`/`distributionweight`/`pax_level`/`expand_minimum`/
//!   `expand_range`/`expand_times`/`electricity_boost`/`passenger_boost`/
//!   `mail_boost`/`electricity_amount`/`electricity_demand`/`passenger_demand`/
//!   `mail_demand`/`sound_interval`の妥当性検証: いずれも`get_int`の無条件
//!   フォールバックのみで`get_int_clamped`は一切使われていない
//!   （factory_writer.cc:165-194）。bridgeの`ClampedRangeRule`に相当する根拠が
//!   無いため見送り（way/tunnel/roadsign/crossing/way-object/groundobj/tree/
//!   citycar/pedestrianの同種フィールドが見送られたのと同じ理由）。
//! - `inputsupplier[N]`/`inputcapacity[N]`/`inputfactor[N]`の妥当性検証:
//!   いずれも`get_int`の無条件フォールバックのみで読まれる
//!   （factory_writer.cc:242-246）。`outputcapacity`と異なり、`inputcapacity`
//!   には対応する`dbg->error`分岐がfactory_writer.cc全文に存在しない。
//! - `inputgood[N]`/`outputgood[N]`/`smoke`/`fields`/`fields[N]`が参照する
//!   `good`/`field`オブジェクトの実在性検証: `xref_writer_t::write_obj`
//!   （xref_writer.cc:12-33）は参照を検証せずゲーム読み込み時まで解決を
//!   遅延する（vehicleの`freight=`・tunnelの`way=`と全く同じ理由）。参照先は
//!   パークセット全体のどこにあってもよく、ディレクトリ横断のレジストリが
//!   本ツールに無いため検証できない。
//! - `has_snow[N]`/`production_per_field[N]`/`storage_capacity[N]`/
//!   `spawn_weight[N]`（`factory_field_group_writer_t::write_obj`、
//!   factory_writer.cc:39-98）の妥当性検証: いずれも`get_int`の無条件
//!   フォールバックのみで読まれる（factory_writer.cc:48-51,67-73）。
//! - `min_fields`/`max_fields`/`start_fields`の相互妥当性（例:
//!   `min_fields > max_fields`）の検証: factory_writer.cc:81-83は
//!   3つとも独立に`get_int`で読むだけで、相互比較のfatal/warning分岐が
//!   存在しない。
//! - `smoketile[i]`が非インデックス形式（添字なし、単数形の`smoketile`）と
//!   インデックス形式（`smoketile[0]`）の混在検証: `smoketile[0]`の有無
//!   だけで分岐が完全に切り替わり（factory_writer.cc:291,306-309）、
//!   混在時にfatal/warningになる分岐は無い。
//! - `num_smoke_offsets`が4を超える`smoketile[4]`以降のキーの検証:
//!   `for(int i=0;i<4;i++)`（factory_writer.cc:292）は4回で無条件に
//!   ループを終えるだけで、`smoketile[4]`以降が定義されていても
//!   無視されるだけ（fatal/warningなし）。
//! - `sound`の妥当性検証: crossingの`sound`と全く同じ理由（数値/非数値どちらの
//!   経路もfatal/warningを出さない）。
//! - `mapcolor`に255を明示的に指定した場合との区別: ソースコード自体が
//!   「未指定（デフォルト255）」と「明示的な255」を区別しない
//!   （`get_color`のデフォルト値とFATAL判定がどちらも255のため）。本ツールも
//!   同じ曖昧さを引き継ぐ（区別する根拠が無い）。

use super::common::check_image_ref;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::{Rule, RuleContext};
use std::path::Path;

/// building.rsと同じく、`DimsRule`が返す(size_x, size_y, layouts)を
/// `TileImageRule`のコンストラクタへ渡す必要があるため、ここで一度だけ
/// `resolve_dims`を呼んで解決してから各ルールを構築する。
pub fn all(dat: &DatFile) -> Vec<Box<dyn Rule>> {
    let (size_x, size_y, layouts) = resolve_dims(dat);
    vec![
        Box::new(TypeOverrideRule),
        Box::new(MapColorRule),
        Box::new(DimsRule),
        Box::new(CursorIconRule),
        Box::new(TileImageRule {
            size_x,
            size_y,
            layouts,
        }),
        Box::new(OutputCapacityRule),
        Box::new(SmokeOffsetRule),
        Box::new(ProbabilityClampRule),
    ]
}

/// `check_building`等と対称的な薄いラッパー。
pub fn check_factory(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    let ctx = RuleContext { dat, dat_dir };
    all(dat).iter().flat_map(|r| r.check(&ctx)).collect()
}

/// building_writer.cc:220相当。factory_writer.cc:220の`obj.put("type", "fac")`は
/// `tabfileobj_t::put()`（tabfile.cc:74-81）の実装上、`type`キーが既に存在すると
/// 何もせず`false`を返す（先勝ち）。`.dat`が`type=`を明示していると、
/// building_writer_t::write_objはその値のまま分岐し、`fac`以外の既知型
/// （factoryとして機能しない）や、obsolete型（FATAL ERROR、building.rsの
/// `ObsoleteKeywordRule`相当）になり得る。
struct TypeOverrideRule;
impl Rule for TypeOverrideRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let type_name = ctx.dat.get("type").unwrap_or("").to_ascii_lowercase();
        if type_name.is_empty() || type_name == "fac" {
            return Vec::new();
        }
        vec![Diagnostic::error(
            "factory-type-override",
            format!(
                "type={type_name} が明示されています。factory_writer.cc:220の\
                 obj.put(\"type\",\"fac\")はtabfileobj_t::put()の先勝ち仕様により\
                 既存のtypeキーを上書きできません。building_writer_t::write_objは\
                 明示された値のまま分岐するため、obsolete型ならFATAL ERROR、\
                 fac以外の既知型（res/com/ind等）ならfactoryとして機能しない\
                 建物が黙って生成されます。obj=factoryではtypeを指定しないでください"
            ),
        )]
    }
}

/// factory_writer.cc:168-172: `obj.get_color("mapcolor", 255)`がデフォルト値
/// 255のままだと`dbg->fatal("Factory", "%s missing an identification color!
/// (mapcolor)", obj_writer_t::last_name)`。`tabfileobj_t::get_color()`は
/// MAKEOBJビルドでは`strtoul(value, NULL, 0)`を返すだけの単純な変換
/// （tabfile.cc:175-178）で、キー欠落時は255にフォールバックする。
struct MapColorRule;
impl Rule for MapColorRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let mapcolor = ctx.dat.get("mapcolor").unwrap_or("");
        let resolved = mapcolor
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|v| (0..=255).contains(v))
            .unwrap_or(255);
        if resolved == 255 {
            vec![Diagnostic::error(
                "factory-missing-mapcolor",
                "mapcolor が未指定（または255）です。factory_writer.cc は\
                 mapcolorが255のままだとFATAL ERRORにします\
                 （\"%s missing an identification color! (mapcolor)\"）。\
                 255を明示的に指定した場合と未指定の場合はmakeobj自体が\
                 区別しません",
            )]
        } else {
            vec![Diagnostic::info(
                "factory-mapcolor-ok",
                format!("mapcolor={resolved}"),
            )]
        }
    }
}

/// `Dims=`を`(size_x, size_y, layouts)`へ解決する。building.rsの`resolve_dims`と
/// 全く同じロジック（factoryはbuilding_writer_t::write_objをそのまま呼ぶため、
/// Dims解決ロジックも共通）。
fn resolve_dims(dat: &DatFile) -> (i64, i64, i64) {
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
struct DimsRule;
impl Rule for DimsRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let ints = ctx.dat.get_ints("dims");
        let (size_x, size_y, layouts) = resolve_dims(ctx.dat);
        let mut diags = vec![Diagnostic::debug(
            "dims-resolved",
            format!("Dims={ints:?} -> size_x={size_x} size_y={size_y} layouts={layouts}"),
        )];

        if size_x * size_y == 0 {
            diags.push(Diagnostic::error(
                "zero-size",
                format!("Dims のサイズが0です (size_x={size_x}, size_y={size_y})"),
            ));
        } else {
            diags.push(Diagnostic::info(
                "dims-ok",
                format!("size={size_x}x{size_y} layouts={layouts}"),
            ));
        }
        diags
    }
}

/// building_writer.cc:372-380: `cursor`/`icon`がどちらも空文字列だと
/// `cursorskin_writer_t::instance()->write_obj`自体が呼ばれない（fatal/warning
/// にはならない）。building.rsの`CursorIconRule`と同様、ビルドメニュー非表示の
/// リスクをwarningではなくerrorとして扱う（buildingと同じ実機観察の根拠を
/// factoryにも適用する。factoryもビルドメニューから選択して建てる建物の一種）。
struct CursorIconRule;
impl Rule for CursorIconRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let cursor = ctx.dat.get("cursor").unwrap_or("");
        let icon = ctx.dat.get("icon").unwrap_or("");
        let mut diags = vec![Diagnostic::debug(
            "raw-cursor-icon",
            format!("cursor=\"{cursor}\" icon=\"{icon}\""),
        )];

        if cursor.is_empty() && icon.is_empty() {
            diags.push(Diagnostic::error(
                "missing-cursor-icon",
                "cursor と icon が両方とも未指定です。makeobjはエラーを出さずにビルドしますが、ゲーム内のビルドメニューに表示されません",
            ));
            return diags;
        }

        if !icon.is_empty() {
            check_image_ref(icon, ctx.dat_dir, "icon", &mut diags);
        }
        if !cursor.is_empty() {
            check_image_ref(cursor, ctx.dat_dir, "cursor", &mut diags);
        }
        diags
    }
}

/// building_writer.cc:266-345: タイル画像の走査ロジック。building.rsの
/// `TileImageRule`と全く同じ構造（factoryはbuilding_writer_t::write_objを
/// そのまま呼ぶため）。
struct TileImageRule {
    size_x: i64,
    size_y: i64,
    layouts: i64,
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
                        "tile-key-lookup",
                        format!("layout {l} tile ({x},{y}): {front5} / {back5} ({front6} / {back6} もfallback確認)"),
                    ));

                    let front = dat.get(&front5).or_else(|| dat.get(&front6));
                    let back = dat.get(&back5).or_else(|| dat.get(&back6));

                    if front.is_none() && back.is_none() {
                        diags.push(Diagnostic::error(
                            "missing-tile-image",
                            format!(
                                "layout {l} tile ({x},{y}) に front/backimage が1枚もありません（makeobjはエラーを出さず空画像のタイルを生成します）"
                            ),
                        ));
                    } else {
                        diags.push(Diagnostic::info(
                            "tile-image-ok",
                            format!("layout {l} tile ({x},{y})"),
                        ));
                        if let Some(v) = front {
                            check_image_ref(
                                v,
                                dat_dir,
                                &format!("frontimage[{l}][{y}][{x}]"),
                                &mut diags,
                            );
                        }
                        if let Some(v) = back {
                            check_image_ref(
                                v,
                                dat_dir,
                                &format!("backimage[{l}][{y}][{x}]"),
                                &mut diags,
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
                        "frontimage-height",
                        format!("{key} : frontimageの高さ(h)は0のみ有効です（makeobjはエラーログを出すだけで処理を継続します）"),
                    ));
                }
            }
        }

        diags
    }
}

/// factory_writer.cc:251-272: `outputgood[N]`が非空の行について
/// `outputcapacity[N]`（`get_int(buf, 0)`）が11未満だと
/// `dbg->error("factory_writer_t::write_obj()", "Factory outputcapacity must be
/// larger than 10! (currently %i)", cap)`。非fatal（`log_t::error`はログ出力
/// のみでプログラムを中断しない、log.cc:245-296）だが、明示的なエラー
/// メッセージを伴う観測可能な分岐のためWarningとして検出する。
struct OutputCapacityRule;
impl Rule for OutputCapacityRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        for i in 0.. {
            let good_key = format!("outputgood[{i}]");
            let good = dat.get(&good_key).unwrap_or("");
            if good.is_empty() {
                break;
            }
            let cap_key = format!("outputcapacity[{i}]");
            let cap = dat
                .get(&cap_key)
                .and_then(|v| v.trim().parse::<i64>().ok())
                .unwrap_or(0);
            if cap < 11 {
                diags.push(Diagnostic::warning(
                    "factory-output-capacity-too-small",
                    format!(
                        "{cap_key}={cap} は11未満です。factory_writerは\
                         outputcapacityが10以下だとエラーログを出しますが\
                         処理は継続します（\"Factory outputcapacity must be \
                         larger than 10! (currently {cap})\"）"
                    ),
                ));
            }
        }
        diags
    }
}

/// factory_writer.cc:289-309: `smoketile[0]`が非空なら「インデックス形式」と
/// みなし、i=0..3を走査する。`smoketile[i]`が非空なのに対応する
/// `smokeoffset[i]`が空だと
/// `dbg->error("factory_writer_t::write_obj", "%s defined but not %s!",
/// str_smoketile, str_smokeoffset)`（非fatal、OutputCapacityRuleと同じ
/// `dbg->error`）。
struct SmokeOffsetRule;
impl Rule for SmokeOffsetRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();

        // factory_writer.cc:291: smoketile[0]が空ならインデックス形式自体を
        // 使っていない（非インデックス形式smoketile/smokeoffsetにフォール
        // バックするだけで検証が無い）。
        if dat.get("smoketile[0]").unwrap_or("").is_empty() {
            return diags;
        }

        for i in 0..4 {
            let tile_key = format!("smoketile[{i}]");
            let tile = dat.get(&tile_key).unwrap_or("");
            if tile.is_empty() {
                break;
            }
            let offset_key = format!("smokeoffset[{i}]");
            let offset = dat.get(&offset_key).unwrap_or("");
            if offset.is_empty() {
                diags.push(Diagnostic::warning(
                    "factory-smoketile-without-offset",
                    format!(
                        "{tile_key} が定義されていますが {offset_key} がありません。\
                         factory_writerはエラーログを出しますが処理は継続します\
                         （\"{tile_key} defined but not {offset_key}!\"）"
                    ),
                ));
            }
        }
        diags
    }
}

/// factory_writer.cc:80-88（`probability_to_spawn`）・176-180
/// （`expand_probability`）: いずれも`get_int(key, def)`が10000以上だと
/// `printf("... too large, set to 10,000\n")`を出力してから10000に
/// クランプされる。`get_int_clamped`ではなく素の`if`+`printf`だが、
/// この`if`は`tabfileobj_t::get()`のNULL/空文字列区別に依存しない単純な
/// 数値比較で常に到達可能であり（tree/ground_objの`climates`警告のような
/// デッドコードではない）、かつ固定文字列のメッセージを出力する
/// （pedestrianの`steps_per_frame`のような完全に無言のクランプでもない）。
struct ProbabilityClampRule;
impl Rule for ProbabilityClampRule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        let dat = ctx.dat;
        let mut diags = Vec::new();
        check_probability_field(
            dat,
            "probability_to_spawn",
            10,
            "probability_to_spawn too large, set to 10,000",
            &mut diags,
        );
        check_probability_field(
            dat,
            "expand_probability",
            0,
            "expand_probability too large, set to 10,000",
            &mut diags,
        );
        diags
    }
}

fn check_probability_field(
    dat: &DatFile,
    key: &str,
    default: i64,
    message: &str,
    diags: &mut Vec<Diagnostic>,
) {
    let value = dat
        .get(key)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default);
    if value >= 10000 {
        diags.push(Diagnostic::warning(
            "factory-probability-clamped",
            format!(
                "{key}={value} は10000以上です。factory_writerはこの値を\
                 サイレントに10000へクランプします（\"{message}\"）"
            ),
        ));
    }
}
