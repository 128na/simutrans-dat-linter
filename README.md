# simutrans-dat-linter

Simutrans アドオンの `.dat`（オブジェクト定義ファイル）を **pak 化する前に** 静的検証する Rust 製 CLI ツールです。

`makeobj` はパラメーター不足・矛盾をほぼ無視して pak を生成してしまい、ゲーム内で初めて不具合に気付く
→ 原因調査に時間がかかる、という問題があります。このツールは makeobj の C++ ソース
（`building_writer.cc` / `vehicle_writer.cc` / `way_writer.cc` / `get_waytype.cc` / `image_writer.cc` /
`xref_writer.cc` / `tabfile.cc`）を精読し、**makeobj が黙って見逃す／FATAL ERROR にする項目**を、
Blender→PNG→pak のフルパイプラインを回さずに一瞬で検出します。makeobj には依存せず、`.dat` 構文を
独自に解析します。

## 3層の役割

| 層 | サブコマンド | 役割 | 例え |
|---|---|---|---|
| formatter | `fmt` | 見やすく整形（キー小文字化・`=`前後の空白除去・任意で並び替え） | — |
| linter | `lint` | pak 化に失敗する／ゲーム内で正しく表示されない項目を検知 | `php -l` |
| 静的解析 | `couplings` | 実行しなくても分かるゲーム内利用時の問題を検知 | PHPStan |

`lint` は1ファイル単位の検証、`couplings` は1ディレクトリ内の複数 `obj=vehicle` を横断する
グラフ解析（連結制約の充足可能性）です。スコープが異なるため意図的に別サブコマンドとして
分離しています。

## インストール

Rust ツールチェーン（stable, edition 2024 のため 1.85 以降）が必要です。

```
# クローンしてビルド
cargo build --release
# 生成物: target/release/dat_linter

# あるいはローカルからインストール（PATH に dat_linter が入る）
cargo install --path .
```

> コマンド名・バイナリ名は `dat_linter` です（リポジトリ名 `simutrans-dat-linter` とは別）。

## 使い方

すべてのサブコマンド（`lint` / `fmt` / `couplings`）は明示的に指定する必要があります
（サブコマンドを省略して `dat_linter <file>` とする旧来のショートカットは廃止しました）。

```
dat_linter --help
dat_linter lint --help
```

### `lint` — 静的検証（`obj=building` / `obj=vehicle` / `obj=way`）

```
dat_linter lint <path/to/file.dat>
dat_linter lint -v  <path/to/file.dat>   # info まで表示
dat_linter lint -vv <path/to/file.dat>   # debug（生の値・解決後パス）まで表示
```

**building** で検出する主な項目（すべて makeobj ソースで裏付け済み）:

- `cursor` と `icon` が両方未指定 → ビルドメニューに表示されない（makeobj はエラーを出さない）
- タイルに front/back image が1枚もない → 空画像タイルが黙って生成される
- `type` が obsolete（`station` / `hall` / `post` …）や未知の値 → FATAL ERROR
- `type=stop` / `type=depot` で `waytype` 未指定・不正 → FATAL ERROR
- `Dims` のサイズが 0
- 参照画像が見つからない／サイズが 128 の倍数でない → FATAL ERROR
- `=` 直後のスペースなど、値に混入した空白による参照失敗

**vehicle** で検出する主な項目（`vehicle_writer.cc` / `get_waytype.cc` / `xref_writer.cc` で裏付け済み）:

- `waytype` 未指定・不正 → FATAL ERROR（building と異なり `obj=vehicle` では常に必須）
- `engine_type` が既知値（`diesel`/`electric`/`steam`/`bio`/`sail`/`fuel_cell`/`hydrogene`/`battery`/`unknown`）
  以外 → fatal/error なしで黙って `diesel` にフォールバックする
  （`waytype=electrified_track` の場合は engine_type 自体が無視されるためチェック対象外）
- `emptyimage[n/e/ne/nw]` のいずれかを定義しているのに8方向すべて揃っていない → FATAL ERROR
- 非 indexed `freightimage[<dir>]` の個数が `emptyimage` と一致しない → FATAL ERROR
- indexed `freightimage[<N>][<dir>]`（複数貨物タイプ形式）の欠落 → FATAL ERROR
- `freightimagetype[<i>]` の欠落（FATAL）／使用範囲より1つ多い定義（WARNING）

**way** で検出する主な項目（`way_writer.cc` / `get_waytype.cc` / `tabfile.cc` で裏付け済み）:

- `waytype` 未指定・不正 → FATAL ERROR（vehicle と同様、`obj=way` では常に必須）
- `image[-]`（直進の無季節画像）が未指定 → FATAL ERROR。ただし `image[-][0]`（冬季season 0版）が
  定義されていれば「冬季画像あり」分岐に入るため対象外（`way_writer.cc` の分岐ロジックを厳密に再現）
- `image[-]` が参照する画像ファイルが見つからない／サイズが 128 の倍数でない → FATAL ERROR
- `clip_below` が 0/1 以外 → fatal/error なしで黙って 0 か 1 にクランプされる
  （`tabfileobj_t::get_int_clamped()` の WARNING）

**obj種別を問わず**: 同一キーの重複定義（`duplicate-key`, WARNING）。makeobj は重複キーを
**先勝ち**で無音に無視するため（`tabfileobj_t::put()`）、後から書いた値は意図せず捨てられます。

### `fmt` — フォーマッタ

```
dat_linter fmt <file.dat>              # 順序を保ったまま正規化（標準出力へ）
dat_linter fmt --reorder <file.dat>    # 慣習的な順序に並び替え
dat_linter fmt --write   <file.dat>    # ファイルに上書き（-w も可）
```

安全な正規化（キー小文字化・`=`前後の空白除去・値の前後トリム）のみ行い、値の内容変更のような
壊しうる操作は行いません。並び替え（`--reorder`）はスタイル上の慣習であり makeobj の動作には影響しないため、
オプトインです（コメント・空行は並び替え後の位置が一意に決まらないため出力から除外し件数を警告します）。
並び順は `obj=` の値ごとに定義されており（`building`/`vehicle`/`way` に対応）、未対応の obj 種別では
並び替えを行わず元の順序のまま出力します。

### `couplings` — 車両連結制約の静的解析

```
dat_linter couplings <path/to/vehicle_dat_dir>
```

1ディレクトリ内の全 `obj=vehicle` を読み込み、`constraint[prev]` / `constraint[next]` について:

1. **dangling 参照チェック**: 参照先の車両名がディレクトリ内に実在するか（makeobj は検証しない）
2. **充足可能性チェック**: 到達可能性解析により「有限な編成として絶対に成立しない車両」が無いか

を検査します。

## ログレベルと終了コード

| level | 表示条件 | 用途 |
|---|---|---|
| error | 既定 | pak 化に失敗する／ゲーム内で正常に表示されない |
| warn | 既定 | 非推奨・設定が推奨される項目 |
| info | `-v` | 各チェックの合格確認 |
| debug | `-vv` | 生の値・解決後パス・索いたキー名 |

終了コードは `error` が1件でもあれば `1`、それ以外（warn のみ含む）は `0`
（makeobj 自身が fatal にする／しないの区別に対応）。

## 検証根拠と対応範囲

各ルールは makeobj の C++ ソースで根拠を確認しています（詳細は `src/rules/mod.rs` /
`src/rules/vehicle.rs` / `src/rules/way.rs` 冒頭コメント参照）。building のルールは vanilla Simutrans と
OTRP（Simutrans-Extended 系フォーク）の該当ファイルを diff し、両者で一致することも
確認済みです。vehicle・way のルールは vanilla Simutrans のみで確認済みで、OTRP との個別 diff は
まだ行っていません。

対応範囲は現状:

- `lint`: `obj=building`（`type=extension` / `stop` / `depot` 系）、`obj=vehicle`、`obj=way`
- `couplings`: `obj=vehicle` の `constraint[prev]` / `constraint[next]`

### 既知の制限（意図的に非対応）

- `obj=wayobj` など building/vehicle/way 以外の obj 種別
- way の `image[new2]`（switch images判定用プローブ）・`imageup[...]`/`imageup2[...]`（坂道画像）・
  `diagonal[...]`（対角画像）の欠落検証。いずれも空文字列のまま「空画像」として書かれるだけで、
  fatal/warning の分岐が無いため対象外（`image[-]`/`image[-][0]` のみが明示的に FATAL ERROR になる特別扱い）
- way の `cursor`/`icon` 未指定検証。building と異なり、makeobj ソース上は空文字列を許容し
  fatal/warning を出さない。ツールバー表示への影響は building の「ビルドメニュー非表示」ほど
  明確な実機観察の根拠がないため見送り
- 実際の `tabfile_t::read()` がサポートするパラメータ／範囲展開構文
  （`key[0-4]=value` や `key[n,s,w]=value`）。現行パーサは最初の `=` で単純に分割するのみのため、
  この構文を使った `.dat` はキーが期待通りに展開されず、意図しない結果になる可能性があります
- `freight=` / `freightimagetype[N]=` が参照する good（貨物種別）オブジェクトの実在性検証。
  makeobj はこの参照を検証せず（`xref_writer_t::write_obj()`）、ゲーム読み込み時まで解決を
  遅延しますが、参照先はパークセット全体のどこにあってもよいため、ディレクトリ横断の
  レジストリが無い現状では検証できません
- makeobj の画像自動クロップ挙動（`image_writer.cc` の `init_dim`）の検証
- `fmt --reorder` でのコメント保持（並び替え後の位置が一意に決まらないため出力から除外）

## 開発

```
cargo test                                   # 統合テスト（tests/*.rs）
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

`testdata/` に正常系・意図的に壊した系・フォーマッタ用・連結制約用の `.dat`／`.png` を用意しています。
CI は Linux / Windows の両方でビルド・テストします。

### アーキテクチャ

```
src/
  main.rs                clap による CLI 入口（lint/fmt/couplings）
  registry.rs             Rule trait・RuleContext・obj種別ディスパッチ
  parser.rs                .dat パーサ（先勝ち・行番号追跡・重複キー検出）
  diagnostics.rs            Diagnostic・Severity・Location
  rules/
    building.rs               obj=building のRule実装
    vehicle.rs                 obj=vehicle のRule実装
    way.rs                      obj=way のRule実装
    common.rs                    共有定数・ヘルパー（KNOWN_WAYTYPES等）・duplicate-key検出
  couplings.rs              vehicle連結制約のグラフ解析（lintとは別スコープ）
  formatter/
    mod.rs                    パース・正規化ロジック
    order.rs                   obj種別ごとの並び順定義
```

各検査項目は `Rule` トレイトの実装として追加します。新しい obj 種別を追加する場合は
`rules/<obj種別>.rs` を新設し、`registry::RuleSet::for_obj_type` にディスパッチを追加してください。

## 由来

本ツールは [simutrans-addon-making-by-ai](https://github.com/128na/simutrans-addon-making-by-ai) の
`try-out/dat_linter/` で行った PoC を独立リポジトリ化し、その後アーキテクチャを再設計して
`obj=vehicle` 対応を追加したものです。設計判断・調査の経緯は try-out 側の README に記録されています。

## ライセンス

MIT License（[LICENSE](LICENSE) 参照）
