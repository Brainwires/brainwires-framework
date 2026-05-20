/**
 * @module @brainwires/agents
 *
 * @deprecated Split in v0.11.0:
 *   - Coordination primitives → `@brainwires/agent`
 *   - LLM workhorses (TaskAgent, Judge, Planner, Validator, Cycle, runtime)
 *     → `@brainwires/inference`
 *   - MAKER voting → `@brainwires/mdap`
 *   - Self-Evolving Agentic Learning → `@brainwires/seal`
 *   - SKILL.md system → `@brainwires/skills`
 *   - Evaluation harness → `@brainwires/eval`
 *
 * This barrel re-exports the new packages for one minor version. Update
 * your imports to the focused names — this barrel receives no further
 * updates, and the v0.11.0 transition keeps it pinned at v0.10.2 only
 * because TS conflicts between the underlying packages prevent a clean
 * superset re-export.
 *
 * Common migrations:
 *   import { TaskAgent } from "@brainwires/agents"
 *      → import { TaskAgent } from "@brainwires/inference"
 *
 *   import { CommunicationHub, FileLockManager } from "@brainwires/agents"
 *      → import { CommunicationHub, FileLockManager } from "@brainwires/agent"
 *
 *   import { FirstToAheadByKVoter } from "@brainwires/agents"
 *      → import { FirstToAheadByKVoter } from "@brainwires/mdap"
 *
 *   import { SkillRegistry } from "@brainwires/agents"
 *      → import { SkillRegistry } from "@brainwires/skills"
 */

export * from "jsr:@brainwires/agent@^0.11.0";
