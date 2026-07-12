import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";
import {
  createCompletionItemProvider,
  findObjTypeAtLine,
  isDatLinterKeysJson,
  parseDatLinterKeysJson,
  DatLinterKeysJson,
  KeysDataCache,
} from "../src/completion";

const EXTENSION_ID = "128na.simutrans-dat-linter";

// out/test -> out -> vscode -> editors -> simutrans-dat-linter (this repo's root)
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");
const TESTDATA_DIR = path.join(REPO_ROOT, "testdata");

// The schema example from `dat_linter keys --format json` (see ../../docs/keys.md
// and ../src/commands/keys.rs on the Rust side).
const SAMPLE_KEYS_JSON = JSON.stringify({
  obj_types: [
    { obj_type: "building", keys: ["obj", "name", "copyright", "type", "waytype", "cursor", "icon"] },
    { obj_type: "vehicle", keys: ["obj", "name", "copyright", "waytype", "speed", "constraint"] },
  ],
  known_values: {
    waytype: ["none", "road", "track", "water", "air"],
    direction: ["s", "w", "sw", "se", "n", "e", "ne", "nw"],
    per_obj_type: [
      { obj_type: "building", key: "type", values: ["res", "com", "ind", "station", "extension"] },
      { obj_type: "factory", key: "location", values: ["land", "water", "city"] },
    ],
  },
});

suite("completion: findObjTypeAtLine (pure function)", () => {
  test("returns the obj type for a cursor line within a single-record file", () => {
    const lines = ["obj=building", "name=Foo", "waytype=track"];
    assert.strictEqual(findObjTypeAtLine(lines, 2), "building");
  });

  test("returns undefined when there is no obj= line above the cursor", () => {
    const lines = ["name=Foo", "waytype=track"];
    assert.strictEqual(findObjTypeAtLine(lines, 1), undefined);
  });

  test("returns undefined for an empty file", () => {
    assert.strictEqual(findObjTypeAtLine([], 0), undefined);
  });

  // Mirrors testdata/multi_object_building.dat: multiple obj definitions
  // concatenated in one file, separated by `-`-prefixed lines. Uses two
  // *different* obj types (unlike the fixture, which uses "building" for all
  // three records) so this test actually exercises per-record obj type
  // detection, not just record-boundary detection.
  const multiRecordLines = [
    "obj=building",
    "name=StageA",
    "waytype=track",
    "",
    "-------------------------------------------------------------------------------",
    "obj=vehicle",
    "name=Loco",
    "waytype=track",
    "speed=100",
    "-------------------------------------------------------------------------------",
    "obj=building",
    "name=StageC",
    "waytype=track",
  ];

  test("resolves the first record's obj type for a cursor line inside it", () => {
    assert.strictEqual(findObjTypeAtLine(multiRecordLines, 2), "building");
  });

  test("resolves the second record's (different) obj type for a cursor line inside it", () => {
    assert.strictEqual(findObjTypeAtLine(multiRecordLines, 8), "vehicle");
  });

  test("resolves the third record's obj type for a cursor line inside it", () => {
    assert.strictEqual(findObjTypeAtLine(multiRecordLines, 12), "building");
  });

  test("does not leak a later record's obj= backward across a separator", () => {
    // Cursor sits right after the first separator but before the second
    // record's own obj= line has been typed yet: nothing in [recordStart,
    // cursorLine] has an obj=, so this must be undefined, not "building"
    // (the previous record) or "vehicle" (the next one).
    const lines = [
      "obj=building",
      "name=StageA",
      "-------------------------------------------------------------------------------",
      "name=Loco",
    ];
    assert.strictEqual(findObjTypeAtLine(lines, 3), undefined);
  });

  test("first obj= line within a record wins over a later duplicate (first-wins, mirrors the Rust parser)", () => {
    const lines = ["obj=building", "obj=vehicle", "name=Foo"];
    assert.strictEqual(findObjTypeAtLine(lines, 2), "building");
  });

  test("obj value is trimmed and lowercased for matching purposes", () => {
    const lines = ["obj= Building ", "name=Foo"];
    assert.strictEqual(findObjTypeAtLine(lines, 1), "building");
  });

  test("skips blank, comment, and indented lines without losing the record's obj type", () => {
    const lines = ["obj=building", "", "# a comment", "  indented continuation", "waytype=track"];
    assert.strictEqual(findObjTypeAtLine(lines, 4), "building");
  });

  test("a tab-indented line is NOT treated as a skippable continuation (matches src/parser.rs and real makeobj's tabfile_t::read_line, which only skip '#' and ' ')", () => {
    // "\tobj=vehicle" is not skipped, but its extracted key is "\tobj" (tab
    // preserved, since only trailing whitespace is trimmed), which never
    // equals "obj" -- so it's simply ignored as an unrecognized key, and the
    // real "obj=building" line above still governs. This exercises the same
    // "unrecognized key gets skipped without affecting obj type" codepath
    // that a tab-conscious isSkippableLine would use for a *different*
    // reason (treating it as a skippable continuation) -- but the outcome
    // for the current cursor line must remain "building" either way.
    const lines = ["obj=building", "\tobj=vehicle", "name=Foo"];
    assert.strictEqual(findObjTypeAtLine(lines, 2), "building");
  });

  test("a record separator is any line starting with '-', not only an all-dashes line", () => {
    const lines = ["obj=building", "name=StageA", "-- not a full dash rule --", "obj=vehicle", "name=Loco"];
    assert.strictEqual(findObjTypeAtLine(lines, 4), "vehicle");
  });

  test("clamps an out-of-range cursor line to the last line", () => {
    const lines = ["obj=building", "name=Foo"];
    assert.strictEqual(findObjTypeAtLine(lines, 999), "building");
  });
});

suite("completion: parseDatLinterKeysJson / isDatLinterKeysJson (pure functions)", () => {
  test("parses a well-formed payload", () => {
    const parsed = parseDatLinterKeysJson(SAMPLE_KEYS_JSON);
    assert.strictEqual(parsed.obj_types.length, 2);
    assert.deepStrictEqual(
      parsed.obj_types.find((o) => o.obj_type === "building")?.keys,
      ["obj", "name", "copyright", "type", "waytype", "cursor", "icon"]
    );
    assert.deepStrictEqual(parsed.known_values.waytype, ["none", "road", "track", "water", "air"]);
    assert.strictEqual(parsed.known_values.per_obj_type.length, 2);
  });

  test("throws a descriptive error on invalid JSON", () => {
    assert.throws(() => parseDatLinterKeysJson("not json{"), /invalid JSON/);
  });

  test("throws a descriptive error when the payload doesn't match the schema", () => {
    const malformed = JSON.stringify({ obj_types: [] }); // missing known_values
    assert.throws(() => parseDatLinterKeysJson(malformed), /did not match the expected schema/);
  });

  test("isDatLinterKeysJson rejects a non-object payload", () => {
    assert.strictEqual(isDatLinterKeysJson(null), false);
    assert.strictEqual(isDatLinterKeysJson("hello"), false);
  });

  test("isDatLinterKeysJson rejects obj_types entries with non-string keys", () => {
    const malformed = {
      obj_types: [{ obj_type: "building", keys: ["obj", 42] }],
      known_values: { waytype: [], direction: [], per_obj_type: [] },
    };
    assert.strictEqual(isDatLinterKeysJson(malformed), false);
  });

  test("isDatLinterKeysJson accepts the sample payload", () => {
    assert.strictEqual(isDatLinterKeysJson(JSON.parse(SAMPLE_KEYS_JSON)), true);
  });
});

suite("completion: createCompletionItemProvider (pure logic, canned keys data)", () => {
  const data: DatLinterKeysJson = JSON.parse(SAMPLE_KEYS_JSON);
  const provider = createCompletionItemProvider(() => data);

  test("offers key-name completions for the record's obj type when no '=' precedes the cursor", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\nname=Foo\nway",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(2, 3); // end of "way" on line 3
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.Invoke, triggerCharacter: undefined }
    ) as vscode.CompletionItem[];

    assert.ok(items, "expected completion items, got none");
    const labels = items.map((i) => i.label);
    assert.ok(labels.includes("waytype"), `expected "waytype" among ${JSON.stringify(labels)}`);
    assert.ok(labels.includes("cursor"), `expected "cursor" among ${JSON.stringify(labels)}`);
    // Sanity check this isn't just "every obj type's keys" -- "speed" only
    // belongs to the "vehicle" obj type's key list, not "building"'s.
    assert.ok(!labels.includes("speed"), `did not expect "speed" among ${JSON.stringify(labels)}`);
  });

  test("returns undefined for key-name completion when no obj= governs the cursor line", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "name=Foo\nway",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(1, 3);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.Invoke, triggerCharacter: undefined }
    );
    assert.strictEqual(items, undefined);
  });

  test("offers known_values.waytype completions for a waytype= value", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\nwaytype=",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(1, "waytype=".length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.TriggerCharacter, triggerCharacter: "=" }
    ) as vscode.CompletionItem[];

    assert.ok(items, "expected completion items, got none");
    assert.deepStrictEqual(
      items.map((i) => i.label),
      ["none", "road", "track", "water", "air"]
    );
  });

  test("offers per_obj_type completions scoped to (obj type, key) for a building's type= value", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\ntype=",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(1, "type=".length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.TriggerCharacter, triggerCharacter: "=" }
    ) as vscode.CompletionItem[];

    assert.ok(items, "expected completion items, got none");
    assert.deepStrictEqual(
      items.map((i) => i.label),
      ["res", "com", "ind", "station", "extension"]
    );
  });

  test("does not offer factory's location= values when the record's obj type is building", async () => {
    // factory's "location" key has known values in SAMPLE_KEYS_JSON, but the
    // record here is "building" -- (building, location) has no entry, so no
    // completions should be offered.
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\nlocation=",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(1, "location=".length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.TriggerCharacter, triggerCharacter: "=" }
    );
    assert.strictEqual(items, undefined);
  });

  test("offers every obj type name for an 'obj=' value", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "obj=",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(0, "obj=".length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.TriggerCharacter, triggerCharacter: "=" }
    ) as vscode.CompletionItem[];

    assert.ok(items, "expected completion items, got none");
    assert.deepStrictEqual(
      items.map((i) => i.label),
      ["building", "vehicle"]
    );
  });

  test("key extraction trims leading whitespace, not just trailing", async () => {
    // Regression test: the key slice used to only trim trailing whitespace
    // (`.replace(/\s+$/, "")`), so a value line with leading whitespace
    // before the key (e.g. "  type=") would fail to match "type" and offer
    // no completions. `.trim()` fixes this symmetrically.
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\n  type=",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(1, "  type=".length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.TriggerCharacter, triggerCharacter: "=" }
    ) as vscode.CompletionItem[];

    assert.ok(items, "expected completion items, got none");
    assert.deepStrictEqual(
      items.map((i) => i.label),
      ["res", "com", "ind", "station", "extension"]
    );
  });

  test("returns no completions when the cursor's line is a '#' comment", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\n# waytype=",
      language: "simutrans-dat",
    });
    const position = new vscode.Position(1, "# waytype=".length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.TriggerCharacter, triggerCharacter: "=" }
    );
    assert.strictEqual(items, undefined);
  });

  test("returns no completions when the cursor's line is a '-' record separator", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\n-------------------------------------------------------------------------------",
      language: "simutrans-dat",
    });
    const line1 = document.lineAt(1).text;
    const position = new vscode.Position(1, line1.length);
    const items = provider.provideCompletionItems!(
      document,
      position,
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.Invoke, triggerCharacter: undefined }
    );
    assert.strictEqual(items, undefined);
  });

  test("returns undefined when no keys data is cached (e.g. dat_linter failed to load)", async () => {
    const noDataProvider = createCompletionItemProvider(() => undefined);
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\nway",
      language: "simutrans-dat",
    });
    const items = noDataProvider.provideCompletionItems!(
      document,
      new vscode.Position(1, 3),
      new vscode.CancellationTokenSource().token,
      { triggerKind: vscode.CompletionTriggerKind.Invoke, triggerCharacter: undefined }
    );
    assert.strictEqual(items, undefined);
  });
});

suite("completion: KeysDataCache.load races (generation guard)", () => {
  const makeOutputChannel = (): vscode.OutputChannel =>
    ({ appendLine: () => undefined } as unknown as vscode.OutputChannel);

  const keysJson = (waytype: string): string =>
    JSON.stringify({
      obj_types: [],
      known_values: { waytype: [waytype], direction: [], per_obj_type: [] },
    });

  test("a slower, earlier load's success does not clobber a faster, later load's success", async () => {
    const cache = new KeysDataCache();
    const outputChannel = makeOutputChannel();

    // First load() is issued first but resolves later ("STALE" result);
    // second load() is issued second but resolves sooner ("FRESH" result).
    // Without the generation guard, the first call's late resolution would
    // overwrite the second call's already-applied fresher result.
    const firstLoad = cache.load("dat_linter", ".", outputChannel, async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
      return keysJson("STALE");
    });
    const secondLoad = cache.load("dat_linter", ".", outputChannel, async () => keysJson("FRESH"));

    await Promise.all([firstLoad, secondLoad]);

    assert.deepStrictEqual(cache.get()?.known_values.waytype, ["FRESH"]);
  });

  test("a slower, earlier load's failure does not clear a faster, later load's success", async () => {
    const cache = new KeysDataCache();
    const outputChannel = makeOutputChannel();

    const firstLoad = cache.load("dat_linter", ".", outputChannel, async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
      throw new Error("stale spawn failure");
    });
    const secondLoad = cache.load("dat_linter", ".", outputChannel, async () => keysJson("FRESH"));

    await Promise.all([firstLoad, secondLoad]);

    assert.deepStrictEqual(cache.get()?.known_values.waytype, ["FRESH"]);
  });

  test("a slower, earlier load's success does not resurrect data over a faster, later load's failure", async () => {
    const cache = new KeysDataCache();
    const outputChannel = makeOutputChannel();

    const firstLoad = cache.load("dat_linter", ".", outputChannel, async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
      return keysJson("STALE");
    });
    const secondLoad = cache.load("dat_linter", ".", outputChannel, async () => {
      throw new Error("newer executablePath is broken");
    });

    await Promise.all([firstLoad, secondLoad]);

    assert.strictEqual(cache.get(), undefined);
  });

  test("without any race, the only load's result is applied normally", async () => {
    const cache = new KeysDataCache();
    const outputChannel = makeOutputChannel();

    await cache.load("dat_linter", ".", outputChannel, async () => keysJson("track"));

    assert.deepStrictEqual(cache.get()?.known_values.waytype, ["track"]);
  });
});

suite("dat_linter VSCode extension completion integration (real dat_linter, real activation)", () => {
  suiteSetup(async function () {
    this.timeout(30000);
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} not found - is it registered under that id?`);
    await ext!.activate();
  });

  test("key-name completion via the real dat_linter binary includes a known building key", async function () {
    this.timeout(20000);
    const filePath = path.join(TESTDATA_DIR, "duplicate_key.dat");
    const document = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
    // Append a partial key on a new line at the end of the document.
    const editor = await vscode.window.showTextDocument(document);
    const endPosition = document.lineAt(document.lineCount - 1).range.end;
    await editor.edit((editBuilder) => {
      editBuilder.insert(endPosition, "\nway");
    });
    const insertedLine = document.lineCount - 1;
    const position = new vscode.Position(insertedLine, document.lineAt(insertedLine).text.length);

    try {
      const list = await vscode.commands.executeCommand<vscode.CompletionList>(
        "vscode.executeCompletionItemProvider",
        document.uri,
        position
      );
      const labels = list.items.map((i) => i.label);
      assert.ok(
        labels.includes("waytype"),
        `expected "waytype" among real dat_linter-backed completions, got: ${JSON.stringify(labels)}`
      );
    } finally {
      // Never persist this edit to testdata/duplicate_key.dat: revert
      // discards the dirty in-memory buffer before closing (best-effort,
      // mirroring extension.test.ts's cleanup pattern for scratch edits).
      try {
        await vscode.commands.executeCommand("workbench.action.revertFile");
      } catch {
        // best-effort cleanup
      }
      try {
        await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      } catch {
        // best-effort cleanup
      }
    }
  });

  test("value completion via the real dat_linter binary offers known waytype values", async function () {
    this.timeout(20000);
    const document = await vscode.workspace.openTextDocument({
      content: "obj=building\nname=Test\ntype=extension\nwaytype=",
      language: "simutrans-dat",
    });
    await vscode.window.showTextDocument(document);
    const position = new vscode.Position(3, document.lineAt(3).text.length);

    const list = await vscode.commands.executeCommand<vscode.CompletionList>(
      "vscode.executeCompletionItemProvider",
      document.uri,
      position
    );
    const labels = list.items.map((i) => i.label);
    assert.ok(
      labels.includes("track"),
      `expected a known waytype value among real dat_linter-backed completions, got: ${JSON.stringify(labels)}`
    );
  });

  test("changing simutransDatLinter.executablePath reloads the keys cache without a window reload", async function () {
    // Regression test: registerDatCompletionItemProvider used to load the
    // keys cache exactly once, at activation. Pointing executablePath at a
    // nonexistent binary *after* activation used to have no effect on
    // completions (the stale, still-valid cache kept serving them) -- now it
    // must re-load and clear the cache, and pointing it back must restore
    // completions again, all without reactivating the extension.
    this.timeout(30000);

    const waytypeDocument = async (): Promise<vscode.CompletionList> => {
      const document = await vscode.workspace.openTextDocument({
        content: "obj=building\nwaytype=",
        language: "simutrans-dat",
      });
      await vscode.window.showTextDocument(document);
      const position = new vscode.Position(1, document.lineAt(1).text.length);
      return vscode.commands.executeCommand<vscode.CompletionList>(
        "vscode.executeCompletionItemProvider",
        document.uri,
        position
      );
    };

    const pollUntil = async (check: () => Promise<boolean>, deadlineMs: number): Promise<boolean> => {
      const deadline = Date.now() + deadlineMs;
      for (;;) {
        if (await check()) {
          return true;
        }
        if (Date.now() > deadline) {
          return false;
        }
        await new Promise((resolve) => setTimeout(resolve, 200));
      }
    };

    const config = vscode.workspace.getConfiguration("simutransDatLinter");
    const originalExecutablePath = config.get<string>("executablePath");

    try {
      await config.update(
        "executablePath",
        "dat_linter_does_not_exist_xyz",
        vscode.ConfigurationTarget.Global
      );

      const brokenCacheTakesEffect = await pollUntil(async () => {
        const list = await waytypeDocument();
        // Don't assert `list.items.length === 0`: VSCode's built-in
        // word-based suggestions can still populate the list from words
        // already in the document, independent of our provider. What must
        // disappear is specifically our known-waytype-value completions.
        return !list.items.some((i) => i.label === "track");
      }, 15000);
      assert.ok(
        brokenCacheTakesEffect,
        "expected the known-waytype-value completion ('track') to disappear after pointing executablePath at a nonexistent binary"
      );

      await config.update("executablePath", originalExecutablePath, vscode.ConfigurationTarget.Global);

      const restoredCacheTakesEffect = await pollUntil(async () => {
        const list = await waytypeDocument();
        return list.items.some((i) => i.label === "track");
      }, 15000);
      assert.ok(
        restoredCacheTakesEffect,
        "expected completions to come back after restoring the original executablePath"
      );
    } finally {
      await config.update("executablePath", originalExecutablePath, vscode.ConfigurationTarget.Global);
    }
  });
});
