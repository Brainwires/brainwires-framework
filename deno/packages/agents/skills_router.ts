/**
 * Skill Router
 *
 * Handles skill activation through semantic matching and keyword patterns.
 * Skills are **suggested** to the user, not auto-activated.
 *
 * ## Activation Flow
 *
 * 1. User query is analyzed against skill descriptions
 * 2. Matching skills are suggested (e.g., "Skill 'review-pr' may help")
 * 3. User explicitly invokes with `/skill-name` or `/skill <name>`
 *
 * ## Matching Methods
 *
 * - **Keyword**: Pattern matching against skill names and descriptions
 */

import {
  explicitMatch,
  keywordMatch,
  type SkillMatch,
  type SkillMetadata,
} from "./skills_metadata.ts";
import type { SkillRegistry } from "./skills_registry.ts";

/** Minimum confidence for showing skill suggestions. */
const MIN_SUGGESTION_CONFIDENCE = 0.5;

/** Keyword match confidence score. */
const KEYWORD_MATCH_CONFIDENCE = 0.6;

/**
 * Skill router for matching queries against skills.
 *
 * Performs keyword-based matching of user queries against registered skill
 * names and descriptions, returning ranked suggestions.
 */
export class SkillRouter {
  /** Reference to skill registry. */
  private registry: SkillRegistry;
  /** Minimum confidence for suggestions. */
  minConfidence: number;

  /**
   * Create a new skill router.
   *
   * @param registry - The skill registry to search
   */
  constructor(registry: SkillRegistry) {
    this.registry = registry;
    this.minConfidence = MIN_SUGGESTION_CONFIDENCE;
  }

  /**
   * Set minimum confidence threshold.
   *
   * @param confidence - New minimum confidence value
   * @returns This router for chaining
   */
  withMinConfidence(confidence: number): this {
    this.minConfidence = confidence;
    return this;
  }

  /**
   * Match query against skill descriptions.
   *
   * Returns matching skills sorted by confidence (highest first).
   *
   * @param query - The user's query string
   * @returns Array of skill matches, sorted by confidence descending
   */
  matchSkills(query: string): SkillMatch[] {
    const allMetadata = this.registry.allMetadata();

    if (allMetadata.length === 0) {
      return [];
    }

    // Use keyword matching
    const matches = this.keywordMatchInternal(query, allMetadata);

    // Filter by minimum confidence
    const filtered = matches.filter((m) => m.confidence >= this.minConfidence);

    // Sort by confidence (highest first)
    filtered.sort((a, b) => b.confidence - a.confidence);

    return filtered;
  }

  /**
   * Keyword-based fallback matching.
   *
   * Matches query words against skill names and descriptions.
   *
   * @param query - The user's query
   * @param metadata - Array of skill metadata to match against
   * @returns Array of keyword matches
   */
  private keywordMatchInternal(
    query: string,
    metadata: SkillMetadata[],
  ): SkillMatch[] {
    const queryLower = query.toLowerCase();
    const queryWords = new Set(
      queryLower.split(/\s+/).filter((w) => w.length > 2),
    );

    if (queryWords.size === 0) {
      return [];
    }

    const results: SkillMatch[] = [];

    for (const m of metadata) {
      const nameLower = m.name.toLowerCase();
      const descLower = m.description.toLowerCase();

      let matchCount = 0;

      // Check name match (higher weight)
      if (queryLower.includes(nameLower) || nameLower.includes(queryLower)) {
        matchCount += 3;
      }

      // Check individual word matches in description
      for (const word of queryWords) {
        if (descLower.includes(word)) {
          matchCount += 1;
        }
      }

      // Check for skill name words in query
      const nameWords = nameLower.split("-");
      for (const nameWord of nameWords) {
        if (queryWords.has(nameWord)) {
          matchCount += 2;
        }
      }

      if (matchCount > 0) {
        const confidence = Math.min(
          KEYWORD_MATCH_CONFIDENCE + matchCount * 0.05,
          0.9,
        );
        results.push(keywordMatch(m.name, confidence));
      }
    }

    return results;
  }

  /**
   * Format skill suggestions for display.
   *
   * Returns undefined if no skills match, otherwise returns a formatted suggestion message.
   *
   * @param matches - Array of skill matches
   * @returns Formatted suggestion string, or undefined
   */
  formatSuggestions(matches: SkillMatch[]): string | undefined {
    if (matches.length === 0) {
      return undefined;
    }

    const suggestions = matches
      .slice(0, 3) // Limit to top 3
      .map((m) => `\`/${m.skillName}\``);

    const skillWord = suggestions.length === 1 ? "skill" : "skills";

    return `The ${skillWord} ${suggestions.join(", ")} may help. Use the command to activate.`;
  }

  /**
   * Check if a skill exists by name.
   *
   * @param name - The skill name to check
   * @returns True if the skill exists
   */
  skillExists(name: string): boolean {
    return this.registry.contains(name);
  }

  /**
   * Get an explicit match for a skill name.
   *
   * Used when user directly invokes `/skill-name`.
   *
   * @param skillName - The skill name
   * @returns An explicit SkillMatch with confidence 1.0
   */
  explicitMatch(skillName: string): SkillMatch {
    return explicitMatch(skillName);
  }
}
