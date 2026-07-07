//! `obj=smoke`（車両・工場が排出する煙エフェクトのスキン画像定義。`.dat`に実際に書く
//! `obj=`の値は`smoke`。この`obj=smoke`（トップレベルobj種別）と、`obj=factory`が
//! 持つ`smoketile=`/`smokeoffset=`/`smoke=`**フィールド**（工場のどのタイル・座標から
//! この煙obj参照を発生させるかの指定）は全くの別概念であり、同じ単語を使うだけで
//! 意味的なつながりは無い。事前合意の14種計画には含まれておらず、soundマイルストーンでの
//! `descriptor/writer/`配下の機械的な棚卸しで発見された7種の新規obj種別のうち5件目に
//! 対応した1件）の検証ルール。検証根拠は`rules/mod.rs`冒頭コメント参照。
//!
//! 全ルールの根拠は vanilla simutrans の pinned commit
//! `1d2799f9a73adf94751e2d8357fea9dabcc4f740`
//! （`src/simutrans/descriptor/writer/skin_writer.cc` / `skin_writer.h` /
//! `imagelist_writer.cc` / `image_writer.cc` / `obj_writer.cc` / `dataobj/tabfile.cc`）を
//! 直接読んで確認した。OTRP側の個別diffはまだ行っていない（building以外のobj種別と
//! 同様）。
//!
//! ## `obj=`文字列について
//!
//! `smoke_writer_t::get_type_name()`（skin_writer.h:77-89）は`return "smoke";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ（menu/cursor/symbolと
//! 同一パターン）。さらに実際の公開`.dat`ファイル（GitHub code search、
//! `"obj=smoke" extension:dat`）でも`Obj=smoke`が使われていることを確認した
//! （例: `simutrans/pak128:base/smokes/Smokes.dat`（`Obj=smoke` /
//! `Name=Diesel` / `Image[0]=misc-smoke-128.0.0`など5フェーズの連番画像）、
//! `aburch/simutrans-pak128.britain:smokes/smoke.dat`、
//! `VictorErik/Pak128.Sweden-Ex:Base/Smokes/smokes.dat`、
//! `Cousjava/simutrans-paks:pak128.German/smoke/Steam.dat`、
//! `Flemmbrav/Pak192.Comic:pakset/vehicles/smoke/smoke_vehicles.dat`）。
//! **注意**: `obj=smoke`（このトップレベルobj種別）と、`obj=factory`が持つ
//! `smoketile[N]=`/`smokeoffset[N]=`/`smoke=`**フィールド**（工場のどのタイル・座標
//! から煙を発生させ、この`obj=smoke`をどう参照するかの指定。`src/rules/factory.rs`の
//! `SmokeOffsetRule`が検証対象）は全くの別概念であり、同じ単語を使うだけで意味的な
//! つながりは無い（`factory_writer.cc`は`smoketile[N]`/`smokeoffset[N]`を読むだけで、
//! `skin_writer.cc`/`skin_writer.h`とは無関係）。
//!
//! ## `skin_writer_t` / `smoke_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`と同じく、
//! `smoke_writer_t`（skin_writer.h:77-89）も共通の基底クラス`skin_writer_t`
//! （skin_writer.h:18-29）のサブクラスであり、`get_type()`/`get_type_name()`の2つの
//! オーバーライドのみを持つ（`write_obj`は一切オーバーライドしない）。よって`smoke`の
//! 実際の書き込みロジックは全て基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）
//! そのものであり、`menu`/`cursor`/`symbol`と**完全に同一**である。共有アーキテクチャの
//! 詳細（`write_obj`のコード引用、`obj.get`のNULL/空文字列挙動、`"-"`センチネル、
//! name/copyright、waytype/cursor/icon不在、`"> "`ズーム不可プレフィックス、
//! count不一致警告の到達不能性など）は`menu.rs`冒頭のdoc comment参照。
//!
//! smoke固有の追加確認事項: 「smoke」という名前が視覚エフェクトのタイミング
//! （アニメーション周期・フェード等）に関する特別な処理を示唆する可能性を考慮し、
//! 本モジュールでは`menu.rs`/`cursor.rs`/`symbol.rs`の調査結果を鵜呑みにせず、
//! skin_writer.h/skin_writer.ccを実際に読み直して独立に確認した。`smoke_writer_t`の
//! クラス定義（skin_writer.h:77-89）には`get_type()`/`get_type_name()`以外の
//! メンバー関数は一切無く、`write_obj`のオーバーライドは無い。「煙のアニメーション
//! 周期・フェードタイミング」に相当する処理（`smokeuplift`/`smokelifetime`等）は
//! `.dat`のこの`obj=smoke`側ではなく、`obj=factory`側のフィールド
//! （`factory_writer.cc`が読む`smokeuplift`/`smokelifetime`）およびゲーム実行時の
//! `smoke_desc_t`（本ツールの対象外）にあり、`obj=smoke`自体は単に連番画像
//! （アニメーションフレーム列）を保持するスキン画像リストに過ぎないことが確認できた。
//! （`>`プレフィックス構文の実例はGitHub code searchでは見つからなかったが、
//! `image_writer_t::write_obj`側の処理はobj種別を一切区別しないため、書けば
//! 同じ挙動になる前提で`strip_zoomable_prefix_and_trim`をそのまま再利用する。）
//!
//! `smoke_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`と**挙動上完全に
//! 同一**であることを確認した（`get_type()`/`get_type_name()`の返り値が異なるだけ）。
//! よって本モジュールのルールは`menu.rs`/`cursor.rs`/`symbol.rs`の`AllImagesRule`と
//! 同一のロジックを、obj_type文字列とコメントの言い回しのみ差し替えて採用する。
//! REJECTEDの理由も全てmenu.rs/cursor.rs/symbol.rsと同一（詳細はmenu.rs参照）。
//! 追加で、アニメーション周期・フェード等の煙エフェクトのタイミング検証
//! （`smokeuplift`/`smokelifetime`の妥当性等）は`obj=factory`側のフィールドであり
//! `skin_writer.cc`/`skin_writer.h`には一切登場しないため対象外
//! （fatal/warning分岐の有無は`src/rules/factory.rs`のREJECTEDコメント参照）。

use super::common;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::Rule;
use std::path::Path;

/// ルール実装本体は`menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6種別で
/// 共有される`common::AllImagesRule`（skin_writer_t::write_objそのもの、根拠は
/// 上記コメント参照）。
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(common::AllImagesRule),
        Box::new(common::NameAndCopyrightStringFieldRule),
    ]
}

/// `tests/smoke_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_smoke(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("smoke", dat, dat_dir)
}
