import { defineConfig } from "@vscode/test-cli";

export default defineConfig([
  {
    // Only direct children of out/test/ (not out/test/workspace/**), which
    // all run with no workspace folder open, matching every existing test's
    // assumption. Kept as a separate glob (rather than out/test/**/*.test.js)
    // specifically so it does NOT pick up the "workspace-cwd" config's test
    // file below -- that one requires an actual workspace folder to be open
    // to exercise resolveExecutionContext's workspace-folder cwd branch
    // (runner.ts), which this configuration cannot provide.
    label: "default",
    files: "out/test/*.test.js",
    mocha: {
      timeout: 30000,
    },
  },
  {
    // Exercises resolveExecutionContext's `vscode.workspace.getWorkspaceFolder`
    // branch (runner.ts), which needs a real workspace folder open to ever
    // return non-undefined -- something none of the "default" configuration's
    // tests provide. See test/workspace/workspace-cwd.test.ts for the actual
    // test and why fixtures/workspace-root's dat_linter.toml makes this a real
    // differential test rather than a tautology.
    label: "workspace-cwd",
    files: "out/test/workspace/*.test.js",
    workspaceFolder: "fixtures/workspace-root",
    mocha: {
      timeout: 30000,
    },
  },
  {
    // Exercises the resource-scoped `getConfiguration("simutransDatLinter", uri)`
    // lookups added to extension.ts's `isLintEnabled` and formatter.ts's
    // `isFormatEnabled` in response to the gemini-code-assist review on PR
    // #23, plus the per-document `onDidChangeConfiguration` handler in
    // extension.ts `activate`. None of the other configurations here ever
    // open more than one workspace folder, so none of them can tell the
    // difference between "reads the correct folder's setting" and "reads
    // whichever folder VSCode happens to hand back first" -- both look
    // identical with a single folder open. See
    // test/multi-root/multi-root.test.ts and
    // fixtures/multi-root/multi-root.code-workspace for the two-folder setup
    // (folder-a: lint.enable/format.enable=true, folder-b: both false) that
    // makes this a real differential test.
    label: "multi-root-workspace",
    files: "out/test/multi-root/*.test.js",
    workspaceFolder: "fixtures/multi-root/multi-root.code-workspace",
    mocha: {
      timeout: 30000,
    },
  },
]);
