import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

// package.json publisher "128na" + name "simutrans-dat-linter"
const EXTENSION_ID = "128na.simutrans-dat-linter";

// out/test/workspace -> out/test -> out -> vscode (this extension's root)
const VSCODE_ROOT = path.resolve(__dirname, "..", "..", "..");
const WORKSPACE_ROOT_FIXTURE = path.join(VSCODE_ROOT, "fixtures", "workspace-root");
const SAMPLE_DAT = path.join(WORKSPACE_ROOT_FIXTURE, "nested", "sample.dat");

/**
 * This suite runs in its own `.vscode-test.mjs` test configuration (see
 * `workspaceFolder` there), which launches a VSCode instance with
 * `fixtures/workspace-root` opened as the workspace folder -- something none
 * of `test/extension.test.ts`'s tests do (they all run with no workspace
 * folder open at all, and additionally override `configPath` explicitly in
 * `suiteSetup`, which bypasses cwd-based config auto-discovery entirely).
 *
 * That means `resolveExecutionContext`'s `vscode.workspace.getWorkspaceFolder`
 * branch (runner.ts) has never actually executed in any prior test run --
 * every existing test happened to fall back to the file's-own-directory cwd,
 * which for a lint/fmt invocation with a real `--config` passed explicitly
 * produces identical output either way, silently hiding the fact that the
 * workspace-folder-root branch itself was never exercised.
 *
 * This test deliberately leaves `simutransDatLinter.configPath` at its
 * default (unset) and instead relies on `fixtures/workspace-root/dat_linter.toml`
 * being auto-discovered via cwd. `nested/sample.dat` sits one directory
 * *below* that config file, so the two possible cwd resolutions produce
 * observably different results:
 *   - cwd = workspace folder root (`fixtures/workspace-root`, correct):
 *     dat_linter finds `dat_linter.toml` there, which excludes
 *     "duplicate-key" -> no duplicate-key diagnostic is reported.
 *   - cwd = the file's own directory (`fixtures/workspace-root/nested`,
 *     the pre-workspace-folder fallback): no config is found there, so
 *     dat_linter falls back to its all-rules-enabled default -> a
 *     duplicate-key diagnostic *is* reported.
 * Verified directly against the CLI while writing this test (see this
 * commit's PR description / assurance-ledger.md entry): running
 * `dat_linter lint nested/sample.dat --format json` with cwd set to
 * `fixtures/workspace-root` yields `"warning_count":0`, while running it
 * with cwd set to `fixtures/workspace-root/nested` yields
 * `"warning_count":1` (the duplicate-key warning). So asserting the
 * diagnostic's absence here is a real differential test of the cwd
 * resolution branch, not a tautology.
 */
suite("dat_linter VSCode extension: workspace folder root cwd auto-discovery", () => {
  let originalConfigPath: string | undefined;

  suiteSetup(async function () {
    this.timeout(30000);
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} not found - is it registered under that id?`);
    await ext!.activate();

    // Sanity check that this test really is running with a workspace folder
    // open, and that it's the fixture we expect -- if this ever fails, the
    // whole premise of the test (exercising the workspace-folder cwd branch)
    // is void, so fail loudly rather than let the rest of the suite pass
    // vacuously.
    const folders = vscode.workspace.workspaceFolders;
    assert.ok(
      folders && folders.length > 0,
      "expected a workspace folder to be open (see .vscode-test.mjs 'workspace-cwd' configuration's workspaceFolder option)"
    );
    assert.strictEqual(
      path.normalize(folders![0].uri.fsPath).toLowerCase(),
      path.normalize(WORKSPACE_ROOT_FIXTURE).toLowerCase(),
      `expected the open workspace folder to be ${WORKSPACE_ROOT_FIXTURE}, got ${folders![0].uri.fsPath}`
    );

    // Deliberately force simutransDatLinter.configPath back to its default
    // "" (rather than merely asserting it's already unset) -- the "default"
    // test configuration's suiteSetup (test/extension.test.ts) sets this to
    // FIXTURE_CONFIG at Global scope, and @vscode/test-cli's two
    // configurations share the same `.vscode-test/user-data` profile, so
    // that Global write leaks into this configuration's run too when both
    // run back-to-back via `npm test`. Leaving it unset is the whole point
    // of this test: it forces resolveExecutionContext's cwd to be what
    // actually determines which dat_linter.toml (if any) gets picked up.
    originalConfigPath = vscode.workspace.getConfiguration("simutransDatLinter").get<string>("configPath");
    await vscode.workspace
      .getConfiguration("simutransDatLinter")
      .update("configPath", "", vscode.ConfigurationTarget.Global);
    // Re-fetch rather than reusing the WorkspaceConfiguration instance from
    // before the update -- it's a point-in-time snapshot and isn't
    // guaranteed to reflect a write made through its own .update() call.
    const configAfterReset = vscode.workspace.getConfiguration("simutransDatLinter");
    assert.strictEqual(
      configAfterReset.get<string>("configPath", ""),
      "",
      "expected simutransDatLinter.configPath to be unset for this test"
    );
  });

  suiteTeardown(async () => {
    // Restore whatever configPath was set before this suite forced it to ""
    // at Global scope, so this suite doesn't leak a changed setting into
    // whichever test configuration/session runs next (see the suiteSetup
    // comment above for why this write happens at Global scope in the first
    // place).
    await vscode.workspace
      .getConfiguration("simutransDatLinter")
      .update("configPath", originalConfigPath, vscode.ConfigurationTarget.Global);
  });

  /**
   * Polls vscode.languages.getDiagnostics until predicate is satisfied, since
   * diagnosticCollection.set() happens asynchronously after dat_linter's
   * child process exits. (Local copy of the identically-named helper in
   * test/extension.test.ts -- this suite intentionally runs in its own
   * VSCode instance/test config and doesn't share modules with that file.)
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

  test("opening nested/sample.dat picks up fixtures/workspace-root/dat_linter.toml via workspace folder cwd, not the file's own directory", async () => {
    const uri = vscode.Uri.file(SAMPLE_DAT);
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document);

    // Positive control: the file DOES get linted and DOES produce
    // diagnostics (the missing-image-file ones), so an empty result here
    // wouldn't be silent success -- it'd be a red flag the pipeline broke.
    const diags = await waitForDiagnostics(uri, (d) => d.length > 0);
    assert.ok(
      diags.some((d) => d.code === "missing-image-file"),
      `expected at least one missing-image-file diagnostic (sanity check that linting actually ran), got: ${JSON.stringify(
        diags.map((d) => ({ code: d.code, message: d.message }))
      )}`
    );

    // The actual assertion: no duplicate-key diagnostic, because
    // fixtures/workspace-root/dat_linter.toml (found via the workspace
    // folder root cwd) excludes it. If resolveExecutionContext instead fell
    // back to the file's own directory (nested/, which has no config), this
    // diagnostic WOULD be present -- see the suite doc comment above for the
    // CLI-verified before/after.
    assert.ok(
      !diags.some((d) => d.code === "duplicate-key"),
      `expected no duplicate-key diagnostic (fixtures/workspace-root/dat_linter.toml excludes it), got: ${JSON.stringify(
        diags.map((d) => ({ code: d.code, message: d.message }))
      )}`
    );
  });
});
