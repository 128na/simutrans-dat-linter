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
/**
 * A brand-new (or emptied-out) `.dat` buffer has no `obj=` definition yet,
 * which `dat_linter lint` treats as a batch-validation failure ("obj= は
 * 未対応です" / `DiagnosticCode::UnsupportedObjType`, see
 * `src/commands/lint.rs`'s `records.is_empty()` branch) -- correct behavior
 * for its CLI batch-linting use case (flagging files with no obj definition
 * at all), but not something we want to surface the instant a user creates a
 * fresh, empty file in the editor before they've typed anything.
 *
 * Skip invoking dat_linter entirely when the buffer is empty or
 * whitespace-only, and just clear any diagnostics left over from before
 * (e.g. the user select-all-deleted a previously-linted file's contents) so
 * stale diagnostics don't linger.
 */
async function lintDocument(document: vscode.TextDocument): Promise<void> {
  if (/^\s*$/.test(document.getText())) {
    diagnosticCollection.delete(document.uri);
    return;
  }

  const filePath = document.uri.fsPath;
  const { executablePath, configPath, cwd } = resolveExecutionContext(document);

  const lintArgsFor = (path: string): string[] => {
    const args = ["lint", path, "--format", "json"];
    if (configPath) {
      args.push("--config", configPath);
    }
    return args;
  };

  try {
    let stdout: string;
    try {
      stdout = await withTempDatFile(
        document.getText(),
        path.dirname(filePath),
        `${path.basename(filePath)}.dat-linter-tmp`,
        (tempFilePath) => runDatLinter(executablePath, lintArgsFor(tempFilePath), cwd, LINT_FORMAT_JSON_VERSION_HINT)
      );
    } catch (err) {
      // If writing the temp file itself failed (e.g. the document's directory
      // is read-only), fall back to linting the on-disk path directly rather
      // than giving up entirely -- this reintroduces the stale-disk-content
      // caveat this function otherwise avoids, but only for the rare case
      // where the buffer-accurate path isn't available at all, which is
      // strictly better than lint silently stopping working in that
      // directory. A failure from `runDatLinter` itself (dat_linter ran but
      // failed) is a plain `Error` with no `.code`, so re-throw those as-is.
      const code = (err as NodeJS.ErrnoException)?.code;
      if (code === "EACCES" || code === "EROFS" || code === "EPERM") {
        stdout = await runDatLinter(executablePath, lintArgsFor(filePath), cwd, LINT_FORMAT_JSON_VERSION_HINT);
      } else {
        throw err;
      }
    }
    const parsed = parseDatLinterJson(stdout);
    const diagnostics = toVscodeDiagnostics(parsed, document);
    diagnosticCollection.set(document.uri, diagnostics);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`dat_linter: ${message}`);
  }
}
