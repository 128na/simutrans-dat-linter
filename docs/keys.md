# `keys` — obj種別ごとの有効キー一覧

```
dat_linter keys                    # 人間可読な一覧表示
dat_linter keys --format json      # VSCode拡張等の機械可読な消費者向けJSON出力
```

VSCode拡張のシンタックスハイライト・スニペット機能の「唯一の正」データソースにするための
コマンドです。obj種別ごとに、その `.dat` で実際に有効なキー一覧を宣言的に返します。
一覧の中身（各obj種別のキー）自体は `src/rules/keys.rs` が唯一の正であり、
`registry::SUPPORTED_OBJ_TYPES`（= `ObjType` の全22種別）を必ず1つずつカバーします
（ワイルドカードなしの網羅matchでコンパイル時に強制。`tests/obj_type_coverage.rs`の
`keys_for_all_obj_types_are_well_formed` が空でない・`name`/`copyright`を含む・
重複が無いことを保証）。

各キーの正しさの根拠は `src/rules/<obj種別>.rs` のドキュメントコメントと
`src/formatter/order.rs` の `<OBJ>_NAMED`/`<OBJ>_*_ORDER`（`descriptor/writer/*.cc`の
フィールド読み取り順から導出した一覧）にあります。`frontimage[l][y][x][h][phase]`の
ような角括弧添字付きキーは、添字を除いたベース名（`frontimage`）のみを保持します。

## `--format json` のスキーマ

```json
{
  "obj_types": [
    { "obj_type": "building", "keys": ["name", "copyright", "type", "waytype", "..."] }
  ],
  "known_values": {
    "waytype": ["none", "road", "track", "..."],
    "direction": ["s", "w", "sw", "se", "n", "e", "ne", "nw"]
  }
}
```

- `obj_types` — `registry::SUPPORTED_OBJ_TYPES`（22件）の各要素について、その obj 種別で
  有効なキー一覧（`name`/`copyright`を含む、全obj種別共通）を返します。
- `known_values` — 特定のキーが取りうる既知の値一覧です。`waytype`は`get_waytype()`が
  受理する13値、`direction`はvehicle/citycar/pedestrianが共有する8方向
  （`.dat`上は`emptyimage[s]`のようにキー名の添字として現れるため、`direction`という
  仮想的な名前で値一覧のみを提供します）。

`lint --format json` と同じく、JSON出力はstdoutへの1回きりの
`serde_json::to_string`のみで、stderrへは何も出しません。

## スコープ外

- `type=`/`location=`等、キーが取りうる値の一覧そのもの（`waytype`/`direction`以外）は
  今回の対象外です（値の意味論的妥当性は引き続き`lint`が担います）。
- キーの必須/任意・欠落時の挙動（fatal/warning/サイレントフォールバック等）は
  `lint`が担う領域であり、`keys`はキー名の一覧のみを返します。
