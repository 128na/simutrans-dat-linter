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

// Collects every value listed under `known_values.per_obj_type` for a given
// `key` (e.g. "type", "location", "climates", "name"), across every obj_type
// that reports it, deduplicated. dat_linter scopes these per obj_type (e.g.
// `{obj_type: "building", key: "type", values: [...]}`), but the grammar
// cannot tell which obj_type a given line belongs to (see the big comment on
// `defined_values` below), so we deliberately flatten obj_type away here and
// treat this as "value known to be valid for this key on *some* obj_type".
function collectPerObjTypeValues(perObjType, key) {
  const valueSet = new Set();
  for (const entry of perObjType ?? []) {
    if (entry.key !== key) continue;
    for (const value of entry.values ?? []) {
      valueSet.add(value);
    }
  }
  // Same longest-first rationale as collectKeys.
  return [...valueSet].sort((a, b) => b.length - a.length || a.localeCompare(b));
}

function buildGrammar(keys, waytypes, directions, buildingTypes, locations, climates, skinNames) {
  const keysAlternation = keys.map(escapeRegex).join("|");
  const waytypesAlternation = waytypes.map(escapeRegex).join("|");
  const directionsAlternation = directions.map(escapeRegex).join("|");
  // Each of these may legitimately be empty (an older dat_linter without
  // `known_values.per_obj_type` support, or a future obj_type dropping a
  // category) -- unlike keys/waytypes/directions this isn't fatal, we just
  // skip that pattern below rather than emitting `(?i:)$`, which would match
  // an empty string at every line ending.
  const buildingTypesAlternation = buildingTypes.map(escapeRegex).join("|");
  const locationsAlternation = locations.map(escapeRegex).join("|");
  const climatesAlternation = climates.map(escapeRegex).join("|");
  const skinNamesAlternation = skinNames.map(escapeRegex).join("|");

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
          // waytype/direction come from the flat `known_values.waytype` /
          // `known_values.direction` lists. The remaining categories come from
          // `known_values.per_obj_type`, which scopes each value list to a
          // specific obj_type (e.g. `type` values differ in meaning between
          // `building` and `factory`). TextMate grammars can't track "what
          // obj_type is this line inside of" without a much bigger stateful
          // rewrite (see the module-level design note in generate-grammar.mjs),
          // so -- same as the existing waytype/direction scopes -- every
          // obj_type's values for a given key are flattened into one alternation
          // per category. This means a value is highlighted as "plausibly valid
          // for this key on some obj_type", not "valid for the obj_type this
          // particular line belongs to"; `dat_linter lint` remains the source of
          // truth for actual per-obj_type correctness.
          //
          // Patterns are tried in array order and the first match wins for a
          // given position, so this order also resolves the (few) value
          // strings that collide across categories -- e.g. "water" appears in
          // both `location` and `climates` (and even the existing waytype
          // list), and building-type values like "post"/"busstop"/"carstop"/
          // "monorailstop" collide with cursor/symbol skin names of the same
          // spelling. Order below: waytype/direction (unchanged, existing
          // behavior) first, then building-types/locations/climates as they
          // apply to keys addon authors set constantly, with skin-names last
          // since those only ever apply to the small set of built-in system
          // obj_types (menu/cursor/symbol/misc/ground) that ordinary addons
          // never define.
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
              ...(buildingTypesAlternation
                ? [
                    {
                      name: "storage.type.building-types.simutrans-dat",
                      match: `\\b(?i:${buildingTypesAlternation})$`,
                    },
                  ]
                : []),
              ...(locationsAlternation
                ? [
                    {
                      name: "storage.type.locations.simutrans-dat",
                      match: `\\b(?i:${locationsAlternation})$`,
                    },
                  ]
                : []),
              ...(climatesAlternation
                ? [
                    {
                      name: "storage.type.climates.simutrans-dat",
                      match: `\\b(?i:${climatesAlternation})$`,
                    },
                  ]
                : []),
              ...(skinNamesAlternation
                ? [
                    {
                      name: "storage.type.skin-names.simutrans-dat",
                      match: `\\b(?i:${skinNamesAlternation})$`,
                    },
                  ]
                : []),
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
  const perObjType = data.known_values?.per_obj_type ?? [];
  const buildingTypes = collectPerObjTypeValues(perObjType, "type");
  const locations = collectPerObjTypeValues(perObjType, "location");
  const climates = collectPerObjTypeValues(perObjType, "climates");
  const skinNames = collectPerObjTypeValues(perObjType, "name");

  if (keys.length === 0) {
    fail("dat_linter reported zero keys across all obj_types -- refusing to generate an empty grammar.");
  }
  if (waytypes.length === 0) {
    fail("dat_linter reported zero known_values.waytype -- refusing to generate a grammar without waytypes.");
  }
  if (directions.length === 0) {
    fail("dat_linter reported zero known_values.direction -- refusing to generate a grammar without directions.");
  }
  // per_obj_type-derived categories are additive value highlighting, not a
  // core requirement like keys/waytypes/directions above -- an older
  // dat_linter without `known_values.per_obj_type` support (or a future one
  // that stops reporting a given key) should still produce a working grammar,
  // just without that category's highlighting (see the empty-alternation
  // guards in buildGrammar).
  for (const [label, values] of [
    ["type", buildingTypes],
    ["location", locations],
    ["climates", climates],
    ["name", skinNames],
  ]) {
    if (values.length === 0) {
      console.warn(
        `generate-grammar: dat_linter reported zero known_values.per_obj_type entries for key "${label}" -- ` +
          `skipping that value-highlighting category (this is not fatal).`
      );
    }
  }

  const grammar = buildGrammar(keys, waytypes, directions, buildingTypes, locations, climates, skinNames);
  writeFileSync(OUT_PATH, JSON.stringify(grammar, null, 2) + "\n", "utf8");
  console.log(
    `generate-grammar: wrote ${keys.length} keys, ${waytypes.length} waytypes, ${directions.length} directions, ` +
      `${buildingTypes.length} building-types, ${locations.length} locations, ${climates.length} climates, ` +
      `${skinNames.length} skin-names to ${path.relative(process.cwd(), OUT_PATH)}`
  );
}

main();
