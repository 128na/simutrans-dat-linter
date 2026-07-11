#!/usr/bin/env node
// Generates syntaxes/simutrans-dat.tmLanguage.json from dat_linter's own
// structured key/value data (`dat_linter keys --format json`), so the
// grammar's key list can never drift from what the linter itself considers
// valid for each obj type.
//
// Usage: node scripts/generate-grammar.mjs
// Env:   DAT_LINTER_BIN - path to the dat_linter executable (default: "dat_linter")
//
// Deliberately depends on Node's standard library only (no devDependencies),
// so a developer without a Rust toolchain can still `npm install` and work
// on everything except this script.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUT_PATH = path.join(SCRIPT_DIR, "..", "syntaxes", "simutrans-dat.tmLanguage.json");
const DAT_LINTER_BIN = process.env.DAT_LINTER_BIN || "dat_linter";

function fail(message) {
  console.error(`generate-grammar: ${message}`);
  process.exit(1);
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function loadKeysData() {
  let stdout;
  try {
    stdout = execFileSync(DAT_LINTER_BIN, ["keys", "--format", "json"], {
      encoding: "utf8",
    });
  } catch (err) {
    if (err.code === "ENOENT") {
      fail(
        `"${DAT_LINTER_BIN}" executable was not found on PATH. Make sure dat_linter is installed ` +
          `and on PATH, or set the DAT_LINTER_BIN environment variable to its full path.`
      );
    } else {
      fail(
        `failed to run "${DAT_LINTER_BIN} keys --format json". It might be that the installed ` +
          `dat_linter version is too old and does not support the "keys" command or "--format json" ` +
          `option.\nUnderlying error: ${err.message}`
      );
    }
  }

  let data;
  try {
    data = JSON.parse(stdout);
  } catch (err) {
    fail(
      `"${DAT_LINTER_BIN} keys --format json" did not produce valid JSON on stdout: ${err.message}\n` +
        `stdout was:\n${stdout}`
    );
  }

  return data;
}

function collectKeys(data) {
  const keySet = new Set();
  for (const entry of data.obj_types ?? []) {
    for (const key of entry.keys ?? []) {
      keySet.add(key);
    }
  }
  // Longest-first so a prefix key (e.g. "image") can't shadow a longer key
  // that starts with it (e.g. "imageup") in the alternation regex — the
  // regex engine takes the first alternative that matches, so a shorter
  // prefix listed earlier would swallow only its own characters and leave
  // the rest of a longer key unscoped.
  return [...keySet].sort((a, b) => b.length - a.length || a.localeCompare(b));
}

function buildGrammar(keys, waytypes, directions) {
  const keysAlternation = keys.map(escapeRegex).join("|");
  const waytypesAlternation = waytypes.map(escapeRegex).join("|");
  const directionsAlternation = directions.map(escapeRegex).join("|");

  return {
    $schema: "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
    name: "simutrans-dat",
    scopeName: "source.simutrans-dat",
    patterns: [{ include: "#comment" }, { include: "#lines" }],
    repository: {
      comment: {
        patterns: [
          { name: "comment.line.simutrans-dat", match: "^#.*" },
          { name: "comment.line.simutrans-dat", match: "^-+$" },
        ],
      },
      lines: {
        patterns: [
          {
            // key [ "=>" | "=" ] value?
            match: "^([^=]+)(=> |=)?((\\d+)|([A-Za-z,_]+)|(.+))?$",
            captures: {
              "1": {
                patterns: [{ include: "#defined_keys" }, { include: "#direction_and_number" }],
              },
              "2": { patterns: [{ include: "#equal" }] },
              "4": { patterns: [{ include: "#numeric" }] },
              "5": { patterns: [{ include: "#defined_values" }] },
              "6": { patterns: [{ include: "#general_value" }] },
            },
          },
        ],
        repository: {
          numeric: {
            patterns: [{ name: "constant.numeric.simutrans-dat", match: "-?\\d+" }],
          },
          // Key names, generated from every obj_type's `keys` list returned by
          // `dat_linter keys --format json` (deduplicated, sorted). Case-insensitive
          // because dat_linter's own key matching is case-insensitive (e.g. BackImage
          // and backimage are equivalent).
          defined_keys: {
            patterns: [
              {
                name: "storage.type.defined-keys.simutrans-dat",
                match: `^(?i:${keysAlternation})`,
              },
            ],
          },
          // Known enum-like values, generated from dat_linter's `known_values`.
          // Scope intentionally limited to waytype/direction: dat_linter does not
          // yet expose structured data for other value families (type names,
          // location, climate, etc.), so those are left as #general_value.
          defined_values: {
            patterns: [
              {
                name: "storage.type.waytypes.simutrans-dat",
                match: `\\b(?i:${waytypesAlternation})$`,
              },
              {
                name: "storage.type.directions.simutrans-dat",
                match: `\\b(?i:${directionsAlternation})$`,
              },
            ],
          },
          general_value: {
            patterns: [
              {
                match: "[.,](-?\\d+)|((-?\\d+),)+?(-?\\d+)",
                captures: {
                  "1": { patterns: [{ include: "#numeric" }] },
                  "2": { patterns: [{ include: "#numeric" }] },
                  "3": { patterns: [{ include: "#numeric" }] },
                  "4": { patterns: [{ include: "#numeric" }] },
                },
              },
            ],
          },
          // Bracketed index/direction suffixes on keys, e.g. image[0], BackImage[n],
          // Constraint[Prev][0]. Only recognizes the 8 direction abbreviations
          // dat_linter reports via known_values.direction plus bare digits/"-";
          // combined forms like [nsew] or [new1] fall through unhighlighted.
          direction_and_number: {
            patterns: [
              {
                name: "variable.parameter.direction-and-number.simutrans-dat",
                match: `\\[(?i:${directionsAlternation}|-|\\d+)\\]`,
              },
            ],
          },
          equal: {
            patterns: [{ name: "keyword.operator.equal.simutrans-dat", match: "=> |=" }],
          },
        },
      },
    },
  };
}

function main() {
  const data = loadKeysData();
  const keys = collectKeys(data);
  const waytypes = data.known_values?.waytype ?? [];
  const directions = data.known_values?.direction ?? [];

  if (keys.length === 0) {
    fail("dat_linter reported zero keys across all obj_types -- refusing to generate an empty grammar.");
  }
  if (waytypes.length === 0) {
    fail("dat_linter reported zero known_values.waytype -- refusing to generate a grammar without waytypes.");
  }
  if (directions.length === 0) {
    fail("dat_linter reported zero known_values.direction -- refusing to generate a grammar without directions.");
  }

  const grammar = buildGrammar(keys, waytypes, directions);
  writeFileSync(OUT_PATH, JSON.stringify(grammar, null, 2) + "\n", "utf8");
  console.log(
    `generate-grammar: wrote ${keys.length} keys, ${waytypes.length} waytypes, ${directions.length} directions to ${path.relative(process.cwd(), OUT_PATH)}`
  );
}

main();
