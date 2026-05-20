/**
 * @module @brainwires/tools
 *
 * @deprecated Split in v0.11.0:
 *   - Tool execution framework (registry / executor / router /
 *     sanitization / OpenAPI / OAuth / validation / transaction)
 *     → `@brainwires/tool-runtime`
 *   - Built-in tools (Bash / FileOps / Git / Web / Search /
 *     SemanticSearch / Calendar / Sessions)
 *     → `@brainwires/tool-builtins`
 *
 * This barrel re-exports both for one minor version.
 */

export * from "jsr:@brainwires/tool-runtime@^0.11.0";
export * from "jsr:@brainwires/tool-builtins@^0.11.0";
