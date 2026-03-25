/**
 * Skill Metadata and Core Types
 *
 * Defines the data structures for Agent Skills with progressive disclosure:
 * - `SkillMetadata`: Lightweight metadata loaded at startup
 * - `Skill`: Full skill content loaded on-demand
 * - Supporting enums for source and execution modes
 */

/** Source location of a skill. */
export type SkillSource = "personal" | "project" | "builtin";

/** Execution mode for a skill. */
export type SkillExecutionMode = "inline" | "subagent" | "script";

/**
 * Parse an execution mode string, defaulting to "inline" for unknown values.
 *
 * @param s - The string to parse
 * @returns The parsed execution mode
 */
export function parseExecutionMode(s: string): SkillExecutionMode {
  const lower = s.toLowerCase();
  if (lower === "subagent") return "subagent";
  if (lower === "script") return "script";
  return "inline";
}

/**
 * Lightweight skill metadata loaded at startup.
 *
 * Only contains the information needed for:
 * - Displaying skill listings
 * - Semantic matching against descriptions
 * - Determining if full content should be loaded
 *
 * The actual instructions are lazily loaded when the skill is activated.
 */
export interface SkillMetadata {
  /** Skill name (lowercase, hyphens only, max 64 chars). Used as the identifier and for `/skill-name` invocation. */
  name: string;
  /** Description (max 1024 chars). Used for semantic matching. */
  description: string;
  /** Optional: Restrict available tools during execution. Serialized as `allowed-tools`. */
  "allowed-tools"?: string[];
  /** Optional: Software license for the skill. */
  license?: string;
  /** Optional: Environment requirements (max 500 chars). */
  compatibility?: string;
  /** Optional: Specific model to use for this skill. */
  model?: string;
  /** Optional: Custom key-value metadata. Common keys: "category", "execution", "author", "version". */
  metadata?: Record<string, string>;
  /** Optional: lifecycle hook event types this skill subscribes to. */
  hooks?: string[];
  /** Source location (personal, project, or builtin). Not serialized to JSON. */
  source: SkillSource;
  /** File path for lazy loading the full content. Not serialized to JSON. */
  sourcePath: string;
}

/**
 * Create a new SkillMetadata with required fields and defaults.
 *
 * @param name - Skill name
 * @param description - Skill description
 * @returns A new SkillMetadata instance
 */
export function createSkillMetadata(
  name: string,
  description: string,
): SkillMetadata {
  return {
    name,
    description,
    source: "personal",
    sourcePath: "",
  };
}

/**
 * Get the execution mode from metadata's custom metadata map.
 *
 * @param meta - The skill metadata
 * @returns The execution mode
 */
export function executionMode(meta: SkillMetadata): SkillExecutionMode {
  const exec = meta.metadata?.["execution"];
  if (exec) return parseExecutionMode(exec);
  return "inline";
}

/**
 * Get a custom metadata value.
 *
 * @param meta - The skill metadata
 * @param key - The metadata key to look up
 * @returns The value if found, or undefined
 */
export function getMetadataValue(
  meta: SkillMetadata,
  key: string,
): string | undefined {
  return meta.metadata?.[key];
}

/**
 * Check if skill has tool restrictions.
 *
 * @param meta - The skill metadata
 * @returns True if the skill restricts tool access
 */
export function hasToolRestrictions(meta: SkillMetadata): boolean {
  const tools = meta["allowed-tools"];
  return tools != null && tools.length > 0;
}

/**
 * Check if a tool is allowed for this skill.
 *
 * @param meta - The skill metadata
 * @param toolName - The tool name to check
 * @returns True if the tool is allowed (no restrictions means all allowed)
 */
export function isToolAllowed(meta: SkillMetadata, toolName: string): boolean {
  const allowed = meta["allowed-tools"];
  if (allowed == null) return true;
  return allowed.includes(toolName);
}

/**
 * Full skill content loaded on-demand.
 *
 * Contains both the metadata and the instruction content.
 * Created by parsing the full SKILL.md file when the skill is activated.
 */
export interface Skill {
  /** Lightweight metadata. */
  metadata: SkillMetadata;
  /** Full instruction content (markdown body after frontmatter). */
  instructions: string;
  /** Execution mode (derived from metadata or defaults to inline). */
  executionMode: SkillExecutionMode;
}

/**
 * Create a new Skill from metadata and instructions.
 *
 * @param metadata - The skill metadata
 * @param instructions - The instruction content
 * @returns A new Skill instance
 */
export function createSkill(
  metadata: SkillMetadata,
  instructions: string,
): Skill {
  return {
    metadata,
    instructions,
    executionMode: executionMode(metadata),
  };
}

/**
 * Check if this skill should run as a subagent.
 *
 * @param skill - The skill to check
 * @returns True if the skill runs as a subagent
 */
export function runsAsSubagent(skill: Skill): boolean {
  return skill.executionMode === "subagent";
}

/**
 * Check if this skill is a script.
 *
 * @param skill - The skill to check
 * @returns True if the skill is a script
 */
export function isScript(skill: Skill): boolean {
  return skill.executionMode === "script";
}

/** Result of skill execution. */
export type SkillResult =
  | {
    /** Discriminant tag. */
    type: "inline";
    /** The rendered instructions. */
    instructions: string;
    /** Optional model override. */
    modelOverride?: string;
  }
  | {
    /** Discriminant tag. */
    type: "subagent";
    /** The spawned agent's ID. */
    agentId: string;
  }
  | {
    /** Discriminant tag. */
    type: "script";
    /** Script output. */
    output: string;
    /** Whether execution resulted in an error. */
    isError: boolean;
  };

/**
 * Create an inline result.
 *
 * @param instructions - The rendered instructions
 * @param modelOverride - Optional model override
 * @returns An inline SkillResult
 */
export function inlineResult(
  instructions: string,
  modelOverride?: string,
): SkillResult {
  return { type: "inline", instructions, modelOverride };
}

/**
 * Create a subagent result.
 *
 * @param agentId - The spawned agent's ID
 * @returns A subagent SkillResult
 */
export function subagentResult(agentId: string): SkillResult {
  return { type: "subagent", agentId };
}

/**
 * Create a script result.
 *
 * @param output - Script output
 * @param isError - Whether execution resulted in an error
 * @returns A script SkillResult
 */
export function scriptResult(output: string, isError: boolean): SkillResult {
  return { type: "script", output, isError };
}

/**
 * Check if a skill result is an error.
 *
 * @param result - The skill result to check
 * @returns True if this is a script result with an error
 */
export function isResultError(result: SkillResult): boolean {
  return result.type === "script" && result.isError;
}

/** How a skill match was determined. */
export type MatchSource = "semantic" | "keyword" | "explicit";

/** Match result from skill router. */
export interface SkillMatch {
  /** Name of the matched skill. */
  skillName: string;
  /** Confidence score (0.0 to 1.0). */
  confidence: number;
  /** How the match was determined. */
  source: MatchSource;
}

/**
 * Create a new skill match.
 *
 * @param skillName - Name of the matched skill
 * @param confidence - Confidence score
 * @param source - How the match was determined
 * @returns A new SkillMatch
 */
export function createSkillMatch(
  skillName: string,
  confidence: number,
  source: MatchSource,
): SkillMatch {
  return { skillName, confidence, source };
}

/**
 * Create a semantic match.
 *
 * @param skillName - Name of the matched skill
 * @param confidence - Confidence score
 * @returns A semantic SkillMatch
 */
export function semanticMatch(
  skillName: string,
  confidence: number,
): SkillMatch {
  return createSkillMatch(skillName, confidence, "semantic");
}

/**
 * Create a keyword match.
 *
 * @param skillName - Name of the matched skill
 * @param confidence - Confidence score
 * @returns A keyword SkillMatch
 */
export function keywordMatch(
  skillName: string,
  confidence: number,
): SkillMatch {
  return createSkillMatch(skillName, confidence, "keyword");
}

/**
 * Create an explicit match (user invoked directly).
 *
 * @param skillName - Name of the matched skill
 * @returns An explicit SkillMatch with confidence 1.0
 */
export function explicitMatch(skillName: string): SkillMatch {
  return createSkillMatch(skillName, 1.0, "explicit");
}
