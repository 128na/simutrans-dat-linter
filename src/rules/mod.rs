// building/vehicle/way/good/bridge/tunnel/roadsign/crossing/way-object/ground_obj/tree/citycar/pedestrian/factory/sound/ground の検証ロジック（cursor/icon省略時のスキップ、タイル画像欠落時の
// phases=0、frontimageのh>0、Dims size=0 fatal、vehicleのwaytype必須、engine_type
// フォールバック、8方向画像・freightimage完全性、wayのwaytype必須・base image必須・
// clip_belowクランプ、goodのfatal/warning分岐皆無の確認、bridgeのwaytype必須・
// get_int_clampedクランプ群・front画像未指定警告、tunnelのwaytype必須・
// 季節数/portal幅の可変画像キー走査、roadsignのwaytype必須・numbered/2D排他画像構文の
// fatal分岐再現、crossingの2waytype必須・解決後waytype一致検出・speed両方必須・
// openimage両方向必須、way-objectの2waytype（waytype/own_waytype）必須・
// ribi/slope/diagonal可変画像キー走査、ground_objのwaytype省略可・
// speed分岐によるphase/8方向季節画像走査、treeのage(5固定)×season全組み合わせ画像
// 必須、citycarのwaytype/engine_type/freight/constraint系フィールド皆無・
// 8方向画像の無条件（早期終了なし）走査、pedestrianのis_animated判定による
// 静止8方向画像/アニメーション画像（方向ごとに独立したフレーム走査）の排他分岐、
// factoryのbuilding_writer直接呼び出し（Dims/タイル画像/cursor・icon共有）・
// type上書きの罠・mapcolor必須・outputcapacity/smoketile-smokeoffsetの非fatal
// error・probability系クランプ、soundのfatal/warning分岐皆無の確認（goodと同型）、
// groundのwaytype/climates等の名前付きフィールド皆無・slope(0..127)×phase可変
// 画像キー走査・範囲外slopeキーが単に無視される点の確認等）は、
// いずれもmakeobjの
// building_writer.cc / vehicle_writer.cc / way_writer.cc / good_writer.cc /
// bridge_writer.cc / tunnel_writer.cc / roadsign_writer.cc / crossing_writer.cc /
// way_obj_writer.cc / groundobj_writer.cc / tree_writer.cc / citycar_writer.cc /
// pedestrian_writer.cc / factory_writer.cc / sound_writer.cc / ground_writer.cc /
// get_waytype.cc / xref_writer.cc / tabfile.cc を
// ソースとして直接ミラーしている。
//
// 検証済み:
// - vanilla simutrans: このリポジトリの `simutrans` submodule, commit 1d2799f9a7 (2026-01-16)
// - OTRP (Simutrans-Extended系フォーク, https://github.com/teamhimeh/simutrans),
//   commit d6d3a5795b (2026-07-01時点のdefaultブランチ) で同等ファイルを diff した結果、
//   building dat の検証に関わるロジックは両者で完全に一致していた
//   （差分はnode書き込みのバイナリフォーマット詳細のみで、dat記述者から見える挙動は同一）
//
// どちらかの本体が更新され、上記コミット以降にtype/waytype一覧やcursor/icon・
// タイル画像・vehicle画像・way画像・goodフィールド・tunnel画像キー・roadsign画像キー・
// crossing画像キー・way-object画像キー・ground_obj画像キー・tree画像キー・citycar画像キー・
// pedestrian画像キー・factoryフィールド・soundフィールド・ground画像キーのロジックが
// 変わった場合はこの定数表を再検証すること。
// vehicle系・way系・good系・bridge系・tunnel系・roadsign系・crossing系・way-object系・
// ground_obj系・tree系・citycar系・pedestrian系・factory系・sound系・ground系・menu系の
// ルールはOTRP側での個別diffはまだ行っていない（rules/vehicle.rs, rules/way.rs,
// rules/good.rs, rules/bridge.rs, rules/tunnel.rs, rules/roadsign.rs, rules/crossing.rs,
// rules/way_obj.rs, rules/groundobj.rs, rules/tree.rs, rules/citycar.rs,
// rules/pedestrian.rs, rules/factory.rs, rules/sound.rs, rules/ground.rs,
// rules/menu.rs参照）。
//
// menu（skin_writer.hの`menuskin_writer_t`、6種のskin_writer_tサブクラスのうち最初の
// 実装）で、`image_writer_t::write_obj`の`"> "`（ズーム不可フラグ）構文が
// obj種別中立の`common::check_image_ref`側に未対応だったことが判明し、
// `strip_zoomable_prefix`として修正済み（詳細はrules/menu.rs参照）。
//
// cursor（skin_writer.hの`cursorskin_writer_t`、6種のskin_writer_tサブクラスのうち
// 2番目の実装）は、skin_writer.h/skin_writer.ccを独立に読み直した結果、
// `menuskin_writer_t`と挙動上完全に同一（`get_type()`/`get_type_name()`の
// オーバーライドのみで`write_obj`は共有）であることを確認した。詳細はrules/cursor.rs
// 参照。
//
// symbol（skin_writer.hの`symbolskin_writer_t`、6種のskin_writer_tサブクラスのうち
// 3番目の実装）も、skin_writer.h/skin_writer.ccを独立に読み直した結果、
// `menuskin_writer_t`/`cursorskin_writer_t`と挙動上完全に同一（`get_type()`/
// `get_type_name()`のオーバーライドのみで`write_obj`は共有）であることを確認した。
// 詳細はrules/symbol.rs参照。
//
// smoke（skin_writer.hの`smoke_writer_t`、6種のskin_writer_tサブクラスのうち
// 4番目の実装）も、skin_writer.h/skin_writer.ccを独立に読み直した結果、
// `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`と挙動上完全に
// 同一（`get_type()`/`get_type_name()`のオーバーライドのみで`write_obj`は共有）で
// あることを確認した。「smoke」という名前から視覚エフェクトのタイミング処理を
// 連想したが、そのような処理は`obj=factory`側のフィールド（`smokeuplift`/
// `smokelifetime`等）とゲーム実行時ロジックに存在し、makeobjの`obj=smoke`書き込み
// 処理自体には含まれない。詳細はrules/smoke.rs参照。`obj=factory`の
// `smoketile[N]`/`smokeoffset[N]`フィールド（factory.rsのSmokeOffsetRuleが対象）
// とは全くの別概念であることに注意。
//
// field（skin_writer.hの`field_writer_t`、6種のskin_writer_tサブクラスのうち
// 5番目の実装）も、skin_writer.h/skin_writer.ccを独立に読み直した結果、
// `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/`smoke_writer_t`と
// 挙動上完全に同一（`get_type()`/`get_type_name()`のオーバーライドのみで
// `write_obj`は共有）であることを確認した。「field」という名前から作物の
// 生育段階の日数・タイミング処理を連想したが、そのような処理は`obj=factory`側の
// フィールド（`production_per_field`等）とゲーム実行時ロジックに存在し、
// makeobjの`obj=field`書き込み処理自体には含まれない。詳細はrules/field.rs参照。
// `obj=factory`の`fields`/`max_fields`/`min_fields`/`start_fields`フィールドとは
// 全くの別概念であることに注意。実際の公開`.dat`
// （`simutrans/pak128:factories/corn_farm.dat`、`Obj=factory`ブロックと
// `obj=field`ブロックを1ファイルに`----`区切りで連結する構成）で
// `obj=field`が使われていることを確認した。
//
// misc（skin_writer.hの`miscimages_writer_t`、6種のskin_writer_tサブクラスのうち
// 6番目（最後）の実装）も、skin_writer.h/skin_writer.ccを独立に読み直した結果、
// `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/`smoke_writer_t`/
// `field_writer_t`と挙動上完全に同一（`get_type()`/`get_type_name()`の
// オーバーライドのみで`write_obj`は共有）であることを確認した。
// `miscimages_writer_t`直前のクラスコメント「Used for images needed by the game
// but not yet integrated as real objects」は意味論上の説明に過ぎず、特別な検証・
// 専用フィールドを一切伴わないことも確認した。詳細はrules/misc.rs参照。実際の
// 公開`.dat`（`simutrans/pak128:base/misc_GUI/construction.dat`、
// `aburch/simutrans-pak128.britain:gui/gui128/misc_images-128.dat`）で
// `obj=misc`が使われていることを直接fetchして確認した。
//
// misc対応をもって、`src/simutrans/descriptor/writer/`配下の全24ヘッダファイルを
// 対象に`register_writer(true)`/`get_type_name()`を機械的に棚卸しした結果、
// makeobjが認識するトップレベルobj種別は building/vehicle/way/good/bridge/
// tunnel/roadsign/crossing/way-object/ground_obj/tree/citycar/pedestrian/
// factory/sound/ground/menu/cursor/symbol/smoke/field/misc の22種で尽きることを
// 確認した（詳細はtests/fmt.rsの`reorder_unsupported_obj_falls_back_to_preserve_order`
// コメント参照）。soundマイルストーンでの「これで最後」という過去の判断が実際には
// 誤りだった前例があるため、将来的にmakeobj本体側に新規obj種別が追加された場合は
// 改めてこの棚卸しを繰り返すこと。

pub mod bridge;
pub mod building;
pub mod citycar;
pub mod common;
pub mod crossing;
pub mod cursor;
pub mod factory;
pub mod field;
pub mod good;
pub mod ground;
pub mod groundobj;
pub mod menu;
pub mod misc;
pub mod pedestrian;
pub mod roadsign;
pub mod smoke;
pub mod sound;
pub mod symbol;
pub mod tree;
pub mod tunnel;
pub mod vehicle;
pub mod way;
pub mod way_obj;

pub use bridge::check_bridge;
pub use building::check_building;
pub use citycar::check_citycar;
pub use common::check_duplicate_keys;
pub use crossing::check_crossing;
pub use cursor::check_cursor;
pub use factory::check_factory;
pub use field::check_field;
pub use good::check_good;
pub use ground::check_ground;
pub use groundobj::check_groundobj;
pub use menu::check_menu;
pub use misc::check_misc;
pub use pedestrian::check_pedestrian;
pub use roadsign::check_roadsign;
pub use smoke::check_smoke;
pub use sound::check_sound;
pub use symbol::check_symbol;
pub use tree::check_tree;
pub use tunnel::check_tunnel;
pub use vehicle::check_vehicle;
pub use way::check_way;
pub use way_obj::check_way_obj;
