// building/vehicle/way の検証ロジック（cursor/icon省略時のスキップ、タイル画像欠落時の
// phases=0、frontimageのh>0、Dims size=0 fatal、vehicleのwaytype必須、engine_type
// フォールバック、8方向画像・freightimage完全性、wayのwaytype必須・base image必須・
// clip_belowクランプ等）は、いずれもmakeobjの
// building_writer.cc / vehicle_writer.cc / way_writer.cc / get_waytype.cc /
// xref_writer.cc / tabfile.cc をソースとして直接ミラーしている。
//
// 検証済み:
// - vanilla simutrans: このリポジトリの `simutrans` submodule, commit 1d2799f9a7 (2026-01-16)
// - OTRP (Simutrans-Extended系フォーク, https://github.com/teamhimeh/simutrans),
//   commit d6d3a5795b (2026-07-01時点のdefaultブランチ) で同等ファイルを diff した結果、
//   building dat の検証に関わるロジックは両者で完全に一致していた
//   （差分はnode書き込みのバイナリフォーマット詳細のみで、dat記述者から見える挙動は同一）
//
// どちらかの本体が更新され、上記コミット以降にtype/waytype一覧やcursor/icon・
// タイル画像・vehicle画像・way画像のロジックが変わった場合はこの定数表を再検証すること。
// vehicle系・way系のルールはOTRP側での個別diffはまだ行っていない
// （rules/vehicle.rs, rules/way.rs参照）。

pub mod building;
pub mod common;
pub mod vehicle;
pub mod way;

pub use building::check_building;
pub use common::check_duplicate_keys;
pub use vehicle::check_vehicle;
pub use way::check_way;
