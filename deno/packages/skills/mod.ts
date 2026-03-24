/**
 * @module @brainwires/skills
 *
 * Agent skills system -- SKILL.md parsing, registry, routing, and execution.
 * Equivalent to Rust's `brainwires-skills` crate.
 *
 * Skills are markdown-based packages that extend agent capabilities using
 * progressive disclosure:
 * - At startup: only metadata (name, description) is loaded for fast matching
 * - On activation: full SKILL.md content is loaded on-demand
 *
 * ## SKILL.md Format
 *
 * ```markdown
 * ---
 * name: review-pr
 * description: Reviews pull requests for code quality and security issues.
 * allowed-tools:
 *   - Read
 *   - Grep
 * model: claude-sonnet-4
 * metadata:
 *   category: code-review
 *   execution: subagent
 * ---
 *
 * # PR Review Instructions
 * ...
 * ```
 */

// Executor types
export { SkillExecutor, type ScriptPrepared, type SubagentPrepared } from "./executor.ts";

// Metadata and core types
export {
  createSkill,
  createSkillMatch,
  createSkillMetadata,
  executionMode,
  explicitMatch,
  getMetadataValue,
  hasToolRestrictions,
  inlineResult,
  isResultError,
  isScript,
  isToolAllowed,
  keywordMatch,
  parseExecutionMode,
  runsAsSubagent,
  scriptResult,
  semanticMatch,
  subagentResult,
  type MatchSource,
  type Skill,
  type SkillExecutionMode,
  type SkillMatch,
  type SkillMetadata,
  type SkillResult,
  type SkillSource,
} from "./metadata.ts";

// Parser
export {
  parseMetadataFromContent,
  parseSkillFile,
  parseSkillFromContent,
  parseSkillMetadata,
  renderTemplate,
  validateCompatibility,
  validateDescription,
  validateSkillName,
} from "./parser.ts";

// Registry
export { SkillRegistry, truncateDescription, type DiscoveryPath } from "./registry.ts";

// Router
export { SkillRouter } from "./router.ts";
