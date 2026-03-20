/**
 * Saga-Style Compensating Transactions.
 *
 * Implements the Saga pattern for multi-step operations. When an operation
 * fails mid-way, compensation actions are executed in reverse order to undo
 * completed sub-operations.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Operation types
// ---------------------------------------------------------------------------

/** Types of saga operations for categorization. */
export type SagaOperationType =
  | "file_write"
  | "file_edit"
  | "file_delete"
  | "git_stage"
  | "git_unstage"
  | "git_commit"
  | "git_branch_create"
  | "git_branch_delete"
  | "build"
  | "test"
  | "generic";

/** Returns true if this operation type can be compensated. */
export function isCompensable(opType: SagaOperationType): boolean {
  switch (opType) {
    case "file_write":
    case "file_edit":
    case "file_delete":
    case "git_stage":
    case "git_unstage":
    case "git_commit":
    case "git_branch_create":
    case "git_branch_delete":
      return true;
    case "build":
    case "test":
    case "generic":
      return false;
  }
}

// ---------------------------------------------------------------------------
// Operation result
// ---------------------------------------------------------------------------

/** Result of an operation, needed for compensation. */
export interface OperationResult {
  /** Unique identifier for this operation. */
  operationId: string;
  /** Whether the operation succeeded. */
  success: boolean;
  /** State captured before operation (for rollback). */
  checkpoint?: Checkpoint;
  /** Metadata needed for compensation. */
  compensationData: unknown;
  /** Output from the operation. */
  output?: string;
}

/** Create a successful operation result. */
export function successResult(
  operationId: string,
  compensationData: unknown = null,
): OperationResult {
  return { operationId, success: true, compensationData };
}

/** Create a failed operation result. */
export function failureResult(operationId: string): OperationResult {
  return { operationId, success: false, compensationData: null };
}

// ---------------------------------------------------------------------------
// Checkpoint
// ---------------------------------------------------------------------------

/** Snapshot of a file's state for restoration. */
export interface FileState {
  path: string;
  contentHash: string;
  originalContent?: string;
}

/** Snapshot of git state for restoration. */
export interface GitCheckpoint {
  headCommit: string;
  stagedFiles: string[];
  branch: string;
}

/** Checkpoint for state restoration. */
export interface Checkpoint {
  id: string;
  timestamp: number;
  fileStates: FileState[];
  gitState?: GitCheckpoint;
}

/** Create a new checkpoint. */
export function createCheckpoint(id: string): Checkpoint {
  return { id, timestamp: Date.now(), fileStates: [] };
}

// ---------------------------------------------------------------------------
// Compensable operation interface
// ---------------------------------------------------------------------------

/** A compensable operation that can be undone. */
export interface CompensableOperation {
  /** Execute the forward operation. */
  execute(): Promise<OperationResult>;
  /** Compensate (undo) the operation. */
  compensate(result: OperationResult): Promise<void>;
  /** Get operation description for logging. */
  description(): string;
  /** Get the operation type. */
  operationType(): SagaOperationType;
}

// ---------------------------------------------------------------------------
// Saga status
// ---------------------------------------------------------------------------

/** Current status of a saga execution. */
export type SagaStatus =
  | "running"
  | "completed"
  | "failed"
  | "compensating"
  | "compensated";

// ---------------------------------------------------------------------------
// Compensation report
// ---------------------------------------------------------------------------

/** Outcome of a compensation attempt. */
export type CompensationOutcome = "success" | "failed" | "skipped";

/** Status of a single compensation action. */
export interface CompensationStatus {
  description: string;
  status: CompensationOutcome;
  error?: string;
}

/** Report of compensation actions. */
export class CompensationReport {
  readonly sagaId: string;
  readonly operations: CompensationStatus[] = [];
  readonly startedAt: number;
  completedAt?: number;

  constructor(sagaId: string) {
    this.sagaId = sagaId;
    this.startedAt = Date.now();
  }

  addSuccess(description: string): void {
    this.operations.push({ description, status: "success" });
  }

  addFailure(description: string, error: string): void {
    this.operations.push({ description, status: "failed", error });
  }

  addSkipped(description: string, reason: string): void {
    this.operations.push({ description, status: "skipped", error: reason });
  }

  /** Returns true if all compensations succeeded or were skipped. */
  allSuccessful(): boolean {
    return this.operations.every(
      (s) => s.status === "success" || s.status === "skipped",
    );
  }

  /** Generate a human-readable summary. */
  summary(): string {
    const successful = this.operations.filter(
      (s) => s.status === "success",
    ).length;
    const failed = this.operations.filter(
      (s) => s.status === "failed",
    ).length;
    const skipped = this.operations.filter(
      (s) => s.status === "skipped",
    ).length;
    return `${successful} successful, ${failed} failed, ${skipped} skipped (total: ${this.operations.length})`;
  }

  markCompleted(): void {
    this.completedAt = Date.now();
  }
}

// ---------------------------------------------------------------------------
// Saga executor
// ---------------------------------------------------------------------------

/** Saga executor that manages compensating transactions. */
export class SagaExecutor {
  readonly sagaId: string;
  readonly agentId: string;
  readonly description: string;
  readonly startedAt: number;
  private completedOps: Array<{
    op: CompensableOperation;
    result: OperationResult;
  }> = [];
  private _status: SagaStatus = "running";
  private compensationHooks: Array<
    (summary: string, allSuccessful: boolean) => void
  > = [];

  constructor(agentId: string, description: string) {
    this.agentId = agentId;
    this.description = description;
    this.startedAt = Date.now();
    this.sagaId = `saga-${agentId}-${Date.now()}`;
  }

  /** Get current status. */
  get status(): SagaStatus {
    return this._status;
  }

  /** Get number of completed operations. */
  operationCount(): number {
    return this.completedOps.length;
  }

  /** Execute an operation within the saga. */
  async executeStep(op: CompensableOperation): Promise<OperationResult> {
    if (this._status !== "running") {
      throw new Error("Cannot execute step: saga is not running");
    }

    const result = await op.execute();

    if (result.success) {
      this.completedOps.push({ op, result });
    } else {
      this._status = "failed";
    }

    return result;
  }

  /** Mark the saga as successfully completed. */
  complete(): void {
    this._status = "completed";
  }

  /** Mark the saga as failed. */
  fail(): void {
    this._status = "failed";
  }

  /** Compensate all completed operations in reverse order. */
  async compensateAll(): Promise<CompensationReport> {
    this._status = "compensating";
    const report = new CompensationReport(this.sagaId);

    while (this.completedOps.length > 0) {
      const { op, result } = this.completedOps.pop()!;

      if (!isCompensable(op.operationType())) {
        report.addSkipped(
          op.description(),
          "Non-compensable operation type",
        );
        continue;
      }

      try {
        await op.compensate(result);
        report.addSuccess(op.description());
      } catch (e) {
        report.addFailure(op.description(), String(e));
        // Continue compensating even if one fails
      }
    }

    this._status = "compensated";

    const summaryText = report.summary();
    const allOk = report.allSuccessful();
    for (const hook of this.compensationHooks) {
      try { hook(summaryText, allOk); } catch { /* ignore */ }
    }

    return report;
  }

  /** Add a hook called after compensation. */
  onCompensation(
    hook: (summary: string, allSuccessful: boolean) => void,
  ): void {
    this.compensationHooks.push(hook);
  }

  /** Get descriptions of all completed operations. */
  getOperationDescriptions(): string[] {
    return this.completedOps.map(({ op }) => op.description());
  }
}
