import * as vscode from "vscode";
import * as fs from "fs/promises";
import * as os from "os";
import * as path from "path";
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
 *
 * IMPORTANT: `dat_linter fmt` has no stdin mode -- it only accepts a file
 * path (see `dat_linter fmt --help`). Passing `document.uri.fsPath` directly
 * would make it read whatever is currently saved on disk, which is *not*
 * necessarily what's in the editor buffer (`document.getText()`). If the
 * buffer has unsaved edits, formatting the on-disk content and replacing the
 * buffer with that result would silently discard those edits -- and if this
 * fires from `editor.formatOnSave`, the discarded state is what then gets
 * written to disk. To avoid this, the current buffer content is written to a
 * throwaway temp file and that temp file (not the real document path) is
 * what gets passed to `dat_linter fmt`.
 */
export async function provideDocumentFormattingEdits(
  document: vscode.TextDocument
): Promise<vscode.TextEdit[] | undefined> {
  const { executablePath, configPath, cwd } = resolveExecutionContext(document);

  const tempFilePath = path.join(
    os.tmpdir(),
    `dat-linter-fmt-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}.dat`
  );

  try {
    await fs.writeFile(tempFilePath, document.getText(), "utf8");

    const args = ["fmt", tempFilePath];
    if (configPath) {
      args.push("--config", configPath);
    }

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
  } finally {
    // Best-effort cleanup: failing to delete the temp file isn't worth
    // surfacing as an error to the user.
    try {
      await fs.unlink(tempFilePath);
    } catch {
      // ignore
    }
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
