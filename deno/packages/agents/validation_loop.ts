/**
 * Validation Loop - Enforces quality checks before agent completion.
 *
 * Wraps task agent execution to automatically validate work before allowing
 * completion. If validation fails, forces the agent to fix issues.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Validation check
// ---------------------------------------------------------------------------

/** Validation checks to enforce. */
export type ValidationCheck =
  | { kind: "no_duplicates" }
  | { kind: "build_success"; buildType: string }
  | { kind: "syntax_valid" }
  | { kind: "custom_command"; command: string; args: string[] };

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

/** Result of validation checks. */
export interface ValidationResult {
  /** Whether all checks passed. */
  passed: boolean;
  /** Issues found during validation. */
  issues: ValidationIssue[];
}

/** A single issue found during validation. */
export interface ValidationIssue {
  /** Name of the check that found this issue. */
  check: string;
  /** Severity of the issue. */
  severity: ValidationSeverity;
  /** Human-readable description. */
  message: string;
  /** File where the issue was found. */
  file?: string;
  /** Line number of the issue. */
  line?: number;
}

/** Severity level for a validation issue. */
export type ValidationSeverity = "error" | "warning" | "info";

// ---------------------------------------------------------------------------
// Validation config
// ---------------------------------------------------------------------------

/** Configuration for validation loop. */
export interface ValidationConfig {
  /** Checks to run. */
  checks: ValidationCheck[];
  /** Working directory for validation. */
  workingDirectory: string;
  /** Maximum validation retry attempts. Default: 3. */
  maxRetries: number;
  /** Whether to run validation (can disable for testing). Default: true. */
  enabled: boolean;
  /** Specific files to validate (from working set). */
  workingSetFiles: string[];
}

/** Create a default ValidationConfig. */
export function defaultValidationConfig(): ValidationConfig {
  return {
    checks: [{ kind: "no_duplicates" }, { kind: "syntax_valid" }],
    workingDirectory: ".",
    maxRetries: 3,
    enabled: true,
    workingSetFiles: [],
  };
}

/** Create a disabled ValidationConfig (for testing). */
export function disabledValidationConfig(): ValidationConfig {
  return { ...defaultValidationConfig(), enabled: false };
}

// ---------------------------------------------------------------------------
// Run validation
// ---------------------------------------------------------------------------

/** Check if file is a source code file worth validating. */
function isSourceFile(path: string): boolean {
  const lower = path.toLowerCase();
  return (
    lower.endsWith(".rs") ||
    lower.endsWith(".ts") ||
    lower.endsWith(".tsx") ||
    lower.endsWith(".js") ||
    lower.endsWith(".jsx") ||
    lower.endsWith(".py") ||
    lower.endsWith(".java") ||
    lower.endsWith(".cpp") ||
    lower.endsWith(".c") ||
    lower.endsWith(".go") ||
    lower.endsWith(".rb")
  );
}

/**
 * Run validation checks on changed files.
 *
 * Note: In the Deno environment, only file existence checks and custom
 * command checks (via Deno.Command) are supported. The duplicate/syntax
 * checks are stubs that pass by default.
 */
export async function runValidation(
  config: ValidationConfig,
): Promise<ValidationResult> {
  if (!config.enabled) {
    return { passed: true, issues: [] };
  }

  const issues: ValidationIssue[] = [];
  const changedFiles = config.workingSetFiles;

  // Verify files in working set actually exist
  for (const file of changedFiles) {
    const filePath = config.workingDirectory === "."
      ? file
      : `${config.workingDirectory}/${file}`;

    try {
      await Deno.stat(filePath);
    } catch {
      issues.push({
        check: "file_existence",
        severity: "error",
        message: `File '${file}' is in working set but does not exist on disk. Agent must create file before completing.`,
        file,
      });
    }
  }

  for (const check of config.checks) {
    if (check.kind === "custom_command") {
      try {
        const cmd = new Deno.Command(check.command, {
          args: check.args,
          cwd: config.workingDirectory,
          stdout: "piped",
          stderr: "piped",
        });
        const output = await cmd.output();
        if (!output.success) {
          const stderr = new TextDecoder().decode(output.stderr);
          issues.push({
            check: "custom_command",
            severity: "error",
            message: `Command '${check.command}' failed: ${stderr}`,
          });
        }
      } catch (e) {
        issues.push({
          check: "custom_command",
          severity: "error",
          message: `Failed to run command '${check.command}': ${e}`,
        });
      }
    }
    // no_duplicates and syntax_valid are stubs in TS -- they pass by default
  }

  return { passed: issues.length === 0, issues };
}

// ---------------------------------------------------------------------------
// Format feedback
// ---------------------------------------------------------------------------

/** Format validation result as feedback for the agent. */
export function formatValidationFeedback(result: ValidationResult): string {
  if (result.passed) {
    return "All validation checks passed!";
  }

  let feedback = "VALIDATION FAILED - You must fix these issues:\n\n";

  for (let idx = 0; idx < result.issues.length; idx++) {
    const issue = result.issues[idx];
    feedback += `${idx + 1}. [${issue.check}] `;
    if (issue.file) {
      feedback += `${issue.file}:`;
      if (issue.line != null) feedback += `${issue.line}:`;
      feedback += " ";
    }
    feedback += `${issue.message}\n`;
  }

  feedback += "\n";
  feedback +=
    "IMPORTANT: You MUST fix ALL of these issues before the task can complete.\n";
  feedback +=
    "After fixing, verify your changes by reading the files back.\n";

  return feedback;
}
