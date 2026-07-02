# simutrans-dat-linter

Simutrans アドオンの `.dat`（オブジェクト定義ファイル）を **pak 化する前に** 静的検証する Rust 製 CLI ツールです。

`makeobj` はパラメーター不足・矛盾をほぼ無視して pak を生成してしまい、ゲーム内で初めて不具合に気付く
→ 原因調査に時間がかかる、という問題があります。このツールは makeobj の C++ ソース
（`building_writer.cc` / `vehicle_writer.cc` / `way_writer.cc` / `good_writer.cc` / `bridge_writer.cc` /
`tunnel_writer.cc` / `roadsign_writer.cc` / `crossing_writer.cc` / `way_obj_writer.cc` /
`groundobj_writer.cc` / `tree_writer.cc` / `citycar_writer.cc` / `pedestrian_writer.cc` /
`factory_writer.cc` /
`get_waytype.cc` / `get_climate.cc` /
`image_writer.cc` / `imagelist_writer.cc` / `imagelist2d_writer.cc` / `xref_writer.cc` /
`skin_writer.cc` / `tabfile.cc`）を精読し、
**makeobj が黙って見逃す／FATAL ERROR にする項目**を、Blender→PNG→pak のフルパイプラインを回さずに
一瞬で検出します。makeobj には依存せず、`.dat` 構文を独自に解析します。

`lint` は building/vehicle/way/good/bridge/tunnel/roadsign/crossing/way-object/ground_obj/tree/
citycar/pedestrian/factory の全14 obj種別（このプロジェクトが事前に合意した対応範囲）をカバーしています。

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

### `lint` — 静的検証（`obj=building` / `obj=vehicle` / `obj=way` / `obj=good` / `obj=bridge` / `obj=tunnel` / `obj=roadsign` / `obj=crossing` / `obj=way-object` / `obj=ground_obj` / `obj=tree` / `obj=citycar` / `obj=pedestrian` / `obj=factory`）

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

**good**（貨物・カーゴ種別）: `good_writer.cc`（`write_obj`本体）を精読した結果、
building/vehicle/way と異なり **makeobj時点でfatal/warningになる分岐が1つも存在しない**ことを確認済みです
（`waytype`自体を読まない、`value`/`catg`/`speed_bonus`/`weight_per_unit`/`mapcolor`は全て
`get_int`/`get_int64`の無条件フォールバックのみで`get_int_clamped`は不使用、`name`/`copyright`/`metric`も
`text_writer_t`が空文字列を無条件許容）。そのためobj種別固有のルールは追加していません
（`src/rules/good.rs`冒頭のREJECTEDコメントに調査過程を記録）。`good`を`lint`の対象として登録したことで、
下記の**obj種別を問わず**適用される重複キー検出だけは`good` datにも有効になります。

**bridge** で検出する主な項目（`bridge_writer.cc` / `get_waytype.cc` / `imagelist_writer.cc` /
`image_writer.cc` / `tabfile.cc` で裏付け済み）:

- `waytype` 未指定・不正 → FATAL ERROR（vehicle/wayと同様、`obj=bridge` では常に必須）
- `pillar_distance` / `pillar_asymmetric` / `max_lenght` / `max_length` / `max_height` /
  `axle_load` / `clip_below` / `intro_year` / `intro_month` / `retire_year` / `retire_month` が
  各フィールドの許容範囲外 → fatal/errorなしで黙って範囲内にクランプされる
  （`tabfileobj_t::get_int_clamped()` のWARNING。wayの`clip_below`と同じ仕組みが
  bridgeでは数値フィールドのほぼ全てに使われている）
- `front{image,start,ramp,pillar}[...]`（無季節・雪季節版计24キー、`backimage[ns][0]`等の
  季節画像有無で対象季節数が変わる）が2文字以下（未指定や`"-"`含む）→ fatal ではないが
  `dbg->warning(..., "No %s specified (might still work)")` が出る（`no-bridge-image-specified`,
  WARNING）。back側の画像には対応する警告分岐が無いためfront側のみ検出
- front画像が実際に画像を指している場合、参照画像が見つからない／サイズが128の倍数でない
  → FATAL ERROR（building/wayと同じ`check_image_ref`を共有）

**tunnel** で検出する主な項目（`tunnel_writer.cc` / `get_waytype.cc` / `imagelist_writer.cc` /
`skin_writer.cc` / `image_writer.cc` / `tabfile.cc` で裏付け済み）:

- `waytype` 未指定・不正 → FATAL ERROR（vehicle/way/bridgeと同様、`obj=tunnel` では常に必須）
- `{front|back}image[{方向}{幅}][{season}]`（`frontimage[n][1]`の有無で季節数、
  `frontimage[nl][0]`（無ければ短縮形`frontimage[nl]`）の有無でbroad portal（4幅）/
  narrow portal（1幅）を判定し、対象season数×2面×portal幅×4方向を機械的に走査する）が
  実際に画像を指している場合、参照画像が見つからない／サイズが128の倍数でない
  → FATAL ERROR（building/way/bridgeと同じ`check_image_ref`を共有）
- bridgeと異なり、tunnelの数値フィールド（`topspeed`/`cost`/`maintenance`/`axle_load`/
  `intro_year`/`intro_month`/`retire_year`/`retire_month`）は全て`get_int`/`get_int64`の
  無条件フォールバックのみで`get_int_clamped`は不使用のため、クランプ系の警告ルールは無し
  （詳細は`src/rules/tunnel.rs`冒頭のREJECTEDコメント参照）

**roadsign**（信号機・道路標識）で検出する主な項目（`roadsign_writer.cc` / `roadsign_writer.h` /
`roadsign_desc.h` / `get_waytype.cc` / `imagelist_writer.cc` / `image_writer.cc` / `tabfile.cc` で
裏付け済み）:

- `waytype` 未指定・不正 → FATAL ERROR（vehicle/way/bridge/tunnelと同様、`obj=roadsign` では常に必須）
- 画像キーは`image[0]`の有無で **numbered構文** と **2D構文** のどちらかに完全に排他分岐する
  （`.dat`記述者がどちらを使うか選ぶのではなく、`image[0]`が非空かどうかでmakeobj側が自動判定する）:
  - **numbered構文**（`image[0]`が非空）: `image[0]`, `image[1]`, ... と連番で走査し、最初の空きキーの
    インデックスが4の倍数でなければ FATAL ERROR（`"image count is %d but must be multiple of 4!"`）
  - **2D構文**（`image[0]`が空、通常の書式）: `image[{方向}][{state}]`を`state=0..8`まで走査する。
    方向セットは`is_private=1`なら`["ns","ew"]`（2方向）、`image[ne][0]`が非空なら信号機ではなく
    交通信号灯と推定して8方向、それ以外は`["n","s","w","e"]`の4方向。state=0の全方向
    （`is_private=1`の場合はstate=1も）は必須で、欠けていると FATAL ERROR（`"%s is missing!"`）。
    それ以降のstateはそのstateの最初の方向のキーが無ければ「以降のstateは無い」とみなして走査終了
    （fatalにならない）が、途中の方向だけが欠けている場合は同じ FATAL ERROR になる
- 画像キーが実際に画像を指す場合、参照画像が見つからない／サイズが128の倍数でない → FATAL ERROR
  （building/way/bridge/tunnelと同じ`check_image_ref`を共有）
- roadsignは`get_int_clamped`を一切使わない（`min_speed`/`offset_left`/`cost`/`maintenance`/
  `intro_year`/`intro_month`/`retire_year`/`retire_month`は全て無条件フォールバック）ため、
  bridgeのようなクランプ系警告ルールは無い（詳細は`src/rules/roadsign.rs`冒頭のREJECTEDコメント参照）

**crossing**（2つのwayが交差する踏切/交差点）で検出する主な項目（`crossing_writer.cc` /
`crossing_writer.h` / `get_waytype.cc` / `imagelist_writer.cc` / `xref_writer.cc` /
`obj_writer.cc` / `tabfile.cc` で裏付け済み）:

- `waytype[0]` / `waytype[1]` 未指定・不正 → FATAL ERROR（他のobj種別と同じ`get_waytype()`
  経由。crossingはwaytypeが1つでなく**2つ**あり、交差する2本のwayを表す点が他と異なる）
- `waytype[0]` と `waytype[1]` が解決後の`waytype_t`列挙値として同一 → FATAL ERROR
  （`"Identical ways (%s) cannot cross (check waytypes)!"`）。文字列としての一致ではなく
  解決後の値の一致で判定されるため、`schiene_tram` と `tram_track` のように別名でも
  同じ列挙値（`tram_wt`）に解決される組み合わせも検出する
- `speed[0]` / `speed[1]`（いずれも`get_int(key, 0)`）のどちらか一方でも0（未指定含む）
  → FATAL ERROR（`"A maxspeed MUST be given for both ways!"`）
- `openimage[ns][0]` / `openimage[ew][0]`（`make_list`による`[0][1][2]...`連番走査）が
  どちらも空リスト（インデックス0から空） → FATAL ERROR
  （`"Missing images (at least one openimage!...)"`）。`front_openimage[ns/ew]` /
  `closedimage[ns/ew]` / `front_closedimage[ns/ew]`は同じ連番走査方式だが
  ソースコード上「optional」と明示されており対応するfatal/warning分岐が無いため対象外
- 画像キーが実際に画像を指す場合、参照画像が見つからない／サイズが128の倍数でない
  → FATAL ERROR（building/way/bridge/tunnel/roadsignと同じ`check_image_ref`を共有）
- crossingは`get_int_clamped`を一切使わない（`animation_time_open`/`animation_time_closed`/
  `intro_year`/`intro_month`/`retire_year`/`retire_month`は全て無条件フォールバック）ため、
  bridgeのようなクランプ系警告ルールは無い（詳細は`src/rules/crossing.rs`冒頭のREJECTED
  コメント参照）
- crossingには`cursor`/`icon`フィールドへの言及がソース上に一つも無いため、
  他のobj種別と異なりそもそもcursor/icon関連のルールは存在しない

**way-object**（架線柱・照明など、wayに付随して描画されるオブジェクト。`.dat`に実際に書く
`obj=`の値は`way-object`。詳細は下記コラム参照）で検出する主な項目（`way_obj_writer.cc` /
`way_obj_writer.h` / `get_waytype.cc` / `imagelist_writer.cc` / `image_writer.cc` /
`skin_writer.cc` / `obj_writer.cc` / `tabfile.cc` で裏付け済み）:

- `waytype` / `own_waytype` のいずれか未指定・不正 → FATAL ERROR（crossingと同様、
  way-objectは他のobj種別と異なりwaytypeフィールドが**2つ**ある。`waytype`はこの
  way-objectが乗る対象のwayの種別、`own_waytype`はこのway-object自身が表す種別
  （架線なら`electrified_track`等）で、意味は異なるがどちらも同じ`get_waytype()`
  経由でFATALになる）
- 画像キー（`{front|back}image[{ribi}]`26方向×2 / `{front|back}imageup[{slope}]`
  4段×2 / `{front|back}imageup2[{slope}]`4段×2 / `{front|back}diagonal[{ribi}]`
  4方向×2 / `cursor` / `icon`）が実際に画像を指す場合、参照画像が見つからない／
  サイズが128の倍数でない → FATAL ERROR（building/way/bridge/tunnel/roadsign/
  crossingと同じ`check_image_ref`を共有）
- way-objectは`get_int_clamped`を一切使わない（`cost`/`maintenance`/`topspeed`/
  `intro_year`/`intro_month`/`retire_year`/`retire_month`は全て無条件フォールバック）
  ため、bridgeのようなクランプ系警告ルールは無い（詳細は`src/rules/way_obj.rs`
  冒頭のREJECTEDコメント参照）
- wayの`image[-]`のような「最低1枚必須」の明示的なFATALがway_obj_writer.cc上に
  存在しないため、画像が1枚も無くてもmakeobj時点ではエラーにならない（詳細は
  `src/rules/way_obj.rs`冒頭のREJECTEDコメント参照）

> **`obj=`文字列について**: このプロジェクトのRustモジュール名・ファイル名は
> `way_obj`（スネークケース、makeobjの`way_obj_writer.cc`ファイル名に合わせている）
> だが、`.dat`に実際に書く`obj=`の値は**`way-object`**（ハイフン区切り）である。
> 根拠は`way_obj_writer_t::get_type_name()`（`way_obj_writer.h`）が
> `return "way-object";`を返し、`obj_writer_t::write`（`obj_writer.cc`）が
> `obj.get("obj")`の文字列でそのままこの`get_type_name()`の登録名を引く実装のため。
> pak128等の公開`.dat`ファイルでも`obj=way-object`が使われていることを確認済み。

**ground_obj**（岩・廃墟・草むらなどの地面装飾オブジェクト。`.dat`に実際に書く`obj=`の値は
`ground_obj`。詳細は下記コラム参照）で検出する主な項目（`groundobj_writer.cc` /
`groundobj_writer.h` / `get_waytype.cc` / `get_climate.cc` / `imagelist2d_writer.cc` /
`imagelist_writer.cc` / `image_writer.cc` / `obj_writer.cc` / `tabfile.cc` で裏付け済み）:

- `waytype`は他のobj種別と異なり**省略可能**です。空文字列/未指定の場合は`get_waytype()`
  自体が呼ばれず`ignore_wt`にサイレントフォールバックするため FATAL になりません
  （building/vehicle/way/bridge/tunnel/roadsign/crossing/way-objectは全て必須ですが、
  ground_objだけはこの分岐が非対称です）。ただし非空の値を指定した場合、それが既知の
  waytypeでなければ`get_waytype()`内で FATAL ERROR になります
- 画像キーは`speed`（`get_int("speed", 0)`）の値で挙動が分岐します:
  - `speed==0`（固定物、岩・草むら等）: `image[<phase>][<season>]`をphase=0,1,2,...と
    走査し、あるphaseの`season 0`画像が空文字列ならそのphaseで走査終了（**FATALに
    ならない**。画像0枚のground_objも許容されます）。ただし`season 0`画像が非空なのに
    それ以降のseason画像が空文字列だと FATAL ERROR（`"Season image for season %i
    missing!"`）
  - `speed!=0`（移動物、鳥・羊等）: 8方向（`s`/`w`/`sw`/`se`/`n`/`e`/`ne`/`nw`）×
    seasons分の`image[<dir>][<season>]`が全て必須で、いずれか1つでも空文字列だと
    即 FATAL ERROR（`"Season image for season %i missing (expected %s)!"`）
- 画像キーが実際に画像を指す場合、参照画像が見つからない／サイズが128の倍数でない
  → FATAL ERROR（building/way/bridge/tunnel/roadsign/crossing/way-objectと同じ
  `check_image_ref`を共有）
- ground_objは`get_int_clamped`を一切使わない（`seasons`/`distributionweight`/`cost`/
  `speed`/`trees_on_top`は全て無条件フォールバック）ため、bridgeのようなクランプ系
  警告ルールは無い（詳細は`src/rules/groundobj.rs`冒頭のREJECTEDコメント参照）
- `climates`未指定時に`dbg->warning("No climates (using default)!")`という分岐が
  ソース上に存在するが、`tabfileobj_t::get()`は欠落キーにもNULLではなく空文字列を
  返すため実行時にこの分岐へ到達しない（実際には常にtrue側に入る）。よって
  climates未指定の警告ルールは追加していない（詳細は同REJECTEDコメント参照）
- ground_objには`cursor`/`icon`フィールドへの言及がソース上に一つも無いため、
  crossingと同様にcursor/icon関連のルールは存在しない

> **`obj=`文字列について**: このプロジェクトのRustモジュール名・ファイル名は
> `groundobj`（スネークケース、アンダースコアなし）だが、`.dat`に実際に書く`obj=`の
> 値は**`ground_obj`**（アンダースコア区切り）である。`groundobj_writer.cc`という
> ファイル名から安易に類推すると`"groundobj"`と誤りやすいため注意が必要。根拠は
> `groundobj_writer_t::get_type_name()`（`groundobj_writer.h`）が
> `return "ground_obj";`を返し、`obj_writer_t::write`（`obj_writer.cc`）が
> `obj.get("obj")`の文字列でそのままこの`get_type_name()`の登録名を引く実装のため。
> pak128・pak144.Excentrique・Pak192.Comic・pak72.Elegance等の公開`.dat`ファイルでも
> `obj=ground_obj`が使われていることを確認済み（`obj=groundobj`のGitHub code
> search結果は0件でした）。

**tree**（樹木の景観オブジェクト。`.dat`に実際に書く`obj=`の値は`tree`）で検出する
主な項目（`tree_writer.cc` / `tree_writer.h` / `get_climate.cc` / `imagelist2d_writer.cc` /
`imagelist_writer.cc` / `image_writer.cc` / `obj_writer.cc` / `tabfile.cc` で裏付け済み）:

- 画像は`age`（0..4の固定5段階）×`season`（`seasons`で指定した`0..number_of_seasons-1`、
  既定値1）の全組み合わせで`image[<age>][<season>]`が必須です。1つでも空文字列
  （キー欠落含む）だと FATAL ERROR（`"Missing image[<age>][<season>]!"`）になります。
  groundobjのようなphase単位の早期終了は無く、常に5段階×seasons分がまるごと必須です
- 画像キーが実際に画像を指す場合、参照画像が見つからない／サイズが128の倍数でない
  → FATAL ERROR（building/way/bridge/tunnel/roadsign/crossing/way-object/ground_objと
  同じ`check_image_ref`を共有）
- treeは`get_int_clamped`を一切使わない（`seasons`/`distributionweight`は全て
  無条件フォールバック）ため、bridgeのようなクランプ系警告ルールは無い（詳細は
  `src/rules/tree.rs`冒頭のREJECTEDコメント参照）
- `climates`未指定時に`printf("WARNING: old syntax without climates!\n")`という分岐が
  ソース上に存在するが、`tabfileobj_t::get()`は欠落キーにもNULLではなく空文字列を
  返すため実行時にこの分岐へ到達しない（ground_objの`climates`警告と全く同じ理由）。
  よってclimates未指定の警告ルールは追加していません
- treeには`cursor`/`icon`/`waytype`フィールドへの言及がソース上に一つも無いため、
  crossing/ground_objと同様にcursor/icon関連のルールは存在せず、goodと同様に
  waytype関連のルールも存在しません（樹木はビルドメニューから選択して建てるもの
  ではなく、マップ生成時に自動配置されるscenery objectのため）

> **`obj=`文字列について**: `tree_writer_t::get_type_name()`（`tree_writer.h`）は
> `return "tree";`を返し、ファイル名`tree_writer.cc`から素直に導ける文字列と一致
> していました（way-object・ground_objのようなファイル名からの単純な類推が外れる
> 前例があったため、今回も念のため実際に確認しています）。pak128・
> pak144.Excentrique・Pak192.Comic等の公開`.dat`ファイルでも`obj=tree`が
> 使われていることを確認済みです。

**citycar**（プレイヤー非所有の私有車。街に自動で出現する乗用車。`.dat`に実際に書く
`obj=`の値は`citycar`）で検出する主な項目（`citycar_writer.cc` / `citycar_writer.h` /
`citycar_desc.h` / `imagelist_writer.cc` / `image_writer.cc` / `obj_writer.cc` /
`tabfile.cc` で裏付け済み）:

- 画像は`s`/`w`/`sw`/`se`/`n`/`e`/`ne`/`nw`の固定8方向について`image[<dir>]`が
  実際に画像を指す場合、参照画像が見つからない／サイズが128の倍数でない → FATAL ERROR
  （building/way/bridge/tunnel/roadsign/crossing/way-object/ground_obj/treeと同じ
  `check_image_ref`を共有）
- vehicleの`emptyimage[dir]`と異なり、citycarの8方向`image[<dir>]`走査
  （`for (i = 0; i < 8; i++)`）は無条件で早期終了ロジックが無いため、8方向のうち
  一部だけが定義されている状態や画像が1枚も無い状態を検出するfatal/warning分岐は
  存在しない（詳細は`src/rules/citycar.rs`冒頭のREJECTEDコメント参照）
- citycarは`obj=vehicle`と異なり`waytype`/`engine_type`/`freight`/
  `freightimage[...]`/`freightimagetype[...]`/`constraint[prev]`/`constraint[next]`の
  いずれへの言及も`citycar_writer.cc`全文に一つも無い。プレイヤーが編成する
  概念を持たない自動生成NPC車両のため、vehicleの`couplings`サブコマンドが対象とする
  連結制約という問題設定自体がcitycarには存在しない
- citycarは`get_int_clamped`を一切使わない（`distributionweight`/`intro_year`/
  `intro_month`/`retire_year`/`retire_month`/`speed`は全て無条件フォールバック）
  ため、bridgeのようなクランプ系警告ルールは無い（詳細は`src/rules/citycar.rs`
  冒頭のREJECTEDコメント参照）
- citycarには`cursor`/`icon`フィールドへの言及がソース上に一つも無いため、
  crossing/ground_obj/treeと同様にcursor/icon関連のルールは存在しない

**pedestrian**（プレイヤー非所有のNPC歩行者。街路上に自動で出現する。`.dat`に実際に書く
`obj=`の値は`pedestrian`）で検出する主な項目（`pedestrian_writer.cc` / `pedestrian_writer.h` /
`pedestrian_desc.h` / `imagelist_writer.cc` / `imagelist2d_writer.cc` / `image_writer.cc` /
`obj_writer.cc` / `tabfile.cc` で裏付け済み）:

- pedestrianはcitycarと同じくNPC的なobj種別だが、画像は**静止画像**と
  **アニメーション画像**の2つの排他的な分岐を持つ。8方向
  （`s`/`w`/`sw`/`se`/`n`/`e`/`ne`/`nw`）のいずれかで`image[<dir>][0]`が
  非空なら「アニメーション画像あり」と判定され、全体がアニメーション分岐になる
  （方向ごとに個別選択できるわけではない）
  - **静止分岐**（全方向で`image[<dir>][0]`が空。pak128の実例はすべてこちら）:
    `image[<dir>]`を8方向全て無条件に読む（citycarの`image[<dir>]`ループと
    全く同じ構造。早期終了なし）
  - **アニメーション分岐**（いずれかの方向で`image[<dir>][0]`が非空）:
    `image[<dir>][<frame>]`をframe=0から最初の空文字列まで方向ごとに
    独立して走査する。ある方向だけ`image[<dir>][0]`が空でも、その方向が
    0フレームのまま許容される（fatal/warningにならない）
  - いずれの分岐でも、実際に画像を指すキーについては参照画像が見つからない／
    サイズが128の倍数でない → FATAL ERROR（building/way/bridge/tunnel/
    roadsign/crossing/way-object/ground_obj/tree/citycarと同じ`check_image_ref`
    を共有）
- pedestrianは`obj=vehicle`と異なり`waytype`/`engine_type`/`freight`/
  `freightimage[...]`/`freightimagetype[...]`/`constraint[prev]`/
  `constraint[next]`のいずれへの言及も`pedestrian_writer.cc`全文に一つも無い
  （citycarと同様、プレイヤーが編成する概念を持たないNPCのため）
- pedestrianは`get_int_clamped`を一切使わない（`distributionweight`/`offset`/
  `intro_year`/`intro_month`/`retire_year`/`retire_month`は全て無条件
  フォールバック）ため、bridgeのようなクランプ系警告ルールは無い。
  `steps_per_frame`は`max(get_int(...), 1)`というC++標準`max()`による
  インラインの下限クランプを持つが、`dbg->warning`等のメッセージを一切
  伴わないため、`get_int_clamped`ベースの`ClampedRangeRule`とは根拠の強さが
  異なると判断し対象外とした（詳細は`src/rules/pedestrian.rs`冒頭のREJECTED
  コメント参照）
- pedestrianには`cursor`/`icon`フィールドへの言及がソース上に一つも無いため、
  crossing/ground_obj/tree/citycarと同様にcursor/icon関連のルールは存在しない

**factory**（生産チェーンを持つ産業施設。`inputgood`/`outputgood`で貨物を消費・生産する）で
検出する主な項目（`factory_writer.cc` / `factory_writer.h` / `factory_desc.h` /
`building_writer.cc`（factoryが直接呼び出す共有経路） / `get_climate.cc` / `xref_writer.cc` /
`dataobj/tabfile.cc` で裏付け済み）:

- factoryは`factory_writer_t::write_obj`内で**`building_writer_t::write_obj`を
  同じ`tabfileobj_t`ごとそのまま呼び出す**ため、buildingが検証する`Dims`のサイズ0
  （FATAL ERROR）・タイル画像（`{front|back}image[layout][y][x][h][phase][season]`）が
  1枚も無いタイル・`cursor`/`icon`両方未指定は、factoryの`.dat`にもそのまま同じ形式・
  同じ重大度で適用される
- `type=`を明示的に指定すると、factory_writer.cc の`obj.put("type","fac")`が
  `tabfileobj_t::put()`の先勝ち仕様により**静かに失敗**し、building_writer側が
  その明示値のまま分岐する → obsolete型ならFATAL ERROR、`fac`以外の既知型
  （`res`/`com`等）なら**factoryとして機能しない建物が黙って生成される**
  （`factory-type-override`, ERROR）。pak128の公開`.dat`は例外なく`type`を
  省略していることを確認済み
- `mapcolor`が未指定（デフォルト255のまま） → FATAL ERROR
  （`"%s missing an identification color! (mapcolor)"`）。255を明示指定した場合との
  区別はmakeobj自体が行わない
- `outputgood[N]`が定義されているのに対応する`outputcapacity[N]`が11未満
  → `dbg->error`（非fatal、ログに出るがpak生成は継続）。明示的なエラー
  メッセージを伴う観測可能な分岐のためWarningとして検出する
  （`factory-output-capacity-too-small`, WARNING）
- インデックス形式の`smoketile[N]`が定義されているのに対応する`smokeoffset[N]`が
  未指定 → 同じく`dbg->error`（非fatal）を根拠にWARNINGとして検出する
  （`factory-smoketile-without-offset`, WARNING）
- `probability_to_spawn` / `expand_probability` が10000以上 → `printf`による
  固定メッセージを出力してから10000へサイレントクランプされる。`get_int_clamped`
  ではないが、tree/ground_objの`climates`警告のような到達不能コードではなく
  常に到達可能、かつpedestrianの`steps_per_frame`のような完全に無言のクランプ
  でもない（実際にメッセージを出す）という2点で両者と区別し、Warningとして
  採用する（`factory-probability-clamped`, WARNING）
- 画像キーが実際に画像を指す場合、参照画像が見つからない／サイズが128の倍数でない
  → FATAL ERROR（building等と同じ`check_image_ref`を共有）

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
並び順は `obj=` の値ごとに定義されており（`building`/`vehicle`/`way`/`good`/`bridge`/`tunnel`/`roadsign`/
`crossing`/`way-object`/`ground_obj`/`tree`/`citycar`/`pedestrian`/`factory` に対応）、未対応の obj 種別では並び替えを行わず元の順序のまま出力します。

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
`src/rules/vehicle.rs` / `src/rules/way.rs` / `src/rules/good.rs` / `src/rules/bridge.rs` /
`src/rules/tunnel.rs` / `src/rules/roadsign.rs` / `src/rules/crossing.rs` /
`src/rules/way_obj.rs` / `src/rules/groundobj.rs` / `src/rules/tree.rs` /
`src/rules/citycar.rs` / `src/rules/pedestrian.rs` / `src/rules/factory.rs` 冒頭コメント参照）。
building のルールは vanilla Simutrans と OTRP（Simutrans-Extended 系フォーク）の該当ファイルを diff し、
両者で一致することも確認済みです。vehicle・way・good・bridge・tunnel・roadsign・crossing・way-object・
ground_obj・tree・citycar・pedestrian・factory のルールは vanilla Simutrans のみで確認済みで、OTRP との個別 diff はまだ行っていません。

対応範囲は現状:

- `lint`: `obj=building`（`type=extension` / `stop` / `depot` 系）、`obj=vehicle`、`obj=way`、`obj=good`、
  `obj=bridge`、`obj=tunnel`、`obj=roadsign`、`obj=crossing`、`obj=way-object`、`obj=ground_obj`、`obj=tree`、
  `obj=citycar`、`obj=pedestrian`、`obj=factory`
- `couplings`: `obj=vehicle` の `constraint[prev]` / `constraint[next]`

### 既知の制限（意図的に非対応）

- building/vehicle/way/good/bridge/tunnel/roadsign/crossing/way-object/ground_obj/tree/citycar/pedestrian/factory
  以外の obj 種別（例: `obj=sound` など。`sound_writer_t::get_type_name()`は`"sound"`を返し
  トップレベルobj種別として登録されているが、このプロジェクトが事前に合意した対応obj種別計画には
  含まれていない）
- good の `name` 未指定・`catg`/`value`/`speed_bonus`/`weight_per_unit`/`mapcolor` の妥当性検証。
  `good_writer.cc`はこれらを全て`get_int`/`get_int64`/`text_writer_t`経由で無条件フォールバックさせるのみで、
  fatal/warningになる分岐が無いため対象外（詳細は`src/rules/good.rs`冒頭のREJECTEDコメント参照）
- way の `image[new2]`（switch images判定用プローブ）・`imageup[...]`/`imageup2[...]`（坂道画像）・
  `diagonal[...]`（対角画像）の欠落検証。いずれも空文字列のまま「空画像」として書かれるだけで、
  fatal/warning の分岐が無いため対象外（`image[-]`/`image[-][0]` のみが明示的に FATAL ERROR になる特別扱い）
- way の `cursor`/`icon` 未指定検証。building と異なり、makeobj ソース上は空文字列を許容し
  fatal/warning を出さない。ツールバー表示への影響は building の「ビルドメニュー非表示」ほど
  明確な実機観察の根拠がないため見送り
- bridge の `cursor`/`icon` 未指定検証。way と同じ理由（`cursorskin_writer_t::write_obj` 経由で
  空文字列を無条件許容し fatal/warning を出さない）
- bridge の `topspeed`（`get_int`）・`cost`/`maintenance`（`get_int64`）の妥当性検証。いずれも
  無条件フォールバックのみで `get_int_clamped` ではないため対象外（way の topspeed 等と同じ理由）
- bridge の `max_lenght`（歴史的スペルミス）と`max_length`（正しいスペル）の二重キー挙動そのものの
  検証。両方指定時は`max_length`が後勝ちで使われる意図的な後方互換設計であり、`dbg->warning`/
  `dbg->fatal`の分岐ではないため対象外（詳細は`src/rules/bridge.rs`冒頭のREJECTEDコメント参照）
- bridge の back画像（`backimage[...]`等）未指定検証。`write_bridge_images`の
  `value.size() <= 2`警告分岐はfront画像にのみ存在し、back画像には対応する警告が無い
- tunnel の `topspeed`/`cost`/`maintenance`/`axle_load`/`intro_year`/`intro_month`/
  `retire_year`/`retire_month` の妥当性検証。`tunnel_writer.cc`はこれら7フィールドを全て
  `get_int`/`get_int64`の無条件フォールバックのみで読み、`get_int_clamped`は一度も
  呼ばれていないため対象外（bridgeの`ClampedRangeRule`に相当する根拠が無い。詳細は
  `src/rules/tunnel.rs`冒頭のREJECTEDコメント参照）
- tunnel の画像未指定警告。bridgeの`front{name}[...]`が`value.size() <= 2`で警告を出す
  分岐に相当するコードが`tunnel_writer.cc`には存在しない（空文字列のキーもそのまま
  `frontkeys`/`backkeys`にappendされ、`imagelist_writer_t::write_obj`のcount不一致警告も
  count==keys.get_count()が常に成立するため発火しない）
- tunnel の `cursor`/`icon` 未指定検証。way/bridgeと同じ理由（`cursorskin_writer_t`経由で
  空文字列を無条件許容し fatal/warning を出さない）
- tunnel の `way=`（地下ウェイオブジェクトへの参照）の実在性検証。`xref_writer_t::write_obj`
  は参照を検証せずゲーム読み込み時まで解決を遅延する（goodのfreight参照と同じ理由）
- roadsign の `min_speed`/`offset_left`/`cost`/`maintenance`/`intro_year`/`intro_month`/
  `retire_year`/`retire_month` の妥当性検証。`roadsign_writer.cc`はこれら8フィールドを全て
  `get_int`/`get_int64`の無条件フォールバックのみで読み、`get_int_clamped`は一度も
  呼ばれていないため対象外（詳細は`src/rules/roadsign.rs`冒頭のREJECTEDコメント参照）
- roadsign の `is_signal`/`free_route`/`is_presignal`/`is_prioritysignal`/`is_longblocksignal`/
  `single_way`/`is_private`/`no_foreground`/`end_of_choose`（フラグ系）の相互排他性検証。
  `roadsign_writer.cc`のif-elseチェーンは優先順位に従って1つだけを採用し、以降の分岐は
  単に無視されるだけでfatal/warningを出さない
- roadsign の numbered構文（`image[N]`）と2D構文（`image[方向][state]`）の同時使用検証。
  `image[0]`が非空なら2D構文のキーは単に無視されて読まれないだけで、fatal/warningの分岐が無い
- roadsign の `cursor`/`icon` 未指定検証。way/bridge/tunnelと同じ理由（`cursorskin_writer_t`経由で
  空文字列を無条件許容し fatal/warning を出さない。roadsignは`*c || *i`の条件分岐を経由するが、
  呼ばれる/呼ばれないいずれのケースもfatal/warningにならない点は同じ）
- crossing の `sound`・`animation_time_open`/`animation_time_closed`・`intro_year`/
  `intro_month`/`retire_year`/`retire_month` の妥当性検証。`crossing_writer.cc`はこれらを
  全て`atoi`/`get_int`の無条件フォールバックのみで読み、`get_int_clamped`は一度も
  呼ばれていないため対象外（詳細は`src/rules/crossing.rs`冒頭のREJECTEDコメント参照）
- crossing の `front_openimage[ns/ew]`・`closedimage[ns/ew]`・`front_closedimage[ns/ew]`の
  未指定警告。`openimage[ns/ew]`の`// these must exists!`と対比され、これら3種は
  ソースコード上「optional」と明示されており対応するfatal/warning分岐が存在しない
- crossing の `cursor`/`icon` 未指定検証。`crossing_writer.cc`全文に`cursor`/`icon`への
  言及が一つも無く、`cursorskin_writer_t`も呼ばれない（他obj種別と異なり、そもそも
  対象フィールドが存在しない）
- crossing の `waytype[0]`/`waytype[1]`が既知だが組み合わせとして不自然な値
  （例: `waytype[0]=power`と`waytype[1]=decoration`）の妥当性検証。解決後の列挙値が
  一致するかどうかしか`crossing_writer.cc`は見ておらず、「意味のある交差の組み合わせ」を
  判定するロジックはmakeobj側に存在しない
- way-object の `cost`/`maintenance`/`topspeed`/`intro_year`/`intro_month`/
  `retire_year`/`retire_month` の妥当性検証。`way_obj_writer.cc`はこれら7フィールドを
  全て`get_int`/`get_int64`の無条件フォールバックのみで読み、`get_int_clamped`は
  一度も呼ばれていないため対象外（詳細は`src/rules/way_obj.rs`冒頭のREJECTEDコメント参照）
- way-object の `waytype`と`own_waytype`が解決後の値として同一（または特定の組み合わせ）
  であることの妥当性検証。crossingの`waytype[0]`/`waytype[1]`一致検出に相当する
  fatal分岐が`way_obj_writer.cc`には存在しない（それぞれ独立に`get_waytype()`を
  呼ぶだけで結果を比較しない）。実際、pak128の実例でも両者は意図的に異なる値を取る
- way-object の画像未指定（空文字列/`"-"`）警告。bridgeの`front{name}[...]`が
  `value.size() <= 2`で警告を出す分岐に相当するコードが`way_obj_writer.cc`には
  存在しない
- way-object の `image[-]`相当の「最低1枚必須」チェック。wayの`BaseImageRequiredRule`が
  依拠する明示的なFATAL分岐（`image with label %s missing`）に相当するコードが
  `way_obj_writer.cc`には存在せず、`frontimage[-]`/`backimage[-]`も他のribiと
  全く同列に扱われる
- way-object の `cursor`/`icon` 未指定検証。way/bridge/tunnel/roadsignと同じ理由
  （`cursorskin_writer_t`経由で空文字列を無条件許容し fatal/warning を出さない）
- way-object の `own_waytype` が既知だが意味的に不自然な値（例: `own_waytype=air`）の
  妥当性検証。`way_obj_desc.h`のコメント「only overheadlines_wt is currently used」は
  現状の利用実態を示すコメントであり、makeobj側にそれ以外の値を拒否する分岐は無い
- ground_obj の `climates` 未指定の警告。ソース上に該当する`dbg->warning`呼び出しは
  存在するが、`tabfileobj_t::get()`が欠落キーにも空文字列（非NULL）を返すため
  実行時にこの分岐へ到達しない（詳細は`src/rules/groundobj.rs`冒頭のREJECTEDコメント参照）
- ground_obj の `seasons`/`distributionweight`/`cost`/`speed`/`trees_on_top` の妥当性検証。
  `groundobj_writer.cc`はこれら5フィールドを全て`get_int`/`get_int64`の無条件フォールバック
  のみで読み、`get_int_clamped`は一度も呼ばれていないため対象外
- ground_obj の `waytype` が既知だが意味的に不自然な値（例: 固定物に`waytype=air`）の
  妥当性検証。groundobj_desc.hのコメントは用途の説明に過ぎず、makeobj側にspeedと
  waytypeの組み合わせを拒否する分岐は無い
- ground_obj の `image[<phase>][<season>]` のphase数（何phaseまで許容されるか）の
  上限検証。`for (unsigned int phase = 0; 1; phase++)`は無限ループであり、makeobj側に
  phase数の妥当性チェックは存在しない
- ground_obj の `cursor`/`icon` 未指定検証。`groundobj_writer.cc`全文に`cursor`/`icon`への
  言及が一つも無く、`cursorskin_writer_t`も呼ばれない（crossingと同様、そもそも対象
  フィールドが存在しない）
- tree の `climates` 未指定の警告。ソース上に該当する`printf("WARNING: old syntax
  without climates!\n")`呼び出しは存在するが、`tabfileobj_t::get()`が欠落キーにも
  空文字列（非NULL）を返すため実行時にこの分岐へ到達しない（詳細は
  `src/rules/tree.rs`冒頭のREJECTEDコメント参照）
- tree の `seasons`/`distributionweight` の妥当性検証。`tree_writer.cc`はこれら
  2フィールドを全て`get_int`の無条件フォールバックのみで読み、`get_int_clamped`は
  一度も呼ばれていないため対象外
- tree の `cursor`/`icon`/`waytype` 未指定検証。`tree_writer.cc`全文にこれらの
  フィールドへの言及が一つも無く、`cursorskin_writer_t`も`get_waytype()`も
  呼ばれない（crossing/ground_objのcursor/iconパターン、goodのwaytypeパターンと
  同様、そもそも対象フィールドが存在しない）
- tree のage段階数（5固定）を`.dat`側でカスタマイズできるかの検証。
  `for (unsigned int age = 0; age < 5; age++)`はソースコード上のハードコードされた
  定数であり、`.dat`側にage数を指定するキー自体が存在しない
- citycar の `distributionweight`/`intro_year`/`intro_month`/`retire_year`/`retire_month`/
  `speed` の妥当性検証。`citycar_writer.cc`はこれら6フィールドを全て`get_int`の
  無条件フォールバックのみで読み、`get_int_clamped`は一度も呼ばれていないため対象外
  （詳細は`src/rules/citycar.rs`冒頭のREJECTEDコメント参照）
- citycar の8方向`image[<dir>]`の一部欠落検証（vehicleの`incomplete-8-direction-images`
  相当）。citycarの画像走査は無条件ループ（早期終了なし）で、vehicleのような
  「一部方向だけ定義」を検出するfatal分岐が存在しないため対象外
- citycar の`image[<dir>]`が1つも定義されていない場合の警告。
  `imagelist_writer_t::write_obj`は空リストでもfatal/warningを出さないため対象外
- citycar の `cursor`/`icon`/`waytype`/`engine_type`/`freight`/`constraint[prev]`/
  `constraint[next]` 未指定検証。`citycar_writer.cc`全文にこれらのフィールドへの
  言及が一つも無く、そもそも対象フィールドが存在しない（goodのwaytypeパターン、
  crossing/ground_obj/treeのcursor/iconパターンと同様）
- pedestrian の `distributionweight`/`offset`/`intro_year`/`intro_month`/
  `retire_year`/`retire_month` の妥当性検証。`pedestrian_writer.cc`はこれら
  6フィールドを全て`get_int`の無条件フォールバックのみで読み、`get_int_clamped`は
  一度も呼ばれていないため対象外（詳細は`src/rules/pedestrian.rs`冒頭のREJECTED
  コメント参照）
- pedestrian の `steps_per_frame`が0または負の値の場合の警告。
  `max(obj.get_int("steps_per_frame", 1), 1)`はC++標準の`max()`によるインラインの
  下限クランプであり、`get_int_clamped`が内部で呼ぶ`dbg->warning`のような
  メッセージ出力を一切伴わない。このプロジェクトが`ClampedRangeRule`として
  一貫して採用してきた「`get_int_clamped`呼び出しである」という基準を満たさないため
  見送った
- pedestrian の静止8方向`image[<dir>]`の一部欠落検証（vehicleの
  `incomplete-8-direction-images`相当）。citycarと同じ理由で、静止分岐の
  画像走査は無条件ループ（早期終了なし）で、一部方向だけ定義されている状態を
  検出するfatal分岐が存在しない
- pedestrian のアニメーション分岐で、ある方向だけ`image[<dir>][0]`が空
  （＝その方向のみ0フレーム）という方向間の不整合検証。
  `pedestrian_writer.cc`のフレーム走査は方向ごとに独立しており、他方向との
  フレーム数比較やfatal/warningの分岐が存在しない
- pedestrian の`image[<dir>]`（静止分岐）または`image[<dir>][<frame>]`
  （アニメーション分岐）が1つも定義されていない場合の警告。
  `imagelist_writer_t::write_obj`/`imagelist2d_writer_t::write_obj`は空リストでも
  fatal/warningを出さないため対象外
- pedestrian の `cursor`/`icon`/`waytype`/`engine_type`/`freight`/
  `constraint[prev]`/`constraint[next]` 未指定検証。`pedestrian_writer.cc`全文に
  これらのフィールドへの言及が一つも無く、そもそも対象フィールドが存在しない
  （citycarと同様のパターン）
- factory の `location` が既知の6値（`land`/`water`/`city`/`river`/`shore`/`forest`）
  のいずれにも一致しない場合の警告。`dbg->warning`/`dbg->fatal`を伴わずに黙って
  `Land`へフォールバックするだけで、このフォールバック自体を示すメッセージ出力が
  一切無い（詳細は`src/rules/factory.rs`冒頭のREJECTEDコメント参照）
- factory の `productivity`/`range`/`distributionweight`/`pax_level`/
  `expand_minimum`/`expand_range`/`expand_times`/`electricity_boost`/
  `passenger_boost`/`mail_boost`/`electricity_amount`/`electricity_demand`/
  `passenger_demand`/`mail_demand`/`sound_interval`の妥当性検証。いずれも
  `get_int`の無条件フォールバックのみで読まれ、`get_int_clamped`は一切
  使われていない
- factory の `inputgood[N]`/`outputgood[N]`/`smoke=`/`fields`/`fields[N]`が
  参照する`good`/`field`オブジェクトの実在性検証。`xref_writer_t::write_obj`は
  参照を検証せずゲーム読み込み時まで解決を遅延する（vehicleの`freight=`・
  tunnelの`way=`と同じ理由）。参照先はパークセット全体のどこにあってもよく、
  ディレクトリ横断のレジストリが本ツールに無いため検証できない
- factory の `inputsupplier[N]`/`inputcapacity[N]`/`inputfactor[N]`の妥当性検証。
  `outputcapacity`と異なり、対応する`dbg->error`分岐が存在しない
- factory の`has_snow[N]`/`production_per_field[N]`/`storage_capacity[N]`/
  `spawn_weight[N]`・`min_fields`/`max_fields`/`start_fields`の相互妥当性検証。
  いずれも無条件フォールバックのみで読まれ、相互比較のfatal/warning分岐も無い
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
    good.rs                      obj=good のRule実装（現状ルール0件、根拠不在の記録が主目的）
    bridge.rs                     obj=bridge のRule実装
    tunnel.rs                      obj=tunnel のRule実装
    roadsign.rs                     obj=roadsign のRule実装
    crossing.rs                      obj=crossing のRule実装
    way_obj.rs                        obj=way-object のRule実装
    groundobj.rs                       obj=ground_obj のRule実装
    tree.rs                             obj=tree のRule実装
    citycar.rs                           obj=citycar のRule実装
    pedestrian.rs                         obj=pedestrian のRule実装
    factory.rs                             obj=factory のRule実装
    common.rs                          共有定数・ヘルパー（KNOWN_WAYTYPES等）・duplicate-key検出
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
