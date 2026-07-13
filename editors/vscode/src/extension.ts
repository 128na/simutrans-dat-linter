import * as vscode from "vscode";
import * as path from "path";
import { parseDatLinterJson, toVscodeDiagnostics } from "./parser";
import { resolveExecutionContext, runDatLinter, VersionIncompatibilityHint, withTempDatFile } from "./runner";
import { registerDatFormattingEditProvider } from "./formatter";
import { registerDatCompletionItemProvider } from "./completion";

let diagnosticCollection: vscode.DiagnosticCollection;

/**
 * Tracks all in-flight lint generations, keyed by `document.uri.toString()`.
 *
 * `lintDocument` can be invoked multiple times for the same document without
 * any ordering guarantee between the invocations' completions -- e.g.
 * `onDidOpenTextDocument` and `onDidSaveTextDocument` firing close together,
 * or two rapid saves under `files.autoSave`. Each `dat_linter lint` child
 * process runs independently and may resolve out of call order, so without
 * this guard a slower, *earlier* call's (now-stale) diagnostics could
 * overwrite a faster, *later* call's fresher diagnostics, leaving the
 * Problems panel showing results for a buffer state the user has since
 * changed.
 *
 * Exported (as a standalone, pure class rather than inlined counters) so
 * test/runner.test.ts can exercise the generation bookkeeping directly,
 * without spawning a real dat_linter process or trying to force two real
 * child processes to resolve out of order.
 */
export class LintGenerationTracker {
  private readonly generations = new Map<string, number>();

  /** Starts a new lint attempt for `key`, returning its generation number. */
  begin(key: string): number {
    const next = (this.generations.get(key) ?? 0) + 1;
    this.generations.set(key, next);
    return next;
  }

  /** True if `generation` is no longer the most recently started one for `key`. */
  isStale(key: string, generation: number): boolean {
    return this.generations.get(key) !== generation;
  }

  /** Forgets `key` entirely, e.g. once its document has closed. */
  forget(key: string): void {
    this.generations.delete(key);
  }
}

const lintGenerations = new LintGenerationTracker();

/**
 * Gate on `vscode.workspace.isTrusted`, factored out as a pure boolean
 * function so test/runner.test.ts can exercise the decision directly. The
 * real guard is `package.json`'s `capabilities.untrustedWorkspaces.supported:
 * false`, which stops VSCode from calling `activate()` at all in an
 * untrusted workspace -- this is defense in depth in case that declaration
 * is ever removed or VSCode's enforcement changes, since `lintDocument` /
 * the formatter / completion all shell out to a user-configurable
 * executable path and are unsafe to run against an untrusted workspace's
 * `.vscode/settings.json`.
 */
export function shouldActivateInWorkspace(isTrusted: boolean): boolean {
  return isTrusted;
}

/**
 * Gate on the `simutransDatLinter.lint.enable` setting, factored out as a
 * pure boolean function -- mirroring `shouldActivateInWorkspace` above -- so
 * test/runner.test.ts can exercise the decision directly. Lets users who
 * don't have `dat_linter` installed and only want syntax
 * highlighting/snippets turn off diagnostics entirely, instead of getting an
 * error popup (`describeFailure` in runner.ts, surfaced via
 * `vscode.window.showErrorMessage`) every time a `.dat` file is opened or
 * saved.
 */
export function shouldLint(enabled: boolean): boolean {
  return enabled;
}

function isLintEnabled(): boolean {
  return shouldLint(vscode.workspace.getConfiguration("simutransDatLinter").get<boolean>("lint.enable", true));
}

export function activate(context: vscode.ExtensionContext): void {
  if (!shouldActivateInWorkspace(vscode.workspace.isTrusted)) {
    // Don't register any diagnostics/formatting/completion providers, and
    // don't run dat_linter at all, until the workspace is trusted. Offer to
    // pick things up once trust is granted without requiring a full restart.
    context.subscriptions.push(
      vscode.workspace.onDidGrantWorkspaceTrust(() => {
        void vscode.window
          .showInformationMessage(
            "Simutrans dat_linter: this workspace is now trusted. Reload the window to enable linting, formatting, and completion.",
            "Reload"
          )
          .then((selection) => {
            if (selection === "Reload") {
              void vscode.commands.executeCommand("workbench.action.reloadWindow");
            }
          });
      })
    );
    return;
  }

  diagnosticCollection = vscode.languages.createDiagnosticCollection("dat-linter");
  context.subscriptions.push(diagnosticCollection);

  const outputChannel = vscode.window.createOutputChannel("Simutrans dat_linter");
  context.subscriptions.push(outputChannel);

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
      lintGenerations.forget(doc.uri.toString());
    }),
    // Toggling `simutransDatLinter.lint.enable` at runtime must take effect
    // immediately, not just for the next open/save: flipping it to false
    // clears every currently-shown diagnostic right away (instead of leaving
    // stale ones visible until the next save), and flipping it back to true
    // re-lints every open .dat document, mirroring the "lint anything already
    // open at activation" pass below.
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (!e.affectsConfiguration("simutransDatLinter.lint.enable")) {
        return;
      }
      if (isLintEnabled()) {
        vscode.workspace.textDocuments.forEach(maybeLint);
      } else {
        diagnosticCollection.clear();
      }
    })
  );

  registerDatFormattingEditProvider(context);
  registerDatCompletionItemProvider(context, outputChannel);

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
 *
 * Also guards against the race described on `LintGenerationTracker` above:
 * this call's generation number is recorded at entry, and every place that
 * would mutate `diagnosticCollection` first checks whether a *later* call for
 * the same document has started in the meantime -- if so, this call's result
 * is stale and is discarded instead of overwriting the newer one.
 */
async function lintDocument(document: vscode.TextDocument): Promise<void> {
  if (!isLintEnabled()) {
    // Don't touch diagnosticCollection here: the onDidChangeConfiguration
    // handler in activate() is what clears any pre-existing diagnostics the
    // instant the setting flips to false. A no-op here just means a
    // subsequent open/save while disabled doesn't produce new ones.
    return;
  }

  const key = document.uri.toString();
  const generation = lintGenerations.begin(key);

  if (/^\s*$/.test(document.getText())) {
    if (!lintGenerations.isStale(key, generation)) {
      diagnosticCollection.delete(document.uri);
    }
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
    if (lintGenerations.isStale(key, generation)) {
      // A newer lintDocument() call was issued while this one was in flight;
      // discard this now-stale result rather than clobbering the newer one.
      // Checked before parsing so a stale response's JSON isn't parsed and
      // converted to diagnostics for nothing.
      return;
    }
    const parsed = parseDatLinterJson(stdout);
    const diagnostics = toVscodeDiagnostics(parsed, document);
    diagnosticCollection.set(document.uri, diagnostics);
  } catch (err) {
    if (lintGenerations.isStale(key, generation)) {
      return;
    }
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`dat_linter: ${message}`);
  }
}
