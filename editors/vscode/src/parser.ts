import * as vscode from "vscode";

/**
 * Parses the JSON output of `dat_linter lint <path> --format json` into VSCode
 * diagnostics.
 *
 * Schema (see ../../src/cli.rs and ../../src/commands/lint.rs on the Rust side
 * for the source of truth):
 *
 *   {
 *     "files": [
 *       {
 *         "path": "vehicle.dat",
 *         "diagnostics": [
 *           { "severity": "error", "code": "missing-waytype", "message": "...", "line": 12, "key": "waytype" },
 *           { "severity": "warn", "code": "duplicate-key", "message": "...", "line": null, "key": null }
 *         ]
 *       }
 *     ],
 *     "summary": { "error_count": 1, "warning_count": 1 }
 *   }
 *
 * `severity` is one of: error | warn | info | debug.
 * `line` is 1-indexed, or null when dat_linter can't anchor the diagnostic to
 * a specific line.
 *
 * The whole payload is emitted as a single JSON value on stdout only;
 * stderr is empty unless the process itself failed to run or to parse its
 * arguments (see extension.ts for that handling).
 */

export type DatLinterSeverity = "error" | "warn" | "info" | "debug";

export interface DatLinterJsonDiagnostic {
  severity: DatLinterSeverity;
  code: string;
  message: string;
  line: number | null;
  key: string | null;
}

export interface DatLinterJsonFile {
  path: string;
  diagnostics: DatLinterJsonDiagnostic[];
}

export interface DatLinterJsonSummary {
  error_count: number;
  warning_count: number;
}

export interface DatLinterJsonOutput {
  files: DatLinterJsonFile[];
  summary: DatLinterJsonSummary;
}

const KNOWN_SEVERITIES: readonly DatLinterSeverity[] = ["error", "warn", "info", "debug"];

function isDatLinterSeverity(value: unknown): value is DatLinterSeverity {
  return typeof value === "string" && (KNOWN_SEVERITIES as readonly string[]).includes(value);
}

function isDatLinterJsonDiagnostic(value: unknown): value is DatLinterJsonDiagnostic {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return (
    isDatLinterSeverity(v.severity) &&
    typeof v.code === "string" &&
    typeof v.message === "string" &&
    (v.line === null || typeof v.line === "number") &&
    (v.key === null || typeof v.key === "string")
  );
}

function isDatLinterJsonFile(value: unknown): value is DatLinterJsonFile {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return (
    typeof v.path === "string" &&
    Array.isArray(v.diagnostics) &&
    v.diagnostics.every(isDatLinterJsonDiagnostic)
  );
}

function isDatLinterJsonSummary(value: unknown): value is DatLinterJsonSummary {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return typeof v.error_count === "number" && typeof v.warning_count === "number";
}

/**
 * Type guard for the top-level `dat_linter lint --format json` payload.
 * Deliberately does not reject unknown extra keys, so additive schema
 * changes on the Rust side don't break parsing; only the fields this
 * extension actually reads are validated.
 */
export function isDatLinterJsonOutput(value: unknown): value is DatLinterJsonOutput {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return (
    Array.isArray(v.files) &&
    v.files.every(isDatLinterJsonFile) &&
    isDatLinterJsonSummary(v.summary)
  );
}

function mapSeverity(sev: DatLinterSeverity): vscode.DiagnosticSeverity {
  switch (sev) {
    case "error":
      return vscode.DiagnosticSeverity.Error;
    case "warn":
      return vscode.DiagnosticSeverity.Warning;
    case "info":
      return vscode.DiagnosticSeverity.Information;
    case "debug":
      return vscode.DiagnosticSeverity.Hint;
  }
}

export interface ParsedDiagnostic {
  /** 1-indexed line number as reported by dat_linter, or null if it gave no anchor. */
  line: number | null;
  severity: vscode.DiagnosticSeverity;
  code: string;
  message: string;
}

/**
 * Parses the stdout of `dat_linter lint <path> --format json` into
 * ParsedDiagnostic[], flattened across every `files[]` entry (in practice
 * the extension always invokes dat_linter against a single path, so this is
 * that one file's diagnostics).
 *
 * Throws a descriptive Error — rather than letting a raw JSON.parse
 * SyntaxError or TypeError bubble up unexplained — when the payload isn't
 * valid JSON or doesn't match the expected schema, so a future dat_linter
 * schema/version mismatch fails loudly instead of silently dropping
 * diagnostics.
 */
export function parseDatLinterJson(stdout: string): ParsedDiagnostic[] {
  let payload: unknown;
  try {
    payload = JSON.parse(stdout);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`dat_linter --format json produced invalid JSON: ${message}`);
  }

  if (!isDatLinterJsonOutput(payload)) {
    throw new Error(
      "dat_linter --format json output did not match the expected schema " +
        "(files[].diagnostics[].{severity,code,message,line,key} + summary.{error_count,warning_count}). " +
        "The dat_linter executable may be a version this extension does not support."
    );
  }

  const results: ParsedDiagnostic[] = [];
  for (const file of payload.files) {
    for (const diagnostic of file.diagnostics) {
      results.push({
        line: diagnostic.line,
        severity: mapSeverity(diagnostic.severity),
        code: diagnostic.code,
        message: diagnostic.message,
      });
    }
  }
  return results;
}

/**
 * Converts parsed diagnostics into vscode.Diagnostic objects anchored to a document.
 * Diagnostics with no line number (line === null) fall back to the document's first line.
 */
export function toVscodeDiagnostics(
  parsed: ParsedDiagnostic[],
  document: vscode.TextDocument
): vscode.Diagnostic[] {
  return parsed.map((p) => {
    // dat_linter reports 1-indexed lines; VSCode ranges are 0-indexed.
    const zeroIndexedLine = p.line !== null ? p.line - 1 : 0;
    const lineNo = Math.max(0, Math.min(zeroIndexedLine, document.lineCount - 1));
    const lineText = document.lineCount > 0 ? document.lineAt(lineNo).text : "";
    const range = new vscode.Range(lineNo, 0, lineNo, Math.max(lineText.length, 1));
    const diagnostic = new vscode.Diagnostic(range, p.message, p.severity);
    diagnostic.code = p.code;
    diagnostic.source = "dat_linter";
    return diagnostic;
  });
}
