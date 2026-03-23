/**
 * @module @brainwires/tool-system
 *
 * Built-in tool implementations for the Brainwires Agent Framework.
 * Equivalent to Rust's `brainwires-tool-system` crate.
 *
 * ## Always Available
 * - **BashTool** - Shell command execution with proactive output management
 * - **FileOpsTool** - File read/write/edit/list/search/delete/create_directory
 * - **GitTool** - Git operations (status, diff, log, stage, commit, push, pull, etc.)
 * - **WebTool** - URL fetching
 * - **SearchTool** - Regex-based code search (respects .gitignore)
 *
 * ## Registry
 * The `ToolRegistry` is a composable container. Create one and register
 * whichever tools you need.
 *
 * ```ts
 * import { ToolRegistry, BashTool, FileOpsTool } from "@brainwires/tool-system";
 *
 * const registry = new ToolRegistry();
 * registry.registerTools(BashTool.getTools());
 * registry.registerTools(FileOpsTool.getTools());
 * ```
 */

// Error taxonomy and classification
export {
  categoryName,
  classifyError,
  defaultRetryStrategy,
  delayForAttempt,
  errorMessage,
  failureOutcome,
  getSuggestion,
  isRetryable,
  maxAttempts,
  retryStrategy,
  successOutcome,
  type ResourceType,
  type RetryStrategy,
  type ToolErrorCategory,
  type ToolOutcome,
} from "./error.ts";

// Tool executor interface
export {
  allow,
  reject,
  type PreHookDecision,
  type ToolExecutor,
  type ToolPreHook,
} from "./executor.ts";

// Tool registry
export { ToolRegistry, type ToolCategory } from "./registry.ts";

// Sanitization
export {
  containsSensitiveData,
  filterToolOutput,
  isInjectionAttempt,
  redactSensitiveData,
  sanitizeExternalContent,
  wrapWithContentSource,
  type ContentSource,
} from "./sanitization.ts";

// Smart router
export {
  analyzeMessages,
  analyzeQuery,
  getContextForAnalysis,
  getSmartTools,
  getSmartToolsWithMcp,
  getToolsForCategories,
} from "./smart_router.ts";

// Transaction manager
export { TransactionManager } from "./transaction.ts";

// Built-in tools
export {
  BashTool,
  FileOpsTool,
  GitTool,
  SearchTool,
  ValidationTool,
  WebTool,
} from "./tools/mod.ts";
export { extractExportName, isExportLine } from "./tools/mod.ts";
export type { OutputMode, StderrMode } from "./tools/mod.ts";

// OpenAPI tool generation
export {
  executeOpenApiTool,
  executeOpenApiToolWithEndpoint,
  openApiToToolDefs,
  openApiToTools,
} from "./tools/mod.ts";
export type {
  HttpMethod,
  OpenApiEndpoint,
  OpenApiParam,
  OpenApiToolDef,
} from "./tools/mod.ts";
