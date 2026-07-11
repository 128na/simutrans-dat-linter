import * as vscode from "vscode";
import { parseDatLinterJson, toVscodeDiagnostics } from "./parser";
import { resolveExecutionContext, runDatLinter, VersionIncompatibilityHint } from "./runner";
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
 */
const LINT_FORMAT_JSON_VERSION_HINT: VersionIncompatibilityHint = {
  test: (stderr: string): boolean => /--format|unexpected argument|unrecognized/i.test(stderr),
  message:
    "dat_linter did not accept --format json. Your dat_linter version may be older than " +
    "the one this extension requires (--format json support was added in dat_linter 0.1.2). " +
    "Please update dat_linter to the latest release.",
};

async function lintDocument(document: vscode.TextDocument): Promise<void> {
  const filePath = document.uri.fsPath;
  const { executablePath, configPath, cwd } = resolveExecutionContext(document);

  const args = ["lint", filePath, "--format", "json"];
  if (configPath) {
    args.push("--config", configPath);
  }

  try {
    const stdout = await runDatLinter(executablePath, args, cwd, LINT_FORMAT_JSON_VERSION_HINT);
    const parsed = parseDatLinterJson(stdout);
    const diagnostics = toVscodeDiagnostics(parsed, document);
    diagnosticCollection.set(document.uri, diagnostics);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`dat_linter: ${message}`);
  }
}
