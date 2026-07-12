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
    { "obj_type": "building", "keys": ["obj", "name", "copyright", "type", "waytype", "..."] }
  ],
  "known_values": {
    "waytype": ["none", "road", "track", "..."],
    "direction": ["s", "w", "sw", "se", "n", "e", "ne", "nw"],
    "per_obj_type": [
      { "obj_type": "building", "key": "type", "values": ["res", "com", "...", "station", "..."] },
      { "obj_type": "factory", "key": "location", "values": ["land", "water", "city", "river", "shore", "forest"] },
      { "obj_type": "building", "key": "climates", "values": ["water", "desert", "...", "sea"] },
      { "obj_type": "tree", "key": "climates", "values": ["water", "desert", "...", "sea"] },
      { "obj_type": "ground_obj", "key": "climates", "values": ["water", "desert", "...", "sea"] },
      { "obj_type": "factory", "key": "climates", "values": ["water", "desert", "...", "sea"] },
      { "obj_type": "menu", "key": "name", "values": ["Button", "Roundbutton", "..."] },
      { "obj_type": "cursor", "key": "name", "values": ["Builder", "GeneralTools", "Marked", "BigLogo", "..."] },
      { "obj_type": "symbol", "key": "name", "values": ["Seasons", "MessageOptions", "...", "BigLogo", "..."] },
      { "obj_type": "misc", "key": "name", "values": ["PowerDest", "PowerSource", "..."] },
      { "obj_type": "ground", "key": "name", "values": ["Shore", "ClimateTexture", "..."] }
    ]
  }
}
```

- `obj_types` — `registry::SUPPORTED_OBJ_TYPES`（22件）の各要素について、その obj 種別で
  有効なキー一覧（`name`/`copyright`を含む、全obj種別共通）を返します。
- `known_values.waytype`/`known_values.direction` — 特定のキーが取りうる既知の値一覧です。
  `waytype`は`get_waytype()`が受理する13値、`direction`はvehicle/citycar/pedestrianが
  共有する8方向（`.dat`上は`emptyimage[s]`のようにキー名の添字として現れるため、
  `direction`という仮想的な名前で値一覧のみを提供します）。**どのobj種別でも同じ値集合**
  であるため、`obj_type`を持たないフラットな配列です。
- `known_values.per_obj_type` — `type=`/`location=`/`climates=`/`name=`（skin系obj種別）
  のように、**同じキー名でもobj種別によって意味・値集合が異なる**キーの既知値一覧です。
  `{ "obj_type": "...", "key": "...", "values": [...] }`の3つ組の配列で、
  `(obj_type, key)`の組が一意になります。
  - `(building, type)` — `building_writer.cc`の`type=`分岐（119-201行目）が受理する
    既知値。現行有効値（`res`/`com`/`ind`/`cur`/`mon`/`tow`/`hq`/`habour`/`harbour`/
    `dock`/`fac`/`stop`/`extension`/`depot`/`any`）とobsolete値
    （`station`/`railstop`/`monorailstop`/`busstop`/`carstop`/`airport`/`wharf`/
    `hall`/`post`/`shed`。makeobjは構文として認識するがFATAL ERRORにする）の両方を
    含みます。obsolete値の妥当性は引き続き`lint`の`obsolete-type`診断が担います。
  - `(factory, location)` — `factory_writer.cc`の`location=`分岐（156-164行目）が
    受理する6値（`land`/`water`/`city`/`river`/`shore`/`forest`）。いずれにも
    一致しない場合は`Land`へ黙ってフォールバックするだけでobsolete区分はありません。
  - `(building, climates)`/`(tree, climates)`/`(ground_obj, climates)`/
    `(factory, climates)` — `get_climate_bits()`（`get_climate.cc`）が受理する
    `climates=`の既知値。8気候名（`water`/`desert`/`tropic`/`mediterran`/
    `temperate`/`tundra`/`rocky`/`arctic`）に、`water`と同一ビットへ解決される
    同義語`sea`を加えた9値です。
  - `(menu, name)`/`(cursor, name)`/`(symbol, name)`/`(misc, name)`/`(ground, name)` —
    これらは`.dat`のkey=valueフィールドではなく、そのobjectを識別する**トップレベルの
    `name=`**が一致しうる特殊値です。根拠はmakeobj本体（コンパイル時の
    `descriptor/writer/`）ではなく**ゲーム本体のランタイムコード**
    （`src/simutrans/simskin.cc`のmenu/cursor/symbol/misc向け特殊オブジェクトテーブル、
    `src/simutrans/descriptor/ground_desc.cc`のground向けテーブル）です。照合は
    大文字小文字を**区別する**`strcmp`（`spezial_obj_tpl.h`）で行われ、他の多くの
    フィールドが使う大文字小文字を区別しない`STRICMP`とは異なります。
    `cursor`/`symbol`は固有の名前一覧に加え、両者で共有される
    `fakultative_objekte`（21値、`BigLogo`/`Mouse`/`TrainStop`等）も含みます。
    一致しない`name=`はfatal/warningにはならず、単に特殊なUI要素に紐づかないだけです
    （lintの検証対象ではありません）。

`lint --format json` と同じく、JSON出力はstdoutへの1回きりの
`serde_json::to_string`のみで、stderrへは何も出しません。

## スコープ外

- `type=`/`location=`/`climates=`/skin系`name=`以外の、値が取りうる集合の一覧
  （`waytype`/`direction`以外）は今回の対象外です（値の意味論的妥当性は引き続き
  `lint`が担います）。
- `roadsign`の`is_signal`等、他のobj種別が持つ0/1フラグ的なキーの値一覧化は対象外
  です（enum的な値集合を持たないフィールドのため）。
- キーの必須/任意・欠落時の挙動（fatal/warning/サイレントフォールバック等）は
  `lint`が担う領域であり、`keys`はキー名・既知値の一覧のみを返します。
