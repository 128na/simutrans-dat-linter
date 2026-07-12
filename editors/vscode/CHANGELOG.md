# Changelog

All notable changes to the "simutrans-dat-linter" VSCode extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Syntax highlighting for `.dat` files: registers a new `simutrans-dat` language
  (`language-configuration.json` + `syntaxes/simutrans-dat.tmLanguage.json`, scope
  `source.simutrans-dat`). The grammar's key list and waytype/direction value lists are
  generated mechanically from `dat_linter keys --format json` by
  `scripts/generate-grammar.mjs` (`npm run generate:grammar`), so they can't drift from what
  dat_linter itself considers valid. CI regenerates the grammar and fails on any diff against
  the committed file, so a stale grammar can't be merged.
- Extended `scripts/generate-grammar.mjs` to also highlight the value families exposed by
  `known_values.per_obj_type` (added in a later dat_linter release): building/factory `type`
  values (`storage.type.building-types.simutrans-dat`), factory `location` values
  (`storage.type.locations.simutrans-dat`), `climates` values
  (`storage.type.climates.simutrans-dat`), and the skin `name` values used by the built-in
  `menu`/`cursor`/`symbol`/`misc`/`ground` obj types (`storage.type.skin-names.simutrans-dat`).
  Same design constraint as waytype/direction: the grammar can't tell which obj type a given
  line belongs to, so each category is flattened across every obj type that reports it rather
  than validated per obj type; `dat_linter lint` remains the source of truth for actual
  per-obj_type correctness. A few value strings collide across categories (e.g. `water` appears
  in `location`/`climates`/skin-names and the existing waytype list; `post`/`busstop`/`carstop`/
  `monorailstop` appear as both building `type` values and cursor/symbol skin names) -- these are
  resolved by pattern order in `defined_values` (see the comment above it in
  `generate-grammar.mjs`), not by any attempt at disambiguation.
- Snippets for `.dat` files (`snippets/snippets.json`, 50 snippets covering every obj type),
  ported from the author's earlier CC0-licensed `128na/simutrans-vscode-extention`. Verified
  against `dat_linter lint` via the new `scripts/lint-snippets.mjs` (`npm run test:snippets`),
  which resolves each snippet's tab stops to placeholder text and fails on any `error`-severity
  diagnostic other than `missing-image-file` (snippets intentionally reference example filenames
  that don't exist on disk). Fixed several snippets that failed this check when ported over:
  missing/obsolete `waytype`, an obsolete `extension_building` key, missing `cursor`/`icon` on
  the HQ building snippet, an unspecified factory `mapcolor`, and a crossing snippet whose two
  `waytype[N]` defaults resolved to the same value.
- `vscode-tmgrammar-test` devDependency plus grammar snapshot fixtures under `fixtures/*.dat`
  (`npm run test:grammar`, backed by `vscode-tmgrammar-snap`), committed as `.snap` files and
  run in CI as a separate step from the existing `npm test` (mocha/`@vscode/test-cli`) suite.
- README.md: new "旧拡張(`128na/simutrans-vscode-extention`)からの移行" section explaining that
  running both extensions at once can make `.dat` highlighting unstable (both contribute a
  language/grammar for the same extension), and recommending uninstalling the old one; lint/
  format remain unaffected either way since they're language-ID-independent.

### Changed

- Documentation updates only (no behavior change in this extension): `dat_linter` itself no
  longer auto-generates `dat_linter.toml` when `configPath` is unset and no config file is found
  in cwd -- it now falls back to all-rules-enabled defaults instead, and file generation is
  opt-in via the new `dat_linter init` subcommand. Updated `README.md`, `package.json`'s
  `simutransDatLinter.configPath` description, and the `src/runner.ts` doc comment to reflect
  this. The extension's own cwd/config resolution logic (`resolveExecutionContext` in
  `src/runner.ts`) is unchanged.

### Added

- Document Formatting support backed by `dat_linter fmt <path> [--config ...]`: register
  `vscode.languages.registerDocumentFormattingEditProvider` for `**/*.dat` files, so
  `editor.formatOnSave` and the `Format Document` command now normalize/reorder `.dat` files.
  The provider runs `dat_linter fmt` without `-w`/`--write` and applies its stdout as a
  `TextEdit` to VSCode's own buffer, rather than letting the CLI write the file directly, to
  avoid a race between the editor and an external process writing the same file. Line endings
  (CRLF/LF) are preserved because `dat_linter fmt` detects and preserves them itself.
- Refactored the shared "resolve cwd / `executablePath` / `configPath`, then run `dat_linter`
  and classify a failure as executable-not-found vs. version-incompatible" logic out of
  `src/extension.ts` and into a new `src/runner.ts`, so both the lint path (`src/extension.ts`)
  and the new formatter (`src/formatter.ts`) share it instead of duplicating it.
- Initial implementation of the `simutrans-dat-linter` VSCode extension, built on top of
  `try-out/vscode-dat-linter-poc` (in the sibling `simutrans_addon` repository) but rewritten
  to consume `dat_linter lint --format json` (dat_linter >= 0.1.2) instead of parsing text output.
- `.dat` files are linted on open and on save; results are surfaced via VSCode's Problems panel.
- Settings: `simutransDatLinter.executablePath` (default `"dat_linter"`) and
  `simutransDatLinter.configPath` (default unset, passed through to `dat_linter --config`).
- Runtime schema validation (`isDatLinterJsonOutput`) for the `--format json` payload, so an
  unexpected/incompatible `dat_linter` output fails with a descriptive error instead of crashing.
- Heuristic error messages distinguishing "executable not found" from "dat_linter version too old
  to support --format json".
