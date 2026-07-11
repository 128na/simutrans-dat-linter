import * as vscode from "vscode";
import { execFile, ExecFileException } from "child_process";
import * as fs from "fs/promises";
import * as path from "path";

/**
 * Resolved inputs needed to invoke the `dat_linter` executable against a
 * given document: which binary to run, which `--config` (if any) to pass,
 * and which directory to run it from.
 *
 * Shared by both the `lint` and `fmt` code paths (see extension.ts and
 * formatter.ts) so the cwd/config resolution logic only lives in one place.
 * When `configPath` is unset, `dat_linter` auto-discovers `dat_linter.toml`
 * in cwd but no longer auto-generates it if missing (run `dat_linter init`
 * there to create one) -- it falls back to all-rules-enabled defaults.
 */
export interface DatLinterExecutionContext {
  executablePath: string;
  configPath: string;
  cwd: string;
}

/**
 * Resolves the executable path, config path, and working directory for a
 * `dat_linter` invocation against `document`.
 *
 * cwd prefers the document's workspace folder root (matching how
 * `dat_linter` resolves a default `dat_linter.toml` when configPath is
 * unset); falls back to the file's own directory when no workspace folder
 * contains it (e.g. a single file opened without a workspace).
 */
export function resolveExecutionContext(document: vscode.TextDocument): DatLinterExecutionContext {
  const filePath = document.uri.fsPath;
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
  const cwd = workspaceFolder ? workspaceFolder.uri.fsPath : path.dirname(filePath);

  const config = vscode.workspace.getConfiguration("simutransDatLinter");
  const executablePath = config.get<string>("executablePath", "dat_linter");
  const configPath = config.get<string>("configPath", "");

  return { executablePath, configPath, cwd };
}

/**
 * Writes `content` to a throwaway temp `.dat` file inside `dir`, invokes `fn`
 * with that file's path, and unconditionally deletes the file afterward
 * (best-effort -- a failed cleanup isn't surfaced as an error).
 *
 * Shared by both the `lint` (extension.ts) and `fmt` (formatter.ts) code
 * paths, which both need to run `dat_linter` against the *buffer* content
 * (`document.getText()`) rather than whatever is currently saved on disk --
 * see formatter.ts's `provideDocumentFormattingEdits` doc comment for why
 * that distinction matters.
 *
 * The two callers deliberately use different `dir`/`namePrefix` values and
 * are NOT unified further:
 *   - `fmt` has no cross-file path resolution concerns, so its temp file
 *     lives in `os.tmpdir()` (prefix `"dat-linter-fmt"`, matching the
 *     pre-existing naming this extension's tests already assert on).
 *   - `lint` resolves `icon=`/`frontimage[...]=` image references relative
 *     to `dat_dir`, which dat_linter (`src/commands/lint.rs`) derives from
 *     the *parent directory of the path it was given* -- so lint's temp file
 *     must live next to the original document (not in `os.tmpdir()`), or
 *     every image reference would suddenly resolve against the wrong
 *     directory and start failing with `missing-image-file`.
 */
export async function withTempDatFile<T>(
  content: string,
  dir: string,
  namePrefix: string,
  fn: (tempFilePath: string) => Promise<T>
): Promise<T> {
  const tempFilePath = path.join(
    dir,
    `${namePrefix}-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}.dat`
  );

  await fs.writeFile(tempFilePath, content, "utf8");
  try {
    return await fn(tempFilePath);
  } finally {
    try {
      await fs.unlink(tempFilePath);
    } catch {
      // ignore
    }
  }
}

/**
 * Runs `dat_linter` and resolves with stdout only. dat_linter's exit code
 * reflects command-specific success/failure semantics (e.g. `lint`'s exit
 * code reflects whether error/warning-level diagnostics were found), NOT
 * whether the process itself failed to run — so a non-zero exit with
 * non-empty stdout is treated as success here. Only a genuine failure to run
 * the tool at all (spawn failure, or a clap argument-parsing error that
 * produces no stdout) is rejected.
 */
export function runDatLinter(
  executablePath: string,
  args: string[],
  cwd: string,
  versionHint?: VersionIncompatibilityHint
): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(executablePath, args, { cwd }, (error, stdout, stderr) => {
      if (stdout && stdout.trim().length > 0) {
        resolve(stdout);
        return;
      }
      reject(describeFailure(error, stderr, executablePath, versionHint));
    });
  });
}

/**
 * A command-specific hint for recognizing the "ran, but this dat_linter
 * build is too old to support the arguments this extension passes" failure
 * mode, distinct from "executable not found". Matched against stderr; see
 * callers (extension.ts for `lint --format json`, formatter.ts for `fmt`)
 * for the concrete patterns/messages.
 */
export interface VersionIncompatibilityHint {
  test: (stderr: string) => boolean;
  message: string;
}

/**
 * Turns a failed `dat_linter` invocation into a message that distinguishes
 * "the executable itself couldn't be found/run" from "it ran but rejected
 * one of the arguments this extension passed", the latter being the
 * signature of a dat_linter build that predates the feature this extension
 * relies on. `versionHint` lets each caller supply its own stderr pattern
 * and message for that second case; this is a best-effort heuristic based on
 * clap's error phrasing (see cli.rs), not a guaranteed classification.
 */
export function describeFailure(
  error: ExecFileException | null,
  stderr: string,
  executablePath: string,
  versionHint?: VersionIncompatibilityHint
): Error {
  if (error && (error.code === "ENOENT" || /not recognized|not found/i.test(error.message))) {
    return new Error(
      `failed to run "${executablePath}" (executable not found). ` +
        'Check the "simutransDatLinter.executablePath" setting, or install dat_linter and add it to PATH.'
    );
  }

  if (versionHint && stderr && versionHint.test(stderr)) {
    return new Error(versionHint.message);
  }

  const detail = stderr && stderr.trim().length > 0 ? stderr.trim() : error?.message ?? "no output";
  return new Error(`failed to run (${detail})`);
}
