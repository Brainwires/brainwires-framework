/**
 * Skill Executor
 *
 * Executes skills in one of three modes:
 * - **Inline**: Instructions returned for injection into the conversation
 * - **Subagent**: Execution info returned; caller spawns via AgentPool
 * - **Script**: Script content returned; caller executes via OrchestratorTool
 *
 * Tool restrictions from `allowed-tools` are enforced in `prepare*` methods.
 */

import {
  executionMode,
  inlineResult,
  type Skill,
  type SkillExecutionMode,
  type SkillResult,
} from "./skills_metadata.ts";
import { renderTemplate } from "./skills_parser.ts";
import { SkillRegistry } from "./skills_registry.ts";

/**
 * Prepared subagent execution -- caller converts into Task + AgentContext.
 */
export interface SubagentPrepared {
  /** Task description (rendered instructions). */
  taskDescription: string;
  /** Tool names allowed for this skill (filtered from available tools). */
  allowedToolNames: string[];
  /** System prompt for the subagent. */
  systemPrompt: string;
  /** Optional model override. */
  modelOverride?: string;
}

/**
 * Prepared script execution -- caller executes via OrchestratorTool.
 */
export interface ScriptPrepared {
  /** The rendered script content. */
  scriptContent: string;
  /** Tool names allowed for this skill (filtered from available tools). */
  allowedToolNames: string[];
  /** Optional model override. */
  modelOverride?: string;
  /** Skill name for logging. */
  skillName: string;
}

/**
 * Skill executor handles the execution of skills in various modes.
 *
 * Dispatches to the appropriate execution mode based on skill metadata,
 * renders templates with provided arguments, and enforces tool restrictions.
 */
export class SkillExecutor {
  /** Reference to skill registry for loading skills. */
  private registry: SkillRegistry;

  /**
   * Create a new skill executor.
   *
   * @param registry - The skill registry
   */
  constructor(registry: SkillRegistry) {
    this.registry = registry;
  }

  /**
   * Execute a skill by name.
   *
   * Loads the skill from registry and executes it with the given arguments.
   *
   * @param skillName - Name of the skill to execute
   * @param args - Template arguments
   * @returns The execution result
   * @throws Error if the skill cannot be loaded
   */
  executeByName(
    skillName: string,
    args: Record<string, string>,
  ): SkillResult {
    const skill = this.registry.getSkill(skillName);
    return this.execute(skill, args);
  }

  /**
   * Execute a skill.
   *
   * Dispatches to the appropriate execution mode based on skill metadata.
   *
   * @param skill - The skill to execute
   * @param args - Template arguments
   * @returns The execution result
   */
  execute(
    skill: Skill,
    args: Record<string, string>,
  ): SkillResult {
    const instructions = renderTemplate(skill.instructions, args);

    switch (skill.executionMode) {
      case "inline":
        return this.executeInline(skill, instructions);
      case "subagent":
        return this.executeSubagent(skill, instructions);
      case "script":
        return this.executeScript(skill, instructions);
    }
  }

  /**
   * Execute skill inline -- returns instructions for injection into the conversation.
   */
  private executeInline(skill: Skill, instructions: string): SkillResult {
    const fullInstructions =
      `## Skill: ${skill.metadata.name}\n\n${skill.metadata.description}\n\n---\n\n${instructions}`;

    return inlineResult(fullInstructions, skill.metadata.model);
  }

  /**
   * Execute skill as a subagent -- returns an agent ID; caller spawns via AgentPool.
   */
  private executeSubagent(skill: Skill, _instructions: string): SkillResult {
    const agentId = `skill-${skill.metadata.name}-${crypto.randomUUID()}`;
    return { type: "subagent", agentId };
  }

  /**
   * Execute skill as a script -- returns script content; caller executes via OrchestratorTool.
   */
  private executeScript(skill: Skill, script: string): SkillResult {
    if (
      !script.includes("let ") && !script.includes("fn ") &&
      !script.includes(";")
    ) {
      console.warn(
        `Script for skill '${skill.metadata.name}' doesn't look like valid Rhai code`,
      );
    }

    return { type: "script", output: script, isError: false };
  }

  /**
   * Filter available tool names to only those allowed by the skill.
   *
   * @param skill - The skill with optional tool restrictions
   * @param available - All available tool names
   * @returns Filtered tool names
   */
  filterAllowedTools(skill: Skill, available: string[]): string[] {
    const allowedTools = skill.metadata["allowed-tools"];
    if (!allowedTools) return [...available];

    return available.filter((name) =>
      allowedTools.some((allowed) =>
        name === allowed || name.endsWith(`__${allowed}`)
      )
    );
  }

  /**
   * Prepare a subagent execution context.
   *
   * Returns task description, filtered tool names, and system prompt.
   * Caller (who has AgentPool access) converts this into a Task + AgentContext.
   *
   * @param skill - The skill to prepare
   * @param availableToolNames - All available tool names
   * @param args - Template arguments
   * @returns Prepared subagent context
   */
  prepareSubagent(
    skill: Skill,
    availableToolNames: string[],
    args: Record<string, string>,
  ): SubagentPrepared {
    const instructions = renderTemplate(skill.instructions, args);
    const allowedToolNames = this.filterAllowedTools(skill, availableToolNames);

    const systemPrompt =
      `You are executing the '${skill.metadata.name}' skill.\n\n` +
      `**Description**: ${skill.metadata.description}\n\n` +
      `**Instructions**:\n${instructions}`;

    return {
      taskDescription: instructions,
      allowedToolNames,
      systemPrompt,
      modelOverride: skill.metadata.model,
    };
  }

  /**
   * Prepare a script execution.
   *
   * Returns the rendered script and filtered tool names.
   * Caller (who has OrchestratorTool access) handles execution.
   *
   * @param skill - The skill to prepare
   * @param availableToolNames - All available tool names
   * @param args - Template arguments
   * @returns Prepared script context
   */
  prepareScript(
    skill: Skill,
    availableToolNames: string[],
    args: Record<string, string>,
  ): ScriptPrepared {
    const scriptContent = renderTemplate(skill.instructions, args);
    const allowedToolNames = this.filterAllowedTools(skill, availableToolNames);

    return {
      scriptContent,
      allowedToolNames,
      modelOverride: skill.metadata.model,
      skillName: skill.metadata.name,
    };
  }

  /**
   * Get the execution mode for a skill.
   *
   * @param skillName - The skill name
   * @returns The execution mode
   * @throws Error if the skill is not found
   */
  getExecutionMode(skillName: string): SkillExecutionMode {
    const metadata = this.registry.getMetadata(skillName);
    if (!metadata) {
      throw new Error(`Skill not found: ${skillName}`);
    }
    return executionMode(metadata);
  }
}
