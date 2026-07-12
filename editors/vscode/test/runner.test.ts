import * as assert from "assert";
import { ExecFileException } from "child_process";
import { describeFailure } from "../src/runner";
import { LINT_FORMAT_JSON_VERSION_HINT, shouldActivateInWorkspace } from "../src/extension";
import { FMT_VERSION_HINT } from "../src/formatter";

// Pure-function tests: no dat_linter binary, no extension activation, no
// vscode workspace/document APIs. Mirrors test/parser.test.ts's pattern for
// exercising logic that doesn't need any of that machinery.

/** Builds a minimal ExecFileException for a given override shape. */
function makeExecError(overrides: Partial<ExecFileException> & { message?: string }): ExecFileException {
  const err = new Error(overrides.message ?? "exec failed") as ExecFileException;
  Object.assign(err, overrides);
  return err;
}

suite("runner: describeFailure (pure function)", () => {
  test("reports executable-not-found for an ENOENT spawn error", () => {
    const err = makeExecError({ code: "ENOENT" });
    const result = describeFailure(err, "", "dat_linter_missing");
    assert.match(result.message, /executable not found/);
    assert.match(result.message, /dat_linter_missing/);
    assert.match(result.message, /executablePath/);
  });

  test("reports executable-not-found when the OS message says 'not recognized' even without an ENOENT code", () => {
    // Windows' cmd.exe-style spawn failure: no `code` at all, but a message
    // shape describeFailure special-cases via its `/not recognized|not found/i` check.
    const err = makeExecError({
      message: "'dat_linter' is not recognized as an internal or external command",
    });
    const result = describeFailure(err, "", "dat_linter");
    assert.match(result.message, /executable not found/);
  });

  test("returns the versionHint's message when stderr matches the hint's test", () => {
    const hint = {
      test: (stderr: string) => stderr.includes("BOOM"),
      message: "custom version incompatibility message",
    };
    const result = describeFailure(null, "some BOOM stderr from clap", "dat_linter", hint);
    assert.strictEqual(result.message, "custom version incompatibility message");
  });

  test("does not use the versionHint's message when stderr doesn't match its test", () => {
    const hint = {
      test: (stderr: string) => stderr.includes("BOOM"),
      message: "custom version incompatibility message",
    };
    const result = describeFailure(null, "totally unrelated stderr", "dat_linter", hint);
    assert.notStrictEqual(result.message, "custom version incompatibility message");
    assert.match(result.message, /failed to run/);
  });

  test("falls back to a generic message containing stderr detail when there's no versionHint", () => {
    const result = describeFailure(null, "some unrelated stderr", "dat_linter");
    assert.match(result.message, /failed to run/);
    assert.match(result.message, /some unrelated stderr/);
  });

  test("falls back to error.message when stderr is empty and there's no versionHint", () => {
    const err = makeExecError({ message: "spawn EACCES" });
    const result = describeFailure(err, "", "dat_linter");
    assert.match(result.message, /spawn EACCES/);
  });

  test("falls back to 'no output' when both stderr and error are absent", () => {
    const result = describeFailure(null, "", "dat_linter");
    assert.match(result.message, /no output/);
  });
});

suite("extension.ts: LINT_FORMAT_JSON_VERSION_HINT.test (pure regex)", () => {
  test("matches clap's 'unexpected argument' phrasing for an unrecognized --format flag", () => {
    assert.strictEqual(
      LINT_FORMAT_JSON_VERSION_HINT.test("error: unexpected argument '--format' found"),
      true
    );
  });

  test("matches clap's 'unrecognized' phrasing", () => {
    assert.strictEqual(LINT_FORMAT_JSON_VERSION_HINT.test("error: unrecognized argument '--format'"), true);
  });

  test("does not match an unrelated stderr message", () => {
    assert.strictEqual(
      LINT_FORMAT_JSON_VERSION_HINT.test("error: file not found: vehicle.dat"),
      false
    );
  });
});

suite("extension.ts: shouldActivateInWorkspace (pure function)", () => {
  test("returns true for a trusted workspace", () => {
    assert.strictEqual(shouldActivateInWorkspace(true), true);
  });

  test("returns false for an untrusted workspace", () => {
    assert.strictEqual(shouldActivateInWorkspace(false), false);
  });
});

suite("formatter.ts: FMT_VERSION_HINT.test (pure regex)", () => {
  test("matches clap's 'unrecognized subcommand' phrasing for an unknown fmt subcommand", () => {
    assert.strictEqual(FMT_VERSION_HINT.test("error: unrecognized subcommand 'fmt'"), true);
  });

  test("matches clap's 'unexpected argument' phrasing for an unrecognized --config flag", () => {
    assert.strictEqual(FMT_VERSION_HINT.test("error: unexpected argument '--config' found"), true);
  });

  test("does not match an unrelated stderr message", () => {
    assert.strictEqual(FMT_VERSION_HINT.test("error: file not found: vehicle.dat"), false);
  });
});
