import * as vscode from "vscode";
import * as path from "path";
import { parseDatLinterJson, toVscodeDiagnostics } from "./parser";
import { resolveExecutionContext, runDatLinter, VersionIncompatibilityHint, withTempDatFile } from "./runner";
import { registerDatFormattingEditProvider } from "./formatter";

let diagnosticCollection: vscode.DiagnosticCollection;

export function activate(context: vscode.ExtensionContext): void {
  diagnosticCollection = vscode.languages.createDiagnosticCollection("dat-linter");
  context.subscriptions.push(diagnosticCollection);

  const maybeLint = (document: vscode.TextDocument) => {
    if (!isDatFile(document)) {
      return;
    }
    void lintDocument(document);
  };

  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(maybeLint),
    vscode.workspace.onDidOpenTextDocument(maybeLint),
    vscode.workspace.onDidCloseTextDocument((doc) => {
      diagnosticCollection.delete(doc.uri);
    })
  );

  registerDatFormattingEditProvider(context);

  // Lint any .dat documents that were already open when the extension activated.
  vscode.workspace.textDocuments.forEach(maybeLint);
}

export function deactivate(): void {
  diagnosticCollection?.dispose();
}

function isDatFile(document: vscode.TextDocument): boolean {
  return (
    document.uri.scheme === "file" &&
    document.fileName.toLowerCase().endsWith(".dat")
  );
}

/**
 * dat_linter ran but rejected `--format json` — heuristically detected from
 * clap's error phrasing (see runner.ts `describeFailure` / cli.rs), the
 * signature of a dat_linter build older than the one this extension
 * requires (--format json support was added in dat_linter 0.1.2).
 *
 * Exported so `test/runner.test.ts` can exercise the regex directly as a
 * pure function, without needing a real dat_linter binary.
 */
export const LINT_FORMAT_JSON_VERSION_HINT: VersionIncompatibilityHint = {
  test: (stderr: string): boolean => /--format|unexpected argument|unrecognized/i.test(stderr),
  message:
    "dat_linter did not accept --format json. Your dat_linter version may be older than " +
    "the one this extension requires (--format json support was added in dat_linter 0.1.2). " +
    "Please update dat_linter to the latest release.",
};

/**
 * Lints `document`'s in-memory buffer, not whatever is currently saved on
 * disk. `dat_linter lint` has no stdin mode -- it only accepts a file path --
 * so, mirroring `formatter.ts`'s `provideDocumentFormattingEdits`, the
 * current buffer content is written to a throwaway temp file and that temp
 * file (not `document.uri.fsPath`) is what gets passed to `dat_linter`.
 *
 * This currently only fires from `onDidSaveTextDocument`/`onDidOpenTextDocument`
 * (see `activate` below), where buffer and disk content are always identical,
 * so the bug this guards against is latent today -- but see CLAUDE.md /
 * docs/assurance-ledger.md: adding a real-time lint trigger
 * (`onDidChangeTextDocument`) in the future would otherwise silently resurrect
 * the same stale-disk-content bug `formatter.ts` once had.
 *
 * IMPORTANT: the temp file is created *next to* the original document (its
 * own directory), not in `os.tmpdir()` like `formatter.ts`'s temp file.
 * `dat_linter lint` resolves `icon=`/`frontimage[...]=` image references
 * relative to `dat_dir`, which it derives from the parent directory of the
 * path it was given (see `src/commands/lint.rs` `dat_dir = path.parent()`).
 * Placing the temp file in `os.tmpdir()` instead would silently change every
 * image reference's resolution base directory, turning previously-valid
 * `icon=`/image references into spurious `missing-image-file` diagnostics.
 */
async function lintDocument(document: vscode.TextDocument): Promise<void> {
  const filePath = document.uri.fsPath;
  const { executablePath, configPath, cwd } = resolveExecutionContext(document);

  try {
    const stdout = await withTempDatFile(
      document.getText(),
      path.dirname(filePath),
      `${path.basename(filePath)}.dat-linter-tmp`,
      async (tempFilePath) => {
        const args = ["lint", tempFilePath, "--format", "json"];
        if (configPath) {
          args.push("--config", configPath);
        }
        return runDatLinter(executablePath, args, cwd, LINT_FORMAT_JSON_VERSION_HINT);
      }
    );
    const parsed = parseDatLinterJson(stdout);
    const diagnostics = toVscodeDiagnostics(parsed, document);
    diagnosticCollection.set(document.uri, diagnostics);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`dat_linter: ${message}`);
  }
}
