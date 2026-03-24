/**
 * Optimistic Concurrency with Conflict Resolution.
 *
 * Provides optimistic concurrency control that allows agents to proceed
 * with operations without acquiring locks upfront. Conflicts are detected
 * at commit time and resolved using configured strategies.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Resource version
// ---------------------------------------------------------------------------

/** Version information for a resource. */
export interface ResourceVersion {
  version: number;
  contentHash: string;
  lastModifier: string;
  modifiedAt: number;
}

// ---------------------------------------------------------------------------
// Resolution strategy
// ---------------------------------------------------------------------------

/** Strategies for merging conflicting changes. */
export type MergeStrategy = "text_merge" | "json_merge" | "append" | { custom: string };

/** Strategy for resolving conflicts. */
export type ResolutionStrategy =
  | { kind: "last_writer_wins" }
  | { kind: "first_writer_wins" }
  | { kind: "merge"; strategy: MergeStrategy }
  | { kind: "escalate" }
  | { kind: "retry"; maxAttempts: number };

// ---------------------------------------------------------------------------
// Conflict types
// ---------------------------------------------------------------------------

/** Describes a conflict between two operations. */
export interface OptimisticConflict {
  resourceId: string;
  conflictingAgent: string;
  expectedVersion: number;
  actualVersion: number;
  holderAgent: string;
  detectedAt: number;
}

/** Full conflict information for resolution. */
export interface OptimisticConflictDetails {
  resourceId: string;
  agentA: string;
  agentB: string;
  versionA: ResourceVersion;
  versionB: ResourceVersion;
  baseVersion: ResourceVersion;
  contentA?: string;
  contentB?: string;
}

/** Result of conflict resolution. */
export type Resolution =
  | { kind: "use_version"; agent: string }
  | { kind: "merged"; hash: string }
  | { kind: "abort_both" }
  | { kind: "keep_both"; suffixA: string; suffixB: string }
  | { kind: "retry" }
  | { kind: "escalate"; reason: string };

// ---------------------------------------------------------------------------
// Optimistic token
// ---------------------------------------------------------------------------

/** Token for optimistic operations. */
export interface OptimisticToken {
  resourceId: string;
  baseVersion: number;
  baseHash: string;
  agentId: string;
  createdAt: number;
}

/** Check if a token has expired (stale). */
export function isTokenStale(token: OptimisticToken, maxAgeMs: number): boolean {
  return Date.now() - token.createdAt > maxAgeMs;
}

// ---------------------------------------------------------------------------
// Conflict record
// ---------------------------------------------------------------------------

/** Record of a conflict for history/debugging. */
export interface ConflictRecord {
  conflict: OptimisticConflict;
  resolution: Resolution;
  resolvedAt: number;
}

// ---------------------------------------------------------------------------
// Commit result
// ---------------------------------------------------------------------------

/** Result of a commit operation. */
export type CommitResult =
  | { kind: "committed"; version: number }
  | { kind: "merged"; version: number; mergedHash: string }
  | { kind: "retry_needed"; currentVersion: number }
  | { kind: "rejected"; reason: string }
  | { kind: "aborted"; reason: string }
  | { kind: "split"; suffixA: string; suffixB: string }
  | { kind: "escalated"; reason: string };

/** Check if the commit result indicates success. */
export function isCommitSuccess(result: CommitResult): boolean {
  return result.kind === "committed" || result.kind === "merged";
}

/** Get the new version from a successful commit. */
export function commitVersion(result: CommitResult): number | undefined {
  if (result.kind === "committed" || result.kind === "merged") {
    return result.version;
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/** Statistics about optimistic concurrency. */
export interface OptimisticStats {
  totalResources: number;
  totalConflicts: number;
  resolvedByRetry: number;
  escalated: number;
}

// ---------------------------------------------------------------------------
// Optimistic controller
// ---------------------------------------------------------------------------

/** Optimistic concurrency controller. */
export class OptimisticController {
  private versions = new Map<string, ResourceVersion>();
  private strategies = new Map<string, ResolutionStrategy>();
  private defaultStrategy: ResolutionStrategy;
  private conflictHistory: ConflictRecord[] = [];
  private maxHistory: number;

  constructor(
    defaultStrategy?: ResolutionStrategy,
    maxHistory = 100,
  ) {
    this.defaultStrategy = defaultStrategy ?? { kind: "first_writer_wins" };
    this.maxHistory = maxHistory;
  }

  /** Start an optimistic operation -- returns token with current version. */
  beginOptimistic(agentId: string, resourceId: string): OptimisticToken {
    const current = this.versions.get(resourceId);
    return {
      resourceId,
      baseVersion: current?.version ?? 0,
      baseHash: current?.contentHash ?? "",
      agentId,
      createdAt: Date.now(),
    };
  }

  /** Commit optimistic operation -- throws conflict if version changed. */
  commitOptimistic(
    token: OptimisticToken,
    newContentHash: string,
  ): number {
    const current = this.versions.get(token.resourceId);
    if (current && current.version !== token.baseVersion) {
      const conflict: OptimisticConflict = {
        resourceId: token.resourceId,
        conflictingAgent: token.agentId,
        expectedVersion: token.baseVersion,
        actualVersion: current.version,
        holderAgent: current.lastModifier,
        detectedAt: Date.now(),
      };
      throw conflict;
    }

    const newVersion = token.baseVersion + 1;
    this.versions.set(token.resourceId, {
      version: newVersion,
      contentHash: newContentHash,
      lastModifier: token.agentId,
      modifiedAt: Date.now(),
    });
    return newVersion;
  }

  /** Try to commit, and if conflict occurs, resolve it. */
  commitOrResolve(
    token: OptimisticToken,
    newContentHash: string,
    _newContent?: string,
  ): CommitResult {
    try {
      const version = this.commitOptimistic(token, newContentHash);
      return { kind: "committed", version };
    } catch (e) {
      const conflict = e as OptimisticConflict;
      const resolution = this.resolveConflictAuto(conflict);
      this.recordConflict(conflict, resolution);

      switch (resolution.kind) {
        case "use_version":
          if (resolution.agent === token.agentId) {
            const version = this.forceCommit(
              token.resourceId,
              newContentHash,
              token.agentId,
            );
            return { kind: "committed", version };
          }
          return {
            kind: "rejected",
            reason: `Conflict resolved in favor of ${resolution.agent}`,
          };
        case "merged":
          return {
            kind: "merged",
            version: this.forceCommit(
              token.resourceId,
              resolution.hash,
              token.agentId,
            ),
            mergedHash: resolution.hash,
          };
        case "retry":
          return {
            kind: "retry_needed",
            currentVersion: conflict.actualVersion,
          };
        case "abort_both":
          return {
            kind: "aborted",
            reason: "Both operations aborted due to conflict",
          };
        case "keep_both":
          return {
            kind: "split",
            suffixA: resolution.suffixA,
            suffixB: resolution.suffixB,
          };
        case "escalate":
          return { kind: "escalated", reason: resolution.reason };
      }
    }
  }

  private forceCommit(
    resourceId: string,
    contentHash: string,
    agentId: string,
  ): number {
    const current = this.versions.get(resourceId);
    const newVersion = (current?.version ?? 0) + 1;
    this.versions.set(resourceId, {
      version: newVersion,
      contentHash,
      lastModifier: agentId,
      modifiedAt: Date.now(),
    });
    return newVersion;
  }

  private resolveConflictAuto(conflict: OptimisticConflict): Resolution {
    const strategy =
      this.strategies.get(conflict.resourceId) ?? this.defaultStrategy;

    switch (strategy.kind) {
      case "last_writer_wins":
        return { kind: "use_version", agent: conflict.conflictingAgent };
      case "first_writer_wins":
        return { kind: "use_version", agent: conflict.holderAgent };
      case "retry":
        if (
          conflict.actualVersion - conflict.expectedVersion <
          strategy.maxAttempts
        ) {
          return { kind: "retry" };
        }
        return {
          kind: "escalate",
          reason: `Max retry attempts (${strategy.maxAttempts}) exceeded`,
        };
      case "escalate":
        return {
          kind: "escalate",
          reason: "Configured to escalate all conflicts",
        };
      case "merge":
        return {
          kind: "escalate",
          reason: "Merge requires content, not available",
        };
    }
  }

  private recordConflict(
    conflict: OptimisticConflict,
    resolution: Resolution,
  ): void {
    this.conflictHistory.push({
      conflict,
      resolution,
      resolvedAt: Date.now(),
    });
    while (this.conflictHistory.length > this.maxHistory) {
      this.conflictHistory.shift();
    }
  }

  /** Register a resolution strategy for a resource pattern. */
  registerStrategy(
    resourcePattern: string,
    strategy: ResolutionStrategy,
  ): void {
    this.strategies.set(resourcePattern, strategy);
  }

  /** Get the current version of a resource. */
  getVersion(resourceId: string): ResourceVersion | undefined {
    return this.versions.get(resourceId);
  }

  /** Check if a resource has been modified since a given version. */
  hasChanged(resourceId: string, sinceVersion: number): boolean {
    const v = this.versions.get(resourceId);
    return v != null && v.version > sinceVersion;
  }

  /** Get conflict history. */
  getConflictHistory(): ConflictRecord[] {
    return [...this.conflictHistory];
  }

  /** Clear conflict history. */
  clearHistory(): void {
    this.conflictHistory = [];
  }

  /** Get statistics. */
  getStats(): OptimisticStats {
    return {
      totalResources: this.versions.size,
      totalConflicts: this.conflictHistory.length,
      resolvedByRetry: this.conflictHistory.filter(
        (r) => r.resolution.kind === "retry",
      ).length,
      escalated: this.conflictHistory.filter(
        (r) => r.resolution.kind === "escalate",
      ).length,
    };
  }
}
