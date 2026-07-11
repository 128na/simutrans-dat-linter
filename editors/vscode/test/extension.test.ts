import * as assert from "assert";
import * as path from "path";
import * as os from "os";
import * as fs from "fs/promises";
import * as vscode from "vscode";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

// package.json publisher "128na" + name "simutrans-dat-linter"
const EXTENSION_ID = "128na.simutrans-dat-linter";

// out/test -> out -> vscode -> editors -> simutrans-dat-linter (this repo's root)
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");
const TESTDATA_DIR = path.join(REPO_ROOT, "testdata");
// out/test -> out -> vscode, then fixtures/test-lint-config.toml
const FIXTURE_CONFIG = path.resolve(__dirname, "..", "..", "fixtures", "test-lint-config.toml");

/**
 * Polls vscode.languages.getDiagnostics until predicate is satisfied, since
 * diagnosticCollection.set() happens asynchronously after dat_linter's child
 * process exits.
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

suite("dat_linter VSCode extension integration", () => {
  suiteSetup(async function () {
    this.timeout(30000);
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} not found - is it registered under that id?`);
    await ext!.activate();

    // Point the extension at a throwaway config so dat_linter's rule include/exclude
    // and language settings are deterministic for this test run (dat_linter no longer
    // auto-generates a dat_linter.toml on its own; see `dat_linter init`).
    const config = vscode.workspace.getConfiguration("simutransDatLinter");
    await config.update(
      "configPath",
      FIXTURE_CONFIG,
      vscode.ConfigurationTarget.Global
    );
  });

  test("duplicate_key.dat produces a duplicate-key Warning on the correct 0-indexed line", async () => {
    const filePath = path.join(TESTDATA_DIR, "duplicate_key.dat");
    const uri = vscode.Uri.file(filePath);
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document);

    const diags = await waitForDiagnostics(uri, (d) => d.length > 0);

    const dup = diags.find((d) => d.code === "duplicate-key");
    assert.ok(
      dup,
      `expected a duplicate-key diagnostic, got: ${JSON.stringify(
        diags.map((d) => ({ code: d.code, severity: d.severity, message: d.message }))
      )}`
    );
    assert.strictEqual(dup!.severity, vscode.DiagnosticSeverity.Warning);
    // dat_linter's JSON output has "line": 3 (1-indexed); VSCode ranges are
    // 0-indexed, so the diagnostic must land on line 2.
    assert.strictEqual(dup!.range.start.line, 2);
    assert.strictEqual(dup!.source, "dat_linter");
  });

  test("broken_missing_waytype.dat produces a missing-waytype Error falling back to line 0", async () => {
    const filePath = path.join(TESTDATA_DIR, "broken_missing_waytype.dat");
    const uri = vscode.Uri.file(filePath);
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document);

    const diags = await waitForDiagnostics(uri, (d) => d.length > 0);

    const missing = diags.find((d) => d.code === "missing-waytype");
    assert.ok(
      missing,
      `expected a missing-waytype diagnostic, got: ${JSON.stringify(
        diags.map((d) => ({ code: d.code, severity: d.severity, message: d.message }))
      )}`
    );
    assert.strictEqual(missing!.severity, vscode.DiagnosticSeverity.Error);
    // dat_linter's JSON output has "line": null for this diagnostic (it's
    // file-wide), so the extension must fall back to line 0.
    assert.strictEqual(missing!.range.start.line, 0);
  });

  test("Format Document on fmt_example.dat matches `dat_linter fmt` CLI output", async () => {
    const filePath = path.join(TESTDATA_DIR, "fmt_example.dat");
    const uri = vscode.Uri.file(filePath);
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document);

    // Note: our provider (src/formatter.ts) returns a single whole-document
    // TextEdit, but VSCode's "vscode.executeFormatDocumentProvider" command
    // (and "editor.action.formatDocument") internally diffs that against the
    // original content and applies a set of minimal, non-overlapping edits
    // instead — so we drive the real formatting command and read back the
    // resulting document text, rather than asserting on edit shape/count.
    await vscode.commands.executeCommand("editor.action.formatDocument");

    // Independently invoke the same `dat_linter fmt` CLI command the extension
    // runs, to get the ground-truth expected output without going through the
    // extension's own code.
    const { stdout: expected } = await execFileAsync("dat_linter", [
      "fmt",
      filePath,
      "--config",
      FIXTURE_CONFIG,
    ]);

    assert.strictEqual(document.getText(), expected);
  });

  test("Format Document reflects unsaved buffer edits rather than stale disk content", async () => {
    // Regression test for a real data-loss bug: the formatter used to invoke
    // `dat_linter fmt <document.uri.fsPath>`, which reads whatever is
    // currently *saved on disk* -- ignoring any unsaved edits in the VSCode
    // buffer. Formatting then replaced the whole buffer with the formatted
    // result of the *stale disk content*, discarding the user's in-progress
    // (unsaved) edits; if this ran via `editor.formatOnSave`, that
    // discarded state is exactly what then got written to disk.
    //
    // The previous formatter test above doesn't catch this: it formats a
    // document immediately after opening it, before the buffer ever diverges
    // from disk, so buffer content and disk content are always identical and
    // the bug is invisible to it.
    //
    // This test opens a throwaway *copy* of testdata/fmt_example.dat (never
    // the tracked fixture itself -- see CLAUDE.md guidance not to mutate
    // testdata/), edits the in-memory buffer without saving, and asserts the
    // formatted result reflects the edited buffer, not the untouched copy
    // still sitting on disk.
    const original = await fs.readFile(path.join(TESTDATA_DIR, "fmt_example.dat"), "utf8");

    const uniqueSuffix = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const scratchFilePath = path.join(os.tmpdir(), `dat-linter-test-unsaved-edit-${uniqueSuffix}.dat`);
    await fs.writeFile(scratchFilePath, original, "utf8");

    try {
      const uri = vscode.Uri.file(scratchFilePath);
      const document = await vscode.workspace.openTextDocument(uri);
      const editor = await vscode.window.showTextDocument(document);

      // Edit the buffer WITHOUT saving: change the copyright line's value to
      // a marker value. This diverges the in-memory buffer from what's still
      // on disk (the untouched copy of the original fixture content).
      const marker = "unsaved_edit_marker";
      const copyrightLineIndex = (() => {
        for (let i = 0; i < document.lineCount; i++) {
          if (document.lineAt(i).text.startsWith("copyright=")) {
            return i;
          }
        }
        throw new Error(`could not find a "copyright=" line in ${scratchFilePath}`);
      })();
      await editor.edit((editBuilder) => {
        editBuilder.replace(document.lineAt(copyrightLineIndex).range, `copyright=${marker}`);
      });
      assert.ok(document.isDirty, "document should have unsaved changes before formatting");

      // Ground truth: run the CLI directly against the *edited* buffer
      // content (written to its own independent scratch file), so the
      // expected value is derived without going through the extension at all.
      const expectedInputPath = path.join(os.tmpdir(), `dat-linter-test-expected-${uniqueSuffix}.dat`);
      await fs.writeFile(expectedInputPath, document.getText(), "utf8");
      let expected: string;
      try {
        const result = await execFileAsync("dat_linter", [
          "fmt",
          expectedInputPath,
          "--config",
          FIXTURE_CONFIG,
        ]);
        expected = result.stdout;
      } finally {
        await fs.unlink(expectedInputPath).catch(() => undefined);
      }
      // Sanity check that the marker actually survives `dat_linter fmt`
      // (i.e. this isn't accidentally a no-op comparison).
      assert.ok(expected.includes(marker), `expected ground-truth fmt output to contain "${marker}": ${expected}`);

      await vscode.commands.executeCommand("editor.action.formatDocument");

      assert.ok(
        document.getText().includes(marker),
        `expected formatted document to retain the unsaved edit marker "${marker}" instead of ` +
          `reverting to stale disk content, got: ${document.getText()}`
      );
      assert.strictEqual(document.getText(), expected);
    } finally {
      // Never persist this scratch document: revert discards the dirty
      // in-memory buffer (back to the untouched copy still on disk) before
      // closing, so nothing is written to disk, then clean up the scratch
      // file itself.
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
      await fs.unlink(scratchFilePath).catch(() => undefined);
    }
  });
});
