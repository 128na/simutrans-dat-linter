#!/usr/bin/env node
// Guards snippets/snippets.json against drifting out of sync with dat_linter's
// validation rules: for every snippet, resolves its tab stops to placeholder
// text, writes it out as a standalone .dat file, and runs `dat_linter lint
// --format json` against it. Fails if any snippet produces an "error"
// severity diagnostic other than missing-image-file (snippets intentionally
// reference example image files like "image.0.0" that don't exist on disk).
//
// Usage: node scripts/lint-snippets.mjs
// Env:   DAT_LINTER_BIN - path to the dat_linter executable (default: "dat_linter")

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const VSCODE_DIR = path.join(SCRIPT_DIR, "..");
const SNIPPETS_PATH = path.join(VSCODE_DIR, "snippets", "snippets.json");
const CONFIG_PATH = path.join(VSCODE_DIR, "fixtures", "test-lint-config.toml");
const DAT_LINTER_BIN = process.env.DAT_LINTER_BIN || "dat_linter";

// Snippets reference example filenames (e.g. "image.0.0") that don't exist on
// disk by design -- that's not a snippet-content bug, so this code is ignored
// even at "error" severity.
const IGNORED_CODES = new Set(["missing-image-file"]);

function fail(message) {
  console.error(`lint-snippets: ${message}`);
  process.exit(1);
}

/**
 * Resolves VSCode snippet tab-stop syntax (`$1`, `${1:default}`,
 * `${1|choice1,choice2|}`, `$0`) into plain placeholder text so the result is
 * syntactically ordinary .dat content.
 */
function resolveSnippetBody(bodyLines) {
  const text = bodyLines.join("\n");
  return text
    .replace(/\$\{(\d+):([^}]*)\}/g, (_match, num, def) => (def.length > 0 ? def : `placeholder${num}`))
    .replace(/\$\{(\d+)\|([^}]*)\|\}/g, (_match, num, choices) => {
      const first = choices.split(",")[0];
      return first && first.length > 0 ? first : `placeholder${num}`;
    })
    .replace(/\$\{(\d+)\}/g, (_match, num) => `placeholder${num}`)
    .replace(/\$(\d+)/g, (_match, num) => (num === "0" ? "" : `placeholder${num}`));
}

function lintSnippet(tmpDir, key, text) {
  const safeName = key.replace(/[^a-zA-Z0-9._-]/g, "_");
  const filePath = path.join(tmpDir, `${safeName}.dat`);
  writeFileSync(filePath, text, "utf8");

  let stdout;
  try {
    stdout = execFileSync(
      DAT_LINTER_BIN,
      ["lint", filePath, "--format", "json", "--config", CONFIG_PATH],
      { encoding: "utf8", cwd: tmpDir }
    );
  } catch (err) {
    // dat_linter exits non-zero when it reports any error-severity diagnostic;
    // stdout still carries the JSON payload in that case.
    if (typeof err.stdout === "string" && err.stdout.length > 0) {
      stdout = err.stdout;
    } else {
      throw new Error(`failed to run dat_linter for snippet "${key}": ${err.message}`);
    }
  }

  let payload;
  try {
    payload = JSON.parse(stdout);
  } catch (err) {
    throw new Error(`dat_linter produced non-JSON output for snippet "${key}": ${err.message}\n${stdout}`);
  }

  const diagnostics = (payload.files ?? []).flatMap((f) => f.diagnostics ?? []);
  return diagnostics;
}

function main() {
  let snippets;
  try {
    snippets = JSON.parse(readFileSync(SNIPPETS_PATH, "utf8"));
  } catch (err) {
    fail(`failed to read/parse ${SNIPPETS_PATH}: ${err.message}`);
  }

  const tmpDir = mkdtempSync(path.join(tmpdir(), "dat-linter-snippets-"));
  const failures = [];

  try {
    for (const [key, snippet] of Object.entries(snippets)) {
      if (!Array.isArray(snippet.body)) {
        failures.push({ key, diagnostics: [{ severity: "error", code: "invalid-snippet", message: '"body" is not an array', line: null }] });
        continue;
      }
      const text = resolveSnippetBody(snippet.body);
      let diagnostics;
      try {
        diagnostics = lintSnippet(tmpDir, key, text);
      } catch (err) {
        failures.push({ key, diagnostics: [{ severity: "error", code: "lint-invocation-failed", message: err.message, line: null }] });
        continue;
      }
      const offending = diagnostics.filter((d) => d.severity === "error" && !IGNORED_CODES.has(d.code));
      if (offending.length > 0) {
        failures.push({ key, diagnostics: offending });
      }
    }
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }

  const total = Object.keys(snippets).length;
  if (failures.length > 0) {
    console.error(`lint-snippets: ${failures.length}/${total} snippet(s) produced dat_linter errors:\n`);
    for (const failure of failures) {
      console.error(`  ${failure.key}:`);
      for (const d of failure.diagnostics) {
        console.error(`    [${d.code}] line ${d.line ?? "?"}: ${d.message}`);
      }
    }
    process.exit(1);
  }

  console.log(`lint-snippets: all ${total} snippets passed dat_linter lint (ignoring: ${[...IGNORED_CODES].join(", ")}).`);
}

main();
