import * as vscode from "vscode";
import { resolveExecutionContext, runDatLinter, VersionIncompatibilityHint } from "./runner";

/**
 * `dat_linter fmt` ran but rejected the `fmt` subcommand or the `--config`
 * flag this extension passes — heuristically detected from clap's error
 * phrasing (see runner.ts `describeFailure` / cli.rs), the signature of a
 * dat_linter build too old to support this extension's formatter.
 */
const FMT_VERSION_HINT: VersionIncompatibilityHint = {
  test: (stderr: string): boolean => /unrecognized subcommand|unexpected argument|unrecognized/i.test(stderr),
  message:
    "dat_linter did not accept the fmt subcommand or its arguments. Your dat_linter version may be " +
    "too old to support this. Please update dat_linter to the latest release.",
};

/**
 * Document formatting provider backed by `dat_linter fmt <path> [--config ...]`.
 *
 * Deliberately run WITHOUT `-w`/`--write`: dat_linter fmt's default (dry-run)
 * mode prints the formatted result to stdout only, leaving the file on disk
 * untouched. This provider takes that stdout and returns it to VSCode as a
 * single whole-document TextEdit, so VSCode applies the change to its own
 * in-memory buffer instead of two processes racing to write the same file.
 *
 * CRLF/LF handling: `dat_linter fmt` detects and preserves the input file's
 * line-ending style (fixed in dat_linter commit 095d663), so no additional
 * normalization is needed here.
 */
export async function provideDocumentFormattingEdits(
  document: vscode.TextDocument
): Promise<vscode.TextEdit[] | undefined> {
  const filePath = document.uri.fsPath;
  const { executablePath, configPath, cwd } = resolveExecutionContext(document);

  const args = ["fmt", filePath];
  if (configPath) {
    args.push("--config", configPath);
  }

  try {
    const stdout = await runDatLinter(executablePath, args, cwd, FMT_VERSION_HINT);
    const fullRange = new vscode.Range(
      document.positionAt(0),
      document.positionAt(document.getText().length)
    );
    return [vscode.TextEdit.replace(fullRange, stdout)];
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`dat_linter fmt: ${message}`);
    return undefined;
  }
}

export function registerDatFormattingEditProvider(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.languages.registerDocumentFormattingEditProvider(
      { pattern: "**/*.dat" },
      { provideDocumentFormattingEdits }
    )
  );
}
