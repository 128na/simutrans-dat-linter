// building/vehicle/way/good/bridge/tunnel/roadsign/crossing の検証ロジック（cursor/icon省略時のスキップ、タイル画像欠落時の
// phases=0、frontimageのh>0、Dims size=0 fatal、vehicleのwaytype必須、engine_type
// フォールバック、8方向画像・freightimage完全性、wayのwaytype必須・base image必須・
// clip_belowクランプ、goodのfatal/warning分岐皆無の確認、bridgeのwaytype必須・
// get_int_clampedクランプ群・front画像未指定警告、tunnelのwaytype必須・
// 季節数/portal幅の可変画像キー走査、roadsignのwaytype必須・numbered/2D排他画像構文の
// fatal分岐再現、crossingの2waytype必須・解決後waytype一致検出・speed両方必須・
// openimage両方向必須等）は、いずれもmakeobjの
// building_writer.cc / vehicle_writer.cc / way_writer.cc / good_writer.cc /
// bridge_writer.cc / tunnel_writer.cc / roadsign_writer.cc / crossing_writer.cc /
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
// crossing画像キーのロジックが変わった場合はこの定数表を再検証すること。vehicle系・way系・
// good系・bridge系・tunnel系・roadsign系・crossing系のルールはOTRP側での個別diffはまだ
// 行っていない（rules/vehicle.rs, rules/way.rs, rules/good.rs, rules/bridge.rs,
// rules/tunnel.rs, rules/roadsign.rs, rules/crossing.rs参照）。

pub mod bridge;
pub mod building;
pub mod common;
pub mod crossing;
pub mod good;
pub mod roadsign;
pub mod tunnel;
pub mod vehicle;
pub mod way;

pub use bridge::check_bridge;
pub use building::check_building;
pub use common::check_duplicate_keys;
pub use crossing::check_crossing;
pub use good::check_good;
pub use roadsign::check_roadsign;
pub use tunnel::check_tunnel;
pub use vehicle::check_vehicle;
pub use way::check_way;
