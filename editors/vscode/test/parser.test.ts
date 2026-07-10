import * as assert from "assert";
import * as vscode from "vscode";
import {
  isDatLinterJsonOutput,
  parseDatLinterJson,
  toVscodeDiagnostics,
} from "../src/parser";

// The schema example from dat_linter's `lint --format json` (see ../../docs/lint.md
// and ../src/cli.rs on the Rust side).
const SAMPLE_JSON = JSON.stringify({
  files: [
    {
      path: "vehicle.dat",
      diagnostics: [
        {
          severity: "error",
          code: "missing-waytype",
          message: "waytype is required",
          line: 12,
          key: "waytype",
        },
        {
          severity: "warn",
          code: "duplicate-key",
          message: "duplicate key",
          line: null,
          key: null,
        },
      ],
    },
  ],
  summary: { error_count: 1, warning_count: 1 },
});

suite("parser: parseDatLinterJson / isDatLinterJsonOutput (pure functions)", () => {
  test("parses a well-formed payload into ParsedDiagnostic[]", () => {
    const parsed = parseDatLinterJson(SAMPLE_JSON);

    assert.strictEqual(parsed.length, 2);

    assert.strictEqual(parsed[0].code, "missing-waytype");
    assert.strictEqual(parsed[0].severity, vscode.DiagnosticSeverity.Error);
    assert.strictEqual(parsed[0].line, 12);
    assert.strictEqual(parsed[0].message, "waytype is required");

    assert.strictEqual(parsed[1].code, "duplicate-key");
    assert.strictEqual(parsed[1].severity, vscode.DiagnosticSeverity.Warning);
    assert.strictEqual(parsed[1].line, null);
  });

  test("maps info and debug severities", () => {
    const payload = JSON.stringify({
      files: [
        {
          path: "x.dat",
          diagnostics: [
            { severity: "info", code: "c1", message: "m1", line: 1, key: null },
            { severity: "debug", code: "c2", message: "m2", line: null, key: null },
          ],
        },
      ],
      summary: { error_count: 0, warning_count: 0 },
    });

    const parsed = parseDatLinterJson(payload);
    assert.strictEqual(parsed[0].severity, vscode.DiagnosticSeverity.Information);
    assert.strictEqual(parsed[1].severity, vscode.DiagnosticSeverity.Hint);
  });

  test("flattens diagnostics across multiple files", () => {
    const payload = JSON.stringify({
      files: [
        { path: "a.dat", diagnostics: [{ severity: "error", code: "e1", message: "m", line: 1, key: null }] },
        { path: "b.dat", diagnostics: [{ severity: "warn", code: "w1", message: "m", line: 2, key: null }] },
      ],
      summary: { error_count: 1, warning_count: 1 },
    });

    const parsed = parseDatLinterJson(payload);
    assert.strictEqual(parsed.length, 2);
    assert.strictEqual(parsed[0].code, "e1");
    assert.strictEqual(parsed[1].code, "w1");
  });

  test("returns an empty array for a clean file (empty diagnostics)", () => {
    const payload = JSON.stringify({
      files: [{ path: "clean.dat", diagnostics: [] }],
      summary: { error_count: 0, warning_count: 0 },
    });

    assert.deepStrictEqual(parseDatLinterJson(payload), []);
  });

  test("throws a descriptive error on invalid JSON", () => {
    assert.throws(() => parseDatLinterJson("not json{"), /invalid JSON/);
  });

  test("throws a descriptive error when the payload doesn't match the schema", () => {
    // Missing "summary" entirely.
    const malformed = JSON.stringify({ files: [] });
    assert.throws(() => parseDatLinterJson(malformed), /did not match the expected schema/);
  });

  test("isDatLinterJsonOutput rejects an unknown severity value", () => {
    const malformed = {
      files: [
        {
          path: "x.dat",
          diagnostics: [{ severity: "critical", code: "c", message: "m", line: null, key: null }],
        },
      ],
      summary: { error_count: 0, warning_count: 0 },
    };
    assert.strictEqual(isDatLinterJsonOutput(malformed), false);
  });

  test("isDatLinterJsonOutput rejects a non-object payload", () => {
    assert.strictEqual(isDatLinterJsonOutput(null), false);
    assert.strictEqual(isDatLinterJsonOutput("hello"), false);
    assert.strictEqual(isDatLinterJsonOutput(42), false);
  });

  test("isDatLinterJsonOutput accepts the sample payload", () => {
    assert.strictEqual(isDatLinterJsonOutput(JSON.parse(SAMPLE_JSON)), true);
  });
});

suite("parser: toVscodeDiagnostics (line: null fallback)", () => {
  test("a diagnostic with line: null falls back to the document's first line (0-indexed line 0)", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "line one\nline two\nline three\n",
      language: "plaintext",
    });

    const diagnostics = toVscodeDiagnostics(
      [
        {
          line: null,
          severity: vscode.DiagnosticSeverity.Error,
          code: "missing-waytype",
          message: "no line anchor",
        },
      ],
      document
    );

    assert.strictEqual(diagnostics.length, 1);
    assert.strictEqual(diagnostics[0].range.start.line, 0);
    assert.strictEqual(diagnostics[0].code, "missing-waytype");
    assert.strictEqual(diagnostics[0].source, "dat_linter");
  });

  test("a diagnostic with a 1-indexed line converts to the matching 0-indexed VSCode line", async () => {
    const document = await vscode.workspace.openTextDocument({
      content: "line one\nline two\nline three\n",
      language: "plaintext",
    });

    const diagnostics = toVscodeDiagnostics(
      [
        {
          line: 3, // 1-indexed -> "line three"
          severity: vscode.DiagnosticSeverity.Warning,
          code: "duplicate-key",
          message: "dup",
        },
      ],
      document
    );

    assert.strictEqual(diagnostics[0].range.start.line, 2);
  });
});
