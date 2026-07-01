# simutrans-dat-linter

Simutrans アドオンの `.dat`（オブジェクト定義ファイル）を **pak 化する前に** 静的検証する Rust 製 CLI ツールです。

`makeobj` はパラメーター不足・矛盾をほぼ無視して pak を生成してしまい、ゲーム内で初めて不具合に気付く
→ 原因調査に時間がかかる、という問題があります。このツールは makeobj の C++ ソース
（`building_writer.cc` / `get_waytype.cc` / `image_writer.cc` / `tabfile.cc` / `vehicle_writer.cc`）を
精読し、**makeobj が黙って見逃す／FATAL ERROR にする項目**を、Blender→PNG→pak のフルパイプラインを
回さずに一瞬で検出します。makeobj には依存せず、`.dat` 構文を独自に解析します。

## 3層の役割

| 層 | サブコマンド | 役割 | 例え |
|---|---|---|---|
| formatter | `fmt` | 見やすく整形（キー小文字化・`=`前後の空白除去・任意で並び替え） | — |
| linter | `lint` | pak 化に失敗する／ゲーム内で正しく表示されない項目を検知 | `php -l` |
| 静的解析 | `couplings` | 実行しなくても分かるゲーム内利用時の問題を検知 | PHPStan |

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

### `lint` — building dat の静的検証

```
dat_linter <path/to/file.dat>
dat_linter lint -v  <path/to/file.dat>   # info まで表示
dat_linter lint -vv <path/to/file.dat>   # debug（生の値・解決後パス）まで表示
```

検出する主な項目（すべて makeobj ソースで裏付け済み）:

- `cursor` と `icon` が両方未指定 → ビルドメニューに表示されない（makeobj はエラーを出さない）
- タイルに front/back image が1枚もない → 空画像タイルが黙って生成される
- `type` が obsolete（`station` / `hall` / `post` …）や未知の値 → FATAL ERROR
- `type=stop` / `type=depot` で `waytype` 未指定・不正 → FATAL ERROR
- `Dims` のサイズが 0
- 参照画像が見つからない／サイズが 128 の倍数でない → FATAL ERROR
- `=` 直後のスペースなど、値に混入した空白による参照失敗

### `fmt` — フォーマッタ

```
dat_linter fmt <file.dat>              # 順序を保ったまま正規化（標準出力へ）
dat_linter fmt --reorder <file.dat>    # 慣習的な順序に並び替え
dat_linter fmt --write   <file.dat>    # ファイルに上書き（-w も可）
```

安全な正規化（キー小文字化・`=`前後の空白除去・値の前後トリム）のみ行い、値の内容変更のような
壊しうる操作は行いません。並び替え（`--reorder`）はスタイル上の慣習であり makeobj の動作には影響しないため、
オプトインです（コメント・空行は並び替え後の位置が一意に決まらないため出力から除外し件数を警告します）。

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

各ルールは makeobj の C++ ソースで根拠を確認しています（詳細は `src/rules.rs` 冒頭コメント参照）。
vanilla Simutrans と OTRP（Simutrans-Extended 系フォーク）の該当ファイルを diff し、building dat の
検証に関わるロジックが両者で一致することも確認済みです。

対応範囲は現状:

- `lint`: `obj=building` の `type=extension` / `stop` / `depot` 系
- `couplings`: `obj=vehicle` の `constraint[prev]` / `constraint[next]`

`way` など他の obj 種別への展開、makeobj の自動クロップ挙動の検証は今後の課題です。

## 開発

```
cargo test                       # 統合テスト（tests/building.rs, fmt.rs, couplings.rs）
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

`testdata/` に正常系・意図的に壊した系・フォーマッタ用・連結制約用の `.dat`／`.png` を用意しています。

## 由来

本ツールは [simutrans-addon-making-by-ai](https://github.com/128na/simutrans-addon-making-by-ai) の
`try-out/dat_linter/` で行った PoC を独立リポジトリ化したものです。設計判断・調査の経緯は
try-out 側の README に記録されています。

## ライセンス

MIT License（[LICENSE](LICENSE) 参照）
