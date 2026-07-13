import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

// package.json publisher "128na" + name "simutrans-dat-linter"
const EXTENSION_ID = "128na.simutrans-dat-linter";

// out/test/multi-root -> out/test -> out -> vscode (this extension's root)
const VSCODE_ROOT = path.resolve(__dirname, "..", "..", "..");
const MULTI_ROOT_FIXTURE = path.join(VSCODE_ROOT, "fixtures", "multi-root");
const FOLDER_A = path.join(MULTI_ROOT_FIXTURE, "folder-a");
const FOLDER_B = path.join(MULTI_ROOT_FIXTURE, "folder-b");

/**
 * Regression test for the gemini-code-assist review on PR #23: getting
 * `simutransDatLinter.lint.enable` / `format.enable` without scoping to the
 * document's own `Uri` (`vscode.workspace.getConfiguration("simutransDatLinter")`,
 * no second argument) always resolves to "one of the applicable scopes"'
 * value rather than the value that actually applies to that document -- in a
 * single-root workspace this is invisible (there is only one folder to
 * disagree with), but in a multi-root workspace with per-folder overrides it
 * silently applies the wrong folder's setting.
 *
 * This suite runs in its own `.vscode-test.mjs` configuration (see
 * "multi-root-workspace" there), which opens
 * `fixtures/multi-root/multi-root.code-workspace` -- a two-folder workspace
 * where `folder-a/.vscode/settings.json` sets `lint.enable`/`format.enable`
 * to `true` and `folder-b/.vscode/settings.json` sets both to `false`. No
 * other test configuration in this project ever opens more than one
 * workspace folder at once, so this is the only place the resource-scoped
 * `getConfiguration(section, uri)` branch (extension.ts's `isLintEnabled`,
 * formatter.ts's `isFormatEnabled`) and the per-document
 * `onDidChangeConfiguration` handler (extension.ts `activate`) actually get
 * exercised against two folders with genuinely different effective values at
 * the same time.
 */
suite("dat_linter VSCode extension: multi-root workspace per-folder lint.enable/format.enable", () => {
  suiteSetup(async function () {
    this.timeout(30000);
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} not found - is it registered under that id?`);
    await ext!.activate();

    const folders = vscode.workspace.workspaceFolders;
    assert.ok(
      folders && folders.length === 2,
      `expected 2 workspace folders to be open (see .vscode-test.mjs 'multi-root-workspace' configuration's workspaceFolder option), got: ${JSON.stringify(
        folders?.map((f) => f.uri.fsPath)
      )}`
    );
    const names = folders!.map((f) => path.basename(f.uri.fsPath)).sort();
    assert.deepStrictEqual(names, ["folder-a", "folder-b"]);
  });

  /**
   * Local copy of the identically-named helper used elsewhere in this
   * project's tests -- this suite intentionally runs in its own VSCode
   * instance/test config and doesn't share modules with test/extension.test.ts.
   */
  async function waitForDiagnostics(
    uri: vscode.Uri,
    predicate: (diags: readonly vscode.Diagnostic[]) => boolean,
    timeoutMs = 15000
  ): Promise<readonly vscode.Diagnostic[]> {
    const start = Date.now();
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const diags = vscode.languages.getDiagnostics(uri);
      if (predicate(diags)) {
        return diags;
      }
      if (Date.now() - start > timeoutMs) {
        throw new Error(
          `Timed out waiting for diagnostics on ${uri.fsPath}. Last seen: ${JSON.stringify(
            diags.map((d) => ({ code: d.code, message: d.message }))
          )}`
        );
      }
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }

  test("folder-a (lint.enable=true) gets diagnostics while folder-b (lint.enable=false) stays clean, and per-folder config resolves via the document's own Uri", async function () {
    this.timeout(30000);

    const uriA = vscode.Uri.file(path.join(FOLDER_A, "sample.dat"));
    const uriB = vscode.Uri.file(path.join(FOLDER_B, "sample.dat"));

    try {
      // Sanity check on folder-a first: this file DOES produce a
      // duplicate-key diagnostic when linting is enabled, so folder-b
      // staying clean below is a real negative, not linting never having
      // run for either document at all.
      const docA = await vscode.workspace.openTextDocument(uriA);
      await vscode.window.showTextDocument(docA, vscode.ViewColumn.One);
      const diagsA = await waitForDiagnostics(uriA, (d) => d.length > 0);
      assert.ok(
        diagsA.some((d) => d.code === "duplicate-key"),
        `expected a duplicate-key diagnostic for folder-a/sample.dat (lint.enable=true there), got: ${JSON.stringify(
          diagsA.map((d) => ({ code: d.code, message: d.message }))
        )}`
      );

      // folder-b has the identical duplicate-key content, but
      // folder-b/.vscode/settings.json sets lint.enable=false. If
      // isLintEnabled() ignored the document's Uri (the bug gemini-code-assist
      // flagged) it would resolve whichever folder's value VSCode happens to
      // return for an un-scoped lookup -- possibly folder-a's `true` -- and
      // this document would incorrectly get linted too.
      const docB = await vscode.workspace.openTextDocument(uriB);
      await vscode.window.showTextDocument(docB, vscode.ViewColumn.Two);
      await docB.save();
      // Give a real lint attempt a chance to run and (incorrectly) populate
      // diagnostics if the per-folder scoping were broken, then assert it
      // didn't.
      await new Promise((resolve) => setTimeout(resolve, 2000));
      assert.deepStrictEqual(
        vscode.languages.getDiagnostics(uriB),
        [],
        "expected no diagnostics for folder-b/sample.dat (lint.enable=false there)"
      );
    } finally {
      try {
        await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      } catch {
        // best-effort cleanup
      }
    }
  });

  test("folder-a (format.enable=true) formats on Format Document while folder-b (format.enable=false) is a no-op", async function () {
    this.timeout(30000);

    const uriA = vscode.Uri.file(path.join(FOLDER_A, "fmt_sample.dat"));
    const uriB = vscode.Uri.file(path.join(FOLDER_B, "fmt_sample.dat"));

    try {
      const docA = await vscode.workspace.openTextDocument(uriA);
      await vscode.window.showTextDocument(docA, vscode.ViewColumn.One);
      const beforeA = docA.getText();
      await vscode.commands.executeCommand("editor.action.formatDocument");
      assert.notStrictEqual(
        docA.getText(),
        beforeA,
        "expected folder-a/fmt_sample.dat to be reformatted (format.enable=true there)"
      );
      // Discard the in-memory formatting edit so this fixture file stays
      // byte-identical on disk for the next test run (the formatting
      // provider returns edits without writing to disk; nothing else here
      // ever saves docA).
      await vscode.commands.executeCommand("workbench.action.files.revert");

      const docB = await vscode.workspace.openTextDocument(uriB);
      await vscode.window.showTextDocument(docB, vscode.ViewColumn.One);
      const beforeB = docB.getText();
      await vscode.commands.executeCommand("editor.action.formatDocument");
      assert.strictEqual(
        docB.getText(),
        beforeB,
        "expected folder-b/fmt_sample.dat to be left untouched (format.enable=false there)"
      );
      assert.strictEqual(docB.isDirty, false);
    } finally {
      try {
        await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      } catch {
        // best-effort cleanup
      }
    }
  });
});
