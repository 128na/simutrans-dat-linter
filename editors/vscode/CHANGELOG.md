# Changelog

All notable changes to the "simutrans-dat-linter" VSCode extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

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
