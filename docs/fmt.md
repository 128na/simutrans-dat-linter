# `fmt` — フォーマッタ

```
dat_linter fmt <file.dat>                    # 慣習的な順序に並び替え（標準出力へ。デフォルト挙動）
dat_linter fmt --no-reorder <file.dat>       # 並び替えず元の順序のまま正規化
dat_linter fmt --write <file.dat>            # ファイルに上書き（-w も可）
dat_linter fmt <dir> --write                 # ディレクトリ内の.datを再帰的に整形して書き戻す
dat_linter fmt "path/to/*.dat" --write       # globパターンも指定できる
```

安全な正規化（キー小文字化・`=`前後の空白除去・値の前後トリム・既知のenum的な値の
大文字小文字統一）のみ行い、値の内容変更のような壊しうる操作は行いません。

### 既知のenum的な値の大文字小文字統一

`waytype`（`waytype[0]`/`waytype[1]`含む）・`climates`・`type`（building）・
`engine_type`（vehicle）・`placing`（factory）の値は、makeobj自身が`STRICMP`
（大文字小文字を区別しない比較）で全小文字の既知リテラルと照合するため、
値全体を一括で小文字化してもmakeobjの動作に一切影響しません。これは
キー小文字化と同じ理屈で無条件（`--no-reorder`でも）に適用される安全な正規化です。
例: `waytype=Track` → `waytype=track`、`climates=Temperate,TUNDRA` → `climates=temperate,tundra`。

`constraint[prev][N]`/`constraint[next][N]`（vehicle）は特殊ケースです。
makeobjがSTRICMPで比較するのは`"none"`（連結制約なしを表す特別な値）のみで、
それ以外の値は他のvehicleの`name=`を指す自由記述・大文字小文字を区別する参照です。
そのため、値が`"none"`と大文字小文字を無視して一致する場合のみ`none`へ
正規化し、それ以外の値（vehicle名の参照）は一切変更しません。

## 並び替え

慣習的な順序への並び替えが**デフォルト挙動**です（並び替え自体はスタイル上の慣習であり
makeobj の動作には影響しません）。無効化したい場合:

- `--no-reorder` — そのプロセスの実行に限り無効化
- `dat_linter.toml` の `[rules] exclude = ["fmt-reorder-applied"]` — 恒久的に無効化
  （優先順位: `--no-reorder` > config設定）

未対応の obj 種別では並び替えを行わず元の順序のまま出力します。

## 複数ファイル

複数ファイルに解決された場合、`--write`（`-w`）を指定しないとエラー終了します。

## コメント行の扱い

`#` 始まりのコメント行は、直後に現れる最初の `key=value` 行に紐づき、そのPairと一緒に
並び替え後も移動します。紐づけ先が無いと判断される場合（不正行の直前・複数obj連結の
区切り行をまたぐ場合・セグメント末尾の場合）は、他の行と同様に出力から削除され、
削除した件数が警告として表示されます。
