/**
 * @module @brainwires/tools
 *
 * @deprecated Renamed/split in v0.11.0. Use `@brainwires/tool-runtime` for
 * the registry / executor / sanitization framework and
 * `@brainwires/tool-builtins` for BashTool / FileOpsTool / GitTool /
 * WebTool / SearchTool / SemanticSearchTool / CalendarTool / SessionsTool.
 *
 * This barrel re-exports both packages during the 0.11.x window and will be
 * removed in 0.12.0. The directory remains on `main` for the transitional
 * publish; a `0.10.2` tombstone of the same name is also published from a
 * release branch for consumers pinned to `^0.10.x`.
 */

export * from "@brainwires/tool-runtime";
export * from "@brainwires/tool-builtins";
