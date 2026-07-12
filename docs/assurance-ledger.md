# 保証台帳（Assurance Ledger）

「これが壊れたら困る」挙動を一覧化し、実際にどう守られているか（Control）・
テストで担保されているか（Test/Quality）を記録する台帳。リリース前・大きな変更前に
この一覧を見直し、Statusが🔴/🟡の項目を優先的に潰す運用とする。

凡例:
- **Control**: None（無防備）/ Detective（事後検知のみ）/ Preventive（事前に防ぐ）
- **Quality**: テストがある場合、期待する結果を直接assertしているか（Strong）、
  間接的な副作用だけか（Weak）
- **Status**: 🔴 Missing Control/Test（無防備または未検証） / 🟡 Structural Weakness
  （守りはあるが構造的な弱点が残る） / 🟢 OK（実効的に守られている）

最終更新: 2026-07-13（Phase 11全体コードレビュー＋Phase 12修正反映）

## dat_linter（Rust本体）

| 挙動 | Control | Test | Quality | Status |
|---|---|---|---|---|
| Shift-JIS(CP932)エンコーディングのフォールバック | Preventive | `shift_jis_encoded_file_is_decoded_as_fallback` | Strong | 🟢 |
| duplicate-keyのfirst-write-wins（採用値まで） | Preventive | `duplicate_key_keeps_first_value` | Strong | 🟢 |
| `--format text`（デフォルト出力）の後方互換性 | Preventive | `lint_text_format_duplicate_key_matches_exact_golden_output`/`lint_text_format_missing_waytype_matches_exact_golden_output`（stdout/stderr全文をassert_eq!で厳密比較） | Strong | 🟢 |
| タイルサイズ5段階優先順位の全組み合わせ | Preventive | ペアワイズのみ、3者競合・overrides vs cell_size直接対決は無し | 部分的 | 🟡 |
| `keys --format json`の各obj種別キー内容（`"obj"`含む） | Preventive | `keys_format_json_emits_valid_json_with_expected_shape`にbuilding種別の`keys`内容検証（obj/name/copyright/waytype）と全obj種別のkeys非空チェックを追加 | Strong | 🟢 |
| テストのcwd分離ガイドライン遵守 | Preventive | 残り11件全てを`run_in_clean_dir`（またはCRLFフィクスチャ専用の一時ディレクトリ）経由に統一 | Strong | 🟢 |
| `keys --format json`の`known_values.waytype`/`direction`後方互換性維持 | Preventive | JSONスキーマを`BTreeMap`から明示的構造体へ変更しつつ既存2フィールドの形は維持、`per_obj_type`は同階層への追加のみ | Strong | 🟢 |
| BOM付き`.dat`ファイルで`obj`キー照合が誤って全滅しない | Preventive | `parser.rs`のBOM除去回帰テスト（`bom_utf8.dat`） | Strong | 🟢 |
| `=`の無い不正な行の検知（`MalformedLine`診断） | Detective | `parser.rs`/`malformed_line.dat`・末尾/冒頭の不正行を取りこぼさないエッジケーステスト | Strong | 🟢 |
| `<...>`算術評価の演算子優先順位（`% > / > * > - > +`）再現 | Preventive | `param_expansion.rs`の優先順位依存ケーステスト（`$0+$1*2`） | Strong | 🟢 |
| roadsignのis_signal系フラグ判定（C++の素の真偽判定と一致） | Preventive | `roadsign_signal_negative_one.dat` | Strong | 🟢 |
| couplings制約スキャンの空値終端一致（`str.empty()`相当） | Preventive | `couplings_empty_constraint/EmptyNextConstraint.dat` | Strong | 🟢 |
| building `extension_building`の`>0`判定一致 | Preventive | `extension_building=0/positive`各fixture | Strong | 🟢 |
| `Dims=`のsint16切り詰め再現（`DimsRule`） | Preventive | `building_dims_sint16_truncation.dat` | Strong | 🟢 |
| `obj=symbol`/`obj=misc`の`name=`既知値検証（`UnknownSkinNameRule`） | Preventive | `symbol`/`misc_lint.rs`のunknown-name/fakultative-name各テスト | Strong | 🟢 |
| `AllImagesRule`の`"-"`判定一貫性（`image[0]=-`と`> -`で同一挙動） | Preventive | cursor/symbol/menu/miscのdash-sentinel回帰テスト | Strong | 🟢 |
| vehicle gear/powerのuint16/uint32切り詰め・パース失敗の誤未指定化防止 | Preventive | `vehicle_gear_parse_failure.dat`等 | Strong | 🟢 |
| factory mapcolorのstrtoul相当パース（16進/8進対応） | Preventive | `factory_mapcolor_hex.dat` | Strong | 🟢 |
| factory `probability_to_spawn`のfields存在ガード | Preventive | `factory_probability_to_spawn_no_fields.dat` | Strong | 🟢 |
| crossing/groundobj speedのuint16切り詰め再現 | Preventive | `crossing_speed_truncates_to_zero.dat`等 | Strong | 🟢 |
| tree seasonsのuint8切り詰め再現 | Preventive | `tree_seasons_zero.dat`/`tree_seasons_overflow.dat` | Strong | 🟢 |
| pedestrian steps_per_frameのnarrow-int-overflow検知 | Preventive | `pedestrian_steps_per_frame_overflow.dat` | Strong | 🟢 |
| building levelの引き算アンダーフローpanic防止 | Preventive | `building_level_extreme_negative_overflow.dat`（`level=i64::MIN`でpanicしないことを確認） | Strong | 🟢 |
| strtol/strtoul相当パースの共通化・オーバーフロー飽和挙動 | Preventive | `common.rs`単体テスト12件（hex/octal/符号/飽和/no-digits） | Strong | 🟢 |
| ディレクトリsymlink循環時に無限再帰・スタックオーバーフローしない | Preventive | 自己参照/祖先ディレクトリsymlinkのfs_walk単体テスト（Windows/Unix両対応、作成失敗時はスキップ） | Strong | 🟢 |
| 読み取り不可なサブディレクトリを「サイレント成功」扱いにしない | Detective | Unix版（`chmod 000`）のみ実装済み。Windows側は`icacls`再現がCI環境ごとに不安定なため見送り | Weak | 🟡 |
| `fmt -w`がファイルsymlink経由でリンク先を書き換えない | Preventive | `fmt_write_refuses_to_write_through_file_symlink`（`tests/cli_integration.rs`） | Strong | 🟢 |

## VSCode拡張（editors/vscode）

| 挙動 | Control | Test | Quality | Status |
|---|---|---|---|---|
| フォーマッタ: バッファ内容を整形（ディスクの古い内容ではない） | Preventive（一時ファイル経由） | `Format Document reflects unsaved buffer edits...`（修正前で実際に失敗することまで確認済み） | Strong | 🟢 |
| lint: バッファ内容を参照 | Preventive（一時ファイル経由。`formatter.ts`と同じ`withTempDatFile`ヘルパーを共有し、`extension.ts`の`lintDocument`もディスク上ではなくバッファ内容を書いた一時ファイルへ実行するよう統一。画像パス解決の基準（`dat_dir`）を変えないよう、一時ファイルは`os.tmpdir()`ではなく元ファイルと同じディレクトリに作成） | 既存の`duplicate_key.dat`/`broken_missing_waytype.dat`lintテスト2件がこの変更後も継続してpass | Strong | 🟢 |
| cwd戦略（workspace folder root自動探索）の自動テスト経路 | コードは存在 | `test/workspace/workspace-cwd.test.ts`を新設。`.vscode-test.mjs`に`workspaceFolder`付きの専用テスト設定（label: `workspace-cwd`）を追加し、`fixtures/workspace-root/`（配下に`dat_linter.toml`と`nested/sample.dat`）を実際に開いた状態で`resolveExecutionContext`のworkspace folder分岐を実行・検証。CLIで事前に「cwd=workspace-root時はwarning_count:0、cwd=nested時はwarning_count:1」の差分を確認済みの上でduplicate-key診断の有無をassert | Strong | 🟢 |
| バージョン非互換検知ヒューリスティック（`describeFailure`等） | Detective | `test/runner.test.ts`を新設（vscode拡張ホスト起動は必要だが、実行ファイル起動・アクティベーション不要な純粋関数テスト）。`describeFailure`（ENOENT/"not recognized"/versionHint一致/フォールバック各分岐）と、`LINT_FORMAT_JSON_VERSION_HINT`（`extension.ts`からexport）・`FMT_VERSION_HINT`（`formatter.ts`からexport）の正規表現をclap風エラー文言でassert | Strong | 🟢 |
| 一時ファイルのエラー時クリーンアップ（`formatter.ts`） | Preventive（try/finally。`withTempDatFile`ヘルパーに集約） | `test/extension.test.ts`に`Format Document cleans up its temp file even when dat_linter fails to run`を追加。`executablePath`を存在しないパスへ一時的に上書きしてフォーマット失敗を発生させ、`os.tmpdir()`配下に`dat-linter-fmt-*`が残っていないことをbefore/after差分でassert | Strong | 🟢 |
| スニペットのlint陳腐化検証 | Detective、CI組込済み | `lint-snippets.mjs`、実バイナリへの直接assert | Strong | 🟢 |
| グラマー生成のCI drift検知 | Detective、CI組込済み | `git diff --exit-code`、実バイナリ出力とコミット済みファイルを直接比較 | Strong | 🟢 |
| `package.json` contributes（language/grammar/snippets）の実機登録検証 | Detective | `test/extension.test.ts`に2件追加: `vscode.languages.getLanguages()`に`"simutrans-dat"`が含まれること、`.dat`ファイルを開いた際に`document.languageId`が`"simutrans-dat"`になることをassert（grammar/snippetsは既存の`test:grammar`/`test:snippets`が別途カバー） | Strong | 🟢 |
| 空/空白のみのバッファでlintを実行しない（誤った"obj=未対応"表示防止） | Preventive | `test/extension.test.ts`の空ファイル/全消去テスト2件 | Strong | 🟢 |
| `known_values.per_obj_type`（type/location/climates/skin名）のシンタックスハイライト反映 | Detective、CI drift検知組込済み | `test/grammar`の3つのfixture・`generate:grammar`再実行でdrift無し確認 | Strong | 🟢 |
| 入力補完のobj種別判定（`-`区切りレコード境界を跨がない） | Preventive | `test/completion.test.ts`の`findObjTypeAtLine`複数レコードテスト12件 | Strong | 🟢 |
| Workspace Trust未対応（未信頼ワークスペースでの任意コード実行リスク） | Preventive（`package.json`の`capabilities.untrustedWorkspaces.supported:false`＋`activate()`内多層防御ガード） | `shouldActivateInWorkspace`単体テスト | Strong | 🟢 |
| `KeysDataCache.load()`の非同期呼び出しレース（古い設定変更の結果が新しい結果を上書き） | Preventive（世代カウンタ） | `test/completion.test.ts`のレース再現テスト（遅い方が先に発行され後から解決するケース3パターン） | Strong | 🟢 |
| `lintDocument`の非同期呼び出しレース（古いバッファの診断が新しいバッファの診断を上書き） | Preventive（`LintGenerationTracker`世代カウンタ） | `test/runner.test.ts`は世代管理のpure logicのみ検証。実dat_linterプロセスの完了順序を確定的に操作する統合テストは無し | Weak | 🟡 |

## 対応履歴

- 2026-07-11: 初回監査実施。以降の対応はこの表のStatus変化で追跡する（🔴/🟡 → 🟢 へ変わった項目に完了日・PR番号を追記していく）。
- 2026-07-11: dat_linter（Rust本体）の3件（`--format text`後方互換性・`keys --format json`のキー内容・
  テストのcwd分離ガイドライン遵守）に対応済み（[PR #6](https://github.com/128na/simutrans-dat-linter/pull/6)）。
- 2026-07-11: VSCode拡張側の5件（lintのバッファ参照統一・cwd戦略の自動テスト経路・バージョン非互換検知ヒューリスティックの単体テスト・
  一時ファイルのエラー時クリーンアップテスト・`package.json` contributesの実機登録検証）に対応済み（[PR #7](https://github.com/128na/simutrans-dat-linter/pull/7)）。
- 2026-07-12: 旧拡張からのフィードバック反映（値レジストリ拡充・シンタックスハイライト拡充・入力補完・空バッファlintスキップ）を
  [PR #10](https://github.com/128na/simutrans-dat-linter/pull/10)〜[#13](https://github.com/128na/simutrans-dat-linter/pull/13)で対応。
  台帳への反映が漏れていたのをPhase 11監査で発見・本更新で追記。
- 2026-07-13: Phase 11（`/review`による全体コードレビュー）で見つかった実バグ・セキュリティ課題を
  [PR #14](https://github.com/128na/simutrans-dat-linter/pull/14)〜[#19](https://github.com/128na/simutrans-dat-linter/pull/19)で対応
  （BOM誤検知・演算子優先順位・roadsign/couplingsの判定不一致・building/Dims/skin名検証・
  vehicle/factory/crossing/groundobj/tree/pedestrianの整数切り詰め・fs_walkのsymlink対策・
  VSCode拡張のWorkspace Trust未対応とrace condition）。
  残存する🟡2件（読み取り不可ディレクトリのWindows版テスト・lintDocumentレースの統合テスト）は
  実装は完了しているがテスト手段の制約により見送り、フォローアップ課題として残す。
