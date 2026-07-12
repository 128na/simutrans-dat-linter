import * as vscode from "vscode";
import { runDatLinter } from "./runner";

/**
 * Mirrors the JSON payload of `dat_linter keys --format json` (see
 * ../../docs/keys.md and ../../src/commands/keys.rs `JsonKeysOutput` on the
 * Rust side for the source of truth):
 *
 *   {
 *     "obj_types": [
 *       { "obj_type": "building", "keys": ["obj", "name", "copyright", "type", "waytype", "..."] }
 *     ],
 *     "known_values": {
 *       "waytype": ["none", "road", "track", "..."],
 *       "direction": ["s", "w", "sw", "se", "n", "e", "ne", "nw"],
 *       "per_obj_type": [
 *         { "obj_type": "building", "key": "type", "values": ["res", "com", "...", "station", "..."] }
 *       ]
 *     }
 *   }
 *
 * `obj_types[].keys` drives key-name completion; `known_values.waytype` /
 * `known_values.direction` and `known_values.per_obj_type` drive value
 * completion (see `createCompletionItemProvider` below).
 */
export interface DatLinterObjTypeKeys {
  obj_type: string;
  keys: string[];
}

export interface DatLinterPerObjTypeValues {
  obj_type: string;
  key: string;
  values: string[];
}

export interface DatLinterKnownValues {
  waytype: string[];
  direction: string[];
  per_obj_type: DatLinterPerObjTypeValues[];
}

export interface DatLinterKeysJson {
  obj_types: DatLinterObjTypeKeys[];
  known_values: DatLinterKnownValues;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

function isDatLinterObjTypeKeys(value: unknown): value is DatLinterObjTypeKeys {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return typeof v.obj_type === "string" && isStringArray(v.keys);
}

function isDatLinterPerObjTypeValues(value: unknown): value is DatLinterPerObjTypeValues {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return typeof v.obj_type === "string" && typeof v.key === "string" && isStringArray(v.values);
}

function isDatLinterKnownValues(value: unknown): value is DatLinterKnownValues {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return (
    isStringArray(v.waytype) &&
    isStringArray(v.direction) &&
    Array.isArray(v.per_obj_type) &&
    v.per_obj_type.every(isDatLinterPerObjTypeValues)
  );
}

/**
 * Type guard for the top-level `dat_linter keys --format json` payload.
 * Deliberately does not reject unknown extra keys (mirrors parser.ts's
 * `isDatLinterJsonOutput`), so additive schema changes on the Rust side
 * don't break parsing; only the fields this extension actually reads are
 * validated.
 */
export function isDatLinterKeysJson(value: unknown): value is DatLinterKeysJson {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return (
    Array.isArray(v.obj_types) &&
    v.obj_types.every(isDatLinterObjTypeKeys) &&
    isDatLinterKnownValues(v.known_values)
  );
}

/**
 * Parses the stdout of `dat_linter keys --format json` into a
 * `DatLinterKeysJson`. Throws a descriptive Error (mirroring
 * parser.ts's `parseDatLinterJson`) when the payload isn't valid JSON or
 * doesn't match the expected schema, so a dat_linter version too old to
 * support this data fails loudly instead of silently producing broken
 * completions.
 */
export function parseDatLinterKeysJson(stdout: string): DatLinterKeysJson {
  let payload: unknown;
  try {
    payload = JSON.parse(stdout);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`dat_linter keys --format json produced invalid JSON: ${message}`);
  }

  if (!isDatLinterKeysJson(payload)) {
    throw new Error(
      "dat_linter keys --format json output did not match the expected schema " +
        "(obj_types[].{obj_type,keys} + known_values.{waytype,direction,per_obj_type[]}). " +
        "The dat_linter executable may be a version this extension does not support."
    );
  }

  return payload;
}

function isSkippableLine(line: string): boolean {
  // Mirrors src/parser.rs `parse_records` (`line.is_empty() || line.starts_with('#') ||
  // line.starts_with(' ')`), which in turn mirrors real makeobj's
  // `tabfile_t::read_line()` (`dataobj/tabfile.cc`: the read loop skips a line only
  // when `*dest == '#' || *dest == ' '`). Deliberately does NOT skip tab
  // (`\t`)-indented lines: neither the Rust parser nor real makeobj treats a
  // tab-led line as a skippable comment/continuation, so this must not either
  // (checked against both sources before writing this comment; a prior
  // suggestion to add `line.startsWith("\t")` here was rejected precisely
  // because it would have diverged from actual parser/makeobj behavior).
  return line.length === 0 || line.startsWith("#") || line.startsWith(" ");
}

function isRecordSeparatorLine(line: string): boolean {
  // Mirrors src/parser.rs `parse_records`: real makeobj's tabfile_t::read()
  // (`while(read_line(...) && *line != '-')`) treats ANY line whose first
  // character is `-` as a record separator -- not specifically an
  // all-dashes divider line (real-world files commonly use a long run of
  // dashes as this separator, e.g. testdata/multi_object_building.dat, but
  // the parser doesn't require that shape).
  return line.startsWith("-");
}

/**
 * Determines the obj type (the value of the governing `obj=` line) for a
 * given cursor line, honoring the `-`-prefixed record separator that lets a
 * single .dat file concatenate multiple obj definitions (see
 * src/parser.rs `parse_records` / `DatFile::parse_all`; example fixture:
 * testdata/multi_object_building.dat).
 *
 * Searches upward from `cursorLine` for the nearest separator line at or
 * above it. If one is found, only lines strictly below that separator (down
 * to and including `cursorLine`) are considered when looking for `obj=`; if
 * none is found, the search spans from the top of the file. Within that
 * range, the *first* `obj=` line (in source order) wins, mirroring the
 * parser's first-wins duplicate-key handling (`tabfileobj_t::put()` keeps
 * the first value for a duplicated key).
 *
 * Returns `undefined` if no `obj=` line is found in the resulting range, or
 * if it has no non-empty value.
 *
 * Deliberately more lenient than the real parser in one respect: the `obj=`
 * value is trimmed and lowercased before being returned (the real parser
 * does neither), since this function only feeds completion-candidate
 * lookups keyed by lowercase obj type names -- not diagnostics that must
 * reproduce makeobj's exact (non-trimming) value semantics.
 */
export function findObjTypeAtLine(lines: readonly string[], cursorLine: number): string | undefined {
  if (lines.length === 0) {
    return undefined;
  }
  const clampedCursor = Math.max(0, Math.min(cursorLine, lines.length - 1));

  let recordStart = 0;
  for (let i = clampedCursor; i >= 0; i--) {
    if (isRecordSeparatorLine(lines[i])) {
      recordStart = i + 1;
      break;
    }
  }

  for (let i = recordStart; i <= clampedCursor; i++) {
    const line = lines[i];
    if (isSkippableLine(line) || isRecordSeparatorLine(line)) {
      continue;
    }
    const eq = line.indexOf("=");
    if (eq === -1) {
      continue;
    }
    const key = line.slice(0, eq).replace(/\s+$/, "").toLowerCase();
    if (key !== "obj") {
      continue;
    }
    const value = line.slice(eq + 1).trim().toLowerCase();
    if (value.length > 0) {
      return value;
    }
    return undefined;
  }

  return undefined;
}

/**
 * Caches the result of `dat_linter keys --format json`, fetched once at
 * extension activation (see `registerCompletionItemProvider` below) rather
 * than once per completion request -- spawning a child process on every
 * keystroke would make completions noticeably laggy.
 *
 * If loading fails (executable missing, too old to support `keys` or
 * `--format json`, unexpected schema, ...), `get()` returns `undefined` and
 * completions are silently unavailable: unlike lint/fmt failures, this is
 * not surfaced via an error message popup (completion is a nice-to-have,
 * not something a missing/outdated dat_linter should nag the user about on
 * every file open). The failure reason is still logged to `outputChannel`
 * for troubleshooting.
 *
 * Also guards against a race: `load()` can be called again (see
 * `registerDatCompletionItemProvider`'s `onDidChangeConfiguration` handler)
 * before a previous, still in-flight call has resolved -- e.g. a user
 * quickly edits `simutransDatLinter.executablePath` twice (a typo, then a
 * correction). Without ordering protection, if the *earlier* call's process
 * happens to resolve *after* the later (correct) call's, its stale result
 * would overwrite the fresher one, leaving completions permanently broken
 * despite the setting now being correct. Each `load()` call records a
 * monotonically increasing generation number at entry and only applies its
 * result if it's still the most recent call by the time it resolves.
 */
export class KeysDataCache {
  private data: DatLinterKeysJson | undefined;
  private generation = 0;

  get(): DatLinterKeysJson | undefined {
    return this.data;
  }

  /**
   * `fetchKeysJson` is a seam for tests to inject a controllable-timing,
   * canned result instead of spawning a real dat_linter process (see
   * test/completion.test.ts's race-condition suite); production callers rely
   * on the default, which runs the real `dat_linter keys --format json`.
   */
  async load(
    executablePath: string,
    cwd: string,
    outputChannel: vscode.OutputChannel,
    fetchKeysJson: (executablePath: string, cwd: string) => Promise<string> = (exe, dir) =>
      runDatLinter(exe, ["keys", "--format", "json"], dir)
  ): Promise<void> {
    const generation = ++this.generation;
    try {
      const stdout = await fetchKeysJson(executablePath, cwd);
      const parsed = parseDatLinterKeysJson(stdout);
      if (generation !== this.generation) {
        // A newer load() call was issued while this one was in flight;
        // discard this now-stale result rather than clobbering the newer one.
        return;
      }
      this.data = parsed;
      outputChannel.appendLine(
        `dat_linter: loaded "keys --format json" data (${this.data.obj_types.length} obj types) for completion.`
      );
    } catch (err) {
      if (generation !== this.generation) {
        return;
      }
      this.data = undefined;
      const message = err instanceof Error ? err.message : String(err);
      outputChannel.appendLine(
        `dat_linter: failed to load "keys --format json" data; key/value completion will be unavailable. ${message}`
      );
    }
  }
}

/**
 * Builds the document's lines as a plain string array, for feeding into the
 * pure `findObjTypeAtLine` helper.
 *
 * Uses a single `getText()` call + split rather than looping
 * `document.lineAt(i)` per line: each `lineAt` call is a bridge call into the
 * VSCode extension host, so looping it over every line in the document is
 * needlessly slow for large .dat files.
 */
function documentLines(document: vscode.TextDocument): string[] {
  return document.getText().split(/\r?\n/);
}

/**
 * Builds the single CompletionItemProvider registered for the
 * `simutrans-dat` language, backed by `getKeysData` (normally
 * `() => cache.get()`, injected here so `test/completion.test.ts` can supply
 * canned data without spawning a real dat_linter process).
 *
 * Returns no items when the cursor's line, trimmed, starts with `#` (comment)
 * or `-` (record separator) -- neither is a place a key or value belongs, so
 * offering completions there would just be noise.
 *
 * Otherwise decides between the two completion modes (see CLAUDE.md task
 * description) purely from whether the text before the cursor on the current
 * line already contains a `=`:
 *   - no `=` yet: still typing the key name -> offer the current record's
 *     obj type's valid keys (`obj_types[].keys`).
 *   - `=` already present: typing the value -> offer known values for that
 *     specific key. `obj` is special-cased to offer every obj type name
 *     (`obj_types[].obj_type`), since it's the key that determines what
 *     "current record's obj type" even means for every other key.
 *     `waytype`/`direction` are looked up directly in `known_values`
 *     (obj-type-independent); any other key is looked up in
 *     `known_values.per_obj_type` scoped to (current obj type, key).
 */
export function createCompletionItemProvider(
  getKeysData: () => DatLinterKeysJson | undefined
): vscode.CompletionItemProvider {
  return {
    provideCompletionItems(
      document: vscode.TextDocument,
      position: vscode.Position
    ): vscode.CompletionItem[] | undefined {
      const data = getKeysData();
      if (!data) {
        return undefined;
      }

      const lineText = document.lineAt(position.line).text;
      const trimmedLine = lineText.trim();
      if (trimmedLine.startsWith("#") || trimmedLine.startsWith("-")) {
        return undefined;
      }

      const objType = findObjTypeAtLine(documentLines(document), position.line);

      const linePrefix = lineText.slice(0, position.character);
      const eqIndex = linePrefix.indexOf("=");

      if (eqIndex === -1) {
        if (!objType) {
          return undefined;
        }
        const entry = data.obj_types.find((o) => o.obj_type === objType);
        if (!entry) {
          return undefined;
        }
        return entry.keys.map(
          (key) => new vscode.CompletionItem(key, vscode.CompletionItemKind.Property)
        );
      }

      const key = linePrefix.slice(0, eqIndex).trim().toLowerCase();
      let values: string[] | undefined;
      if (key === "obj") {
        values = data.obj_types.map((o) => o.obj_type);
      } else if (key === "waytype") {
        values = data.known_values.waytype;
      } else if (key === "direction") {
        values = data.known_values.direction;
      } else if (objType) {
        values = data.known_values.per_obj_type.find(
          (v) => v.obj_type === objType && v.key === key
        )?.values;
      }
      if (!values || values.length === 0) {
        return undefined;
      }
      return values.map(
        (value) => new vscode.CompletionItem(value, vscode.CompletionItemKind.EnumMember)
      );
    },
  };
}

/**
 * Loads the `keys --format json` cache and registers the completion
 * provider for the `simutrans-dat` language. Called once from
 * extension.ts's `activate`.
 *
 * The cache load is fire-and-forget (`void cache.load(...)`): completion
 * requests that race ahead of it simply see `cache.get() === undefined` and
 * return no items (see `KeysDataCache`'s doc comment) until it resolves,
 * rather than activation blocking on a child process before VSCode
 * considers the extension started.
 *
 * Also watches for changes to the `simutransDatLinter.executablePath` setting
 * and re-runs `cache.load(...)` with the updated path when it changes, so a
 * user who edits the setting after activation (e.g. because they only just
 * installed dat_linter, or switched to a different binary) doesn't have to
 * reload the whole window to get completions working.
 */
export function registerDatCompletionItemProvider(
  context: vscode.ExtensionContext,
  outputChannel: vscode.OutputChannel
): void {
  const cache = new KeysDataCache();

  const config = vscode.workspace.getConfiguration("simutransDatLinter");
  const executablePath = config.get<string>("executablePath", "dat_linter");
  const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();

  void cache.load(executablePath, cwd, outputChannel);

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("simutransDatLinter.executablePath")) {
        const updatedConfig = vscode.workspace.getConfiguration("simutransDatLinter");
        const updatedExecutablePath = updatedConfig.get<string>("executablePath", "dat_linter");
        void cache.load(updatedExecutablePath, cwd, outputChannel);
      }
    }),
    vscode.languages.registerCompletionItemProvider(
      { language: "simutrans-dat" },
      createCompletionItemProvider(() => cache.get()),
      "="
    )
  );
}
