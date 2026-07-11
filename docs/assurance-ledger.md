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

最終更新: 2026-07-11（初回作成、Assurance Auditスキルによる監査結果を反映）

## dat_linter（Rust本体）

| 挙動 | Control | Test | Quality | Status |
|---|---|---|---|---|
| Shift-JIS(CP932)エンコーディングのフォールバック | Preventive | `shift_jis_encoded_file_is_decoded_as_fallback` | Strong | 🟢 |
| duplicate-keyのfirst-write-wins（採用値まで） | Preventive | `duplicate_key_keeps_first_value` | Strong | 🟢 |
| `--format text`（デフォルト出力）の後方互換性 | Preventive | `lint_text_format_duplicate_key_matches_exact_golden_output`/`lint_text_format_missing_waytype_matches_exact_golden_output`（stdout/stderr全文をassert_eq!で厳密比較） | Strong | 🟢 |
| タイルサイズ5段階優先順位の全組み合わせ | Preventive | ペアワイズのみ、3者競合・overrides vs cell_size直接対決は無し | 部分的 | 🟡 |
| `keys --format json`の各obj種別キー内容（`"obj"`含む） | Preventive | `keys_format_json_emits_valid_json_with_expected_shape`にbuilding種別の`keys`内容検証（obj/name/copyright/waytype）と全obj種別のkeys非空チェックを追加 | Strong | 🟢 |
| テストのcwd分離ガイドライン遵守 | Preventive | 残り11件全てを`run_in_clean_dir`（またはCRLFフィクスチャ専用の一時ディレクトリ）経由に統一 | Strong | 🟢 |

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

## 対応履歴

- 2026-07-11: 初回監査実施。以降の対応はこの表のStatus変化で追跡する（🔴/🟡 → 🟢 へ変わった項目に完了日・PR番号を追記していく）。
- 2026-07-11: dat_linter（Rust本体）の3件（`--format text`後方互換性・`keys --format json`のキー内容・
  テストのcwd分離ガイドライン遵守）に対応済み（[PR #6](https://github.com/128na/simutrans-dat-linter/pull/6)）。
- 2026-07-11: VSCode拡張側の5件（lintのバッファ参照統一・cwd戦略の自動テスト経路・バージョン非互換検知ヒューリスティックの単体テスト・
  一時ファイルのエラー時クリーンアップテスト・`package.json` contributesの実機登録検証）に対応済み（`fix/vscode-assurance-audit-hardening`ブランチ、PR番号は追ってMain側が追記）。
