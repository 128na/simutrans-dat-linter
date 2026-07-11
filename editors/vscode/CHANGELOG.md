# Changelog

All notable changes to the "simutrans-dat-linter" VSCode extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
