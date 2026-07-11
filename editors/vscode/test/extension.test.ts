import * as assert from "assert";
import * as path from "path";
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

    // Point the extension at a throwaway config so dat_linter never falls back to
    // auto-generating dat_linter.toml next to the (read-only, ref-only) testdata files.
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
});
