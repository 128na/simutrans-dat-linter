//! `obj=misc`（ゲームがまだ「本物のオブジェクト」として統合していない、雑多なUI用画像を
//! 保持するスキン画像定義。`skin_writer.h`のクラス名コメント「Used for images needed by
//! the game but not yet integrated as real objects」の通り）で検出する主な項目。
//! 事前合意の14種計画には含まれておらず、soundマイルストーンでの`descriptor/writer/`配下の
//! 機械的な棚卸しで発見された7種の新規obj種別のうち7件目（最後）に対応した1件。
//! 検証根拠は`rules/mod.rs`冒頭コメント参照。
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
//! `miscimages_writer_t::get_type_name()`（skin_writer.h:106-118）は`return "misc";`を
//! そのまま返す。根拠は`obj_writer_t::write`（obj_writer.cc:39-59）が
//! `obj.get("obj")`の文字列でそのまま`writer_by_name->get(type)`を引く実装であり、
//! `writer_by_name`への登録キーは各writerの`get_type_name()`の返り値
//! （`obj_writer_t::register_writer(true)`、obj_writer.cc:24-36の`if (main_obj)`分岐
//! 経由）である点は他のobj種別と同じ（menu/cursor/symbol/smoke/fieldと同一パターン）。
//! さらに実際の公開`.dat`ファイルを直接fetchして確認した（GitHub code searchは
//! 部分一致ノイズが多いため、有望なパス名を直接あたった）:
//! - `simutrans/pak128:base/misc_GUI/construction.dat`
//!   （`Obj=misc` / `Name=Construction` / `copyright=MHz` /
//!   `Image[0]=construction.1.0` 〜 `Image[9]=construction.1.9`の10枚構成）
//! - `aburch/simutrans-pak128.britain:gui/gui128/misc_images-128.dat`
//!   （`Obj=misc` / `Name=Construction` / `copyright=James` /
//!   `Image[0]` 〜 `Image[6]`の7枚構成）
//!
//! いずれも`skin_writer_t::write_obj`が生成する構造（`image[i]`の1次元・無制限走査 +
//! name/copyright）と完全に一致しており、`misc`固有の追加フィールドは`.dat`側に
//! 一切存在しないことも実例から確認できた。
//!
//! ## `skin_writer_t` / `miscimages_writer_t` の構造（skin_writer.h, skin_writer.cc）
//!
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/`smoke_writer_t`/
//! `field_writer_t`と同じく、`miscimages_writer_t`（skin_writer.h:106-118）も
//! 共通の基底クラス`skin_writer_t`（skin_writer.h:18-29）のサブクラスであり、
//! `get_type()`/`get_type_name()`の2つのオーバーライドのみを持つ（`write_obj`は
//! 一切オーバーライドしない）。よって`misc`の実際の書き込みロジックは全て
//! 基底`skin_writer_t::write_obj`（skin_writer.cc:18-51）そのものであり、
//! `menu`/`cursor`/`symbol`/`smoke`/`field`と**完全に同一**である。共有アーキテクチャの
//! 詳細（`write_obj`のコード引用、`obj.get`のNULL/空文字列挙動、`"-"`センチネル、
//! name/copyright、waytype/cursor/icon不在、`"> "`ズーム不可プレフィックス、
//! count不一致警告の到達不能性など）は`menu.rs`冒頭のdoc comment参照。
//!
//! misc固有の追加確認事項: `miscimages_writer_t`の直前コメント（skin_writer.h:104-106、
//! 「Used for images needed by the game but not yet integrated as real objects」）は
//! 「まだ本物のオブジェクトとして統合されていない画像用」という意味論上の説明であり、
//! 特別な検証処理・専用フィールドを示唆するものではないかを確認する必要があった。
//! 他5種と同様に本モジュールでも結論を鵜呑みにせずskin_writer.h/skin_writer.ccを
//! 実際に読み直して独立に確認したが、そのような特別な処理は`write_obj`側に一切無く、
//! `miscimages_writer_t`のクラス定義（skin_writer.h:106-118）には`get_type()`/
//! `get_type_name()`以外のメンバー関数は一切無い。（`>`プレフィックス構文の実例は
//! `construction.dat`系ファイルには見つからなかったが、`image_writer_t::write_obj`
//! 側の処理はobj種別を一切区別しないため、書けば同じ挙動になる前提で
//! `strip_zoomable_prefix_and_trim`をそのまま再利用する。）
//!
//! `miscimages_writer_t`はskin_writer.h/skin_writer.ccを実際に読み直した結果、
//! `menuskin_writer_t`/`cursorskin_writer_t`/`symbolskin_writer_t`/`smoke_writer_t`/
//! `field_writer_t`と**挙動上完全に同一**であることを確認した（`get_type()`/
//! `get_type_name()`の返り値が異なるだけ）。「まだ本物のオブジェクトとして統合されて
//! いない画像用」というクラス直前コメントの説明は、`obj=misc`という種別の
//! **意味論的な位置づけ**（パークセット全体でメタ的に使う未整理画像素材）を示す
//! だけであり、makeobjの`obj=misc`書き込み処理自体には特別な検証・専用フィールドは
//! 一切含まれないことを確認した。よって本モジュールのルールは`menu.rs`/`cursor.rs`/
//! `symbol.rs`/`smoke.rs`/`field.rs`の`AllImagesRule`と同一のロジックを、obj_type
//! 文字列とコメントの言い回しのみ差し替えて採用する。REJECTEDの理由も全て
//! menu.rs/cursor.rs/symbol.rs/smoke.rs/field.rsと同一（詳細はmenu.rs参照）。追加で、
//! `image[i]`の枚数・用途（例: `construction.dat`のコメントが示す「0=city
//! building, 1=tourist attraction...」というインデックスごとの意味）の妥当性検証は
//! ゲーム実行時ロジック（`skin_desc_t`側でインデックスの意味を解釈する）の領分であり
//! makeobj側には一切登場しないため対象外。

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

/// `tests/misc_lint.rs`専用。本番と同じ`RuleSet::for_obj_type`経由で
/// ディスパッチする（`super::common::check_via_dispatch`のdocコメント参照）。
pub fn check_misc(dat: &DatFile, dat_dir: &Path) -> Vec<Diagnostic> {
    super::common::check_via_dispatch("misc", dat, dat_dir)
}
