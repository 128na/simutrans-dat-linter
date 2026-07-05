//! `obj=field`（`obj=factory`の`fields=`フィールドが名前で参照する、収穫段階
//! （作物の生育フェーズ）ごとのスキン画像定義。`.dat`に実際に書く`obj=`の値は
//! `field`。この`obj=field`（トップレベルobj種別）と、`obj=factory`が持つ
//! `fields=`/`max_fields=`/`min_fields=`/`start_fields=`**フィールド**（工場が
//! どの`obj=field`を何個まで生成するかの指定。`src/rules/factory.rs`が対象）は
//! 全くの別概念であり、同じ単語を使うだけで意味的なつながりは無い。事前合意の
//! 14種計画には含まれておらず、soundマイルストーンでの`descriptor/writer/`配下の
//! 機械的な棚卸しで発見された7種の新規obj種別のうち6件目に対応した1件）の
//! 検証ルール。検証根拠は`rules/mod.rs`冒頭コメント参照。
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
//! `field_writer_t::get_type_name()`（skin_writer.h:91-101）は`return "field";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`register_writer(true)`経由）である点は他のobj種別と同じ（menu/cursor/symbol/
//! smokeと同一パターン）。さらに実際の公開`.dat`ファイル（GitHub code search、
//! `repos/simutrans/pak128`内`corn_farm.dat`を直接fetchして確認）でも
//! `obj=field`が使われていることを確認した。`corn_farm.dat`は1ファイル内に
//! `Obj=factory`ブロックと`obj=field`ブロックの**2つ**を`----`区切り線で
//! 連結する構成になっており、factory側の`fields=corn_field`が名前でこの
//! `obj=field`ブロック（`name=corn_field`）を参照する:
//!
//! ```text
//! Obj=factory
//! ...
//! max_fields=80
//! min_fields=10
//! production_per_field=5
//! fields=corn_field
//! ...
//! ----------------------------------
//! obj=field
//! name=corn_field
//! copyright=Sarlock
//!
//! Image[0]=corn_farm.4.3
//! Image[1]=corn_farm.4.0
//! Image[2]=corn_farm.4.1
//! Image[3]=corn_farm.4.2
//! Image[4]=corn_farm.4.4
//! ```
//!
//! 同様に`repos/simutrans/pak128:factories/fields_agriculture.dat`は
//! `obj=field`ブロックのみを複数（`cotton_field`/`grain_field`等）連結した
//! ファイルで、いずれも`name`/`copyright`/`image[0..4]`（収穫段階5枚、うち
//! 1枚は積雪版）という構成だった。これは`skin_writer_t::write_obj`が
//! 生成する構造そのもの（`image[i]`の1次元・無制限走査 + name/copyright）と
//! 完全に一致しており、`field`固有の追加フィールド（生育段階数や日数等）は
//! `.dat`側に一切存在しないことも実例から確認できた。
//!
//! ## `skin_writer_t` / `field_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/
//! `smoke_writer_t`と同じく、`field_writer_t`（skin_writer.h:91-101）も
//! 共通の基底クラス`skin_writer_t`（skin_writer.h:18-29）のサブクラスであり、
//! `get_type()`/`get_type_name()`の2つのオーバーライドのみを持つ（`write_obj`は
//! 一切オーバーライドしない）。よって`field`の実際の書き込みロジックは全て
//! 基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものであり、
//! `menu`/`cursor`/`symbol`/`smoke`と**完全に同一**である。共有アーキテクチャの
//! 詳細（`write_obj`のコード引用、`obj.get`のNULL/空文字列挙動、`"-"`センチネル、
//! name/copyright、waytype/cursor/icon不在、`"> "`ズーム不可プレフィックス、
//! count不一致警告の到達不能性など）は`menu.rs`冒頭のdoc comment参照。
//!
//! field固有の追加確認事項: 「field」という名前が作物の生育段階の日数・タイミング
//! 処理（`.dat`側での段階数指定や生育速度パラメータ等）を連想させる可能性を
//! 考慮し、本モジュールでもsmokeマイルストーンと同様に他4種の結論を鵜呑みにせず
//! skin_writer.h/skin_writer.ccを実際に読み直して独立に確認したが、そのような
//! 特別な処理は`write_obj`側に一切無い。実例（`fields_agriculture.dat`のコメント
//! 「5 images, of which one is snow」）からも、生育段階数は`.dat`記述側で単に
//! `image[0..4]`を書く枚数として表現されるだけで、段階数・タイミングを指定する
//! 専用フィールドは存在しないことがわかる（生育の進行速度自体は`obj=factory`側の
//! `production_per_field`やゲーム実行時ロジックの領分であり、`obj=field`自体は
//! 単なる画像リスト）。`>`プレフィックス構文を使う実例は`corn_farm.dat`/
//! `fields_agriculture.dat`には見つからなかったが、`image_writer_t::write_obj`側の
//! 処理はobj種別を一切区別しないため、書けば同じ挙動になる前提で
//! `strip_zoomable_prefix_and_trim`をそのまま再利用する。
//!
//! `field_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/
//! `smoke_writer_t`と**挙動上完全に同一**であることを確認した（`get_type()`/
//! `get_type_name()`の返り値が異なるだけ）。よって本モジュールのルールは
//! `menu.rs`/`cursor.rs`/`symbol.rs`/`smoke.rs`の`AllImagesRule`と同一のロジックを、
//! obj_type文字列とコメントの言い回しのみ差し替えて採用する。REJECTEDの理由も
//! 全てmenu.rs/cursor.rs/symbol.rs/smoke.rsと同一（詳細はmenu.rs参照）。追加で、
//! 生育段階数・生育速度等の妥当性検証（画像枚数と`obj=factory`側の
//! `production_per_field`等との整合性）は`obj=factory`側のフィールドであり
//! `skin_writer.cc`/`skin_writer.h`には一切登場しないため対象外
//! （fatal/warning分岐の有無は`src/rules/factory.rs`のREJECTEDコメント参照）。
//! `obj=field`側から見て参照元の`obj=factory`が何個・どの名前でこの`obj=field`を
//! 参照しているかを検証するには複数`.dat`ファイルを横断する解析が必要であり、
//! これは1ファイル単位の`lint`ではなく`couplings`サブコマンドのようなスコープに
//! なる（現時点では対象外）。

use super::common;
use crate::diagnostics::Diagnostic;
use crate::parser::DatFile;
use crate::registry::Rule;
use std::path::Path;

/// ルール実装本体は`menu`/`cursor`/`symbol`/`smoke`/`field`/`misc`の6種別で
/// 共有される`common::AllImagesRule`（skin_writer_t::write_objそのもの、根拠は
/// 上記コメント参照）。
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![Box::new(common::AllImagesRule)]
}

/// `tests/field_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_field(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("field", dat, dat_dir)
}
