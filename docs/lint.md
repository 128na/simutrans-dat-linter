# `lint` — 静的検証

```
dat_linter lint <path/to/file.dat>
dat_linter lint -v  <path/to/file.dat>   # info まで表示
dat_linter lint -vv <path/to/file.dat>   # debug（生の値・解決後パス）まで表示
```

## 複数ファイル一括処理

ファイルパスの他に、ディレクトリや glob パターンも指定できます。

```
dat_linter lint path/to/dir             # ディレクトリ内の.datを再帰的に収集して検証
dat_linter lint "path/to/*.dat"         # globパターン（"**/*.dat"のような再帰パターンも可）
```

PowerShell は `*` をシェル側で自動展開しないため、`dat_linter` 自身が glob を解釈します。
複数ファイル指定時は各ファイルの結果に加え、末尾に集計行が出ます
（`合計: 対象ファイル 4 件 / error 12 件 / warning 3 件`）。
1件でも error/warning があれば終了コードは `1` になります。

## `list` / `describe` サブコマンド

```
dat_linter list                    # 診断codeの一覧
dat_linter describe obsolete-type  # 指定codeの説明（なぜNGか・どう直すか）を表示
```

ルールの有効/無効の切り替えは [config.md](config.md) を参照してください。

## 対応 obj 種別

`lint` は makeobj が認識する全 22 obj 種別をカバーしています:

`building` / `vehicle` / `way` / `good` / `bridge` / `tunnel` / `roadsign` / `crossing` /
`way-object` / `ground_obj` / `tree` / `citycar` / `pedestrian` / `factory` / `sound` /
`ground` / `menu` / `cursor` / `symbol` / `smoke` / `field` / `misc`

各 obj 種別の検出項目・makeobj ソース上の根拠・意図的に対応していない項目とその理由は、
`src/rules/<obj種別>.rs` のドキュメントコメント（`REJECTED` として記録）を参照してください。

大半のルールは makeobj 自体（コンパイル時）の `dbg->fatal`/`dbg->warning` が根拠ですが、
`vehicle` の `power-gear-mismatch` と `factory` の `productivity-zero` の2ルールのみ、
ゲームエンジンのランタイムコードを根拠とする「静的解析」層の例外です。
