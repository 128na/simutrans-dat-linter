import * as vscode from "vscode";
import { execFile, ExecFileException } from "child_process";
import * as path from "path";
import { parseDatLinterJson, toVscodeDiagnostics } from "./parser";

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

async function lintDocument(document: vscode.TextDocument): Promise<void> {
  const filePath = document.uri.fsPath;
  // Prefer the workspace folder root as cwd (matches how dat_linter resolves a
  // default dat_linter.toml when configPath is unset); fall back to the file's
  // own directory when no workspace folder contains it (e.g. a single file
  // opened without a workspace).
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
  const cwd = workspaceFolder ? workspaceFolder.uri.fsPath : path.dirname(filePath);

  const config = vscode.workspace.getConfiguration("simutransDatLinter");
  const executablePath = config.get<string>("executablePath", "dat_linter");
  const configPath = config.get<string>("configPath", "");

  const args = ["lint", filePath, "--format", "json"];
  if (configPath) {
    args.push("--config", configPath);
  }

  try {
    const stdout = await runDatLinter(executablePath, args, cwd);
    const parsed = parseDatLinterJson(stdout);
    const diagnostics = toVscodeDiagnostics(parsed, document);
    diagnosticCollection.set(document.uri, diagnostics);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`dat_linter: ${message}`);
  }
}

/**
 * Runs dat_linter and resolves with stdout only. dat_linter's exit code
 * reflects whether error/warning-level diagnostics were found, NOT whether
 * the process itself failed to run — so a non-zero exit with non-empty
 * stdout is treated as success here (stdout always carries a full JSON
 * payload, even for a clean file: `{"files":[{"diagnostics":[]}],...}`).
 * Only a genuine failure to run the tool at all (spawn failure, or a clap
 * argument-parsing error that produces no stdout) is rejected.
 */
function runDatLinter(executablePath: string, args: string[], cwd: string): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(executablePath, args, { cwd }, (error, stdout, stderr) => {
      if (stdout && stdout.trim().length > 0) {
        resolve(stdout);
        return;
      }
      reject(describeFailure(error, stderr, executablePath));
    });
  });
}

/**
 * Turns a failed dat_linter invocation into a message that distinguishes
 * "the executable itself couldn't be found/run" from "it ran but rejected
 * --format json", the latter being the signature of a dat_linter build that
 * predates --format json support. This is a best-effort heuristic based on
 * clap's error phrasing (see cli.rs), not a guaranteed classification.
 */
function describeFailure(
  error: ExecFileException | null,
  stderr: string,
  executablePath: string
): Error {
  if (error && (error.code === "ENOENT" || /not recognized|not found/i.test(error.message))) {
    return new Error(
      `failed to run "${executablePath}" (executable not found). ` +
        'Check the "simutransDatLinter.executablePath" setting, or install dat_linter and add it to PATH.'
    );
  }

  if (stderr && /--format|unexpected argument|unrecognized/i.test(stderr)) {
    return new Error(
      "dat_linter did not accept --format json. Your dat_linter version may be older than " +
        "the one this extension requires (--format json support was added in dat_linter 0.1.2). " +
        "Please update dat_linter to the latest release."
    );
  }

  const detail = stderr && stderr.trim().length > 0 ? stderr.trim() : error?.message ?? "no output";
  return new Error(`failed to run (${detail})`);
}
