import { assertEquals } from "jsr:@std/assert";
import { DEFAULT_LIMIT, DEFAULT_MAX_FILE_SIZE, DEFAULT_MIN_SCORE } from "./mod.ts";
import type {
  AdvancedSearchRequest,
  ChunkMetadata,
  ClearResponse,
  GitSearchResult,
  IndexingMode,
  IndexRequest,
  IndexResponse,
  QueryRequest,
  QueryResponse,
  SearchGitHistoryRequest,
  SearchGitHistoryResponse,
  SearchResult,
  StatisticsResponse,
} from "./mod.ts";

// ---------------------------------------------------------------------------
// Default constants
// ---------------------------------------------------------------------------

Deno.test("DEFAULT_MAX_FILE_SIZE is 1MB", () => {
  assertEquals(DEFAULT_MAX_FILE_SIZE, 1_048_576);
});

Deno.test("DEFAULT_LIMIT is 10", () => {
  assertEquals(DEFAULT_LIMIT, 10);
});

Deno.test("DEFAULT_MIN_SCORE is 0.7", () => {
  assertEquals(DEFAULT_MIN_SCORE, 0.7);
});

// ---------------------------------------------------------------------------
// IndexRequest / IndexResponse
// ---------------------------------------------------------------------------

Deno.test("IndexRequest can be constructed with defaults", () => {
  const req: IndexRequest = {
    path: "/test",
  };
  assertEquals(req.path, "/test");
  assertEquals(req.project, undefined);
  assertEquals(req.includePatterns, undefined);
  assertEquals(req.maxFileSize, undefined);
});

Deno.test("IndexResponse full mode", () => {
  const resp: IndexResponse = {
    mode: "full",
    filesIndexed: 100,
    chunksCreated: 500,
    embeddingsGenerated: 500,
    durationMs: 1000,
    errors: [],
    filesUpdated: 0,
    filesRemoved: 0,
  };
  assertEquals(resp.mode, "full" as IndexingMode);
  assertEquals(resp.filesIndexed, 100);
});

Deno.test("IndexResponse incremental mode", () => {
  const resp: IndexResponse = {
    mode: "incremental",
    filesIndexed: 10,
    chunksCreated: 50,
    embeddingsGenerated: 50,
    durationMs: 500,
    errors: [],
    filesUpdated: 5,
    filesRemoved: 2,
  };
  assertEquals(resp.mode, "incremental" as IndexingMode);
  assertEquals(resp.filesUpdated, 5);
  assertEquals(resp.filesRemoved, 2);
});

// ---------------------------------------------------------------------------
// QueryRequest / QueryResponse / SearchResult
// ---------------------------------------------------------------------------

Deno.test("QueryRequest can be constructed", () => {
  const req: QueryRequest = {
    query: "test",
    limit: 10,
    minScore: 0.7,
    hybrid: true,
  };
  assertEquals(req.query, "test");
  assertEquals(req.hybrid, true);
});

Deno.test("SearchResult has all expected fields", () => {
  const result: SearchResult = {
    filePath: "src/main.rs",
    content: "fn main() {}",
    score: 0.95,
    vectorScore: 0.92,
    keywordScore: 0.85,
    startLine: 1,
    endLine: 10,
    language: "Rust",
    indexedAt: 0,
  };
  assertEquals(result.score, 0.95);
  assertEquals(result.vectorScore, 0.92);
  assertEquals(result.keywordScore, 0.85);
  assertEquals(result.language, "Rust");
});

Deno.test("QueryResponse with threshold info", () => {
  const resp: QueryResponse = {
    results: [],
    durationMs: 100,
    thresholdUsed: 0.7,
    thresholdLowered: false,
  };
  assertEquals(resp.thresholdUsed, 0.7);
  assertEquals(resp.thresholdLowered, false);
});

// ---------------------------------------------------------------------------
// AdvancedSearchRequest
// ---------------------------------------------------------------------------

Deno.test("AdvancedSearchRequest with filters", () => {
  const req: AdvancedSearchRequest = {
    query: "test",
    limit: 20,
    minScore: 0.8,
    fileExtensions: ["rs", "toml"],
    languages: ["Rust"],
    pathPatterns: ["src/**"],
  };
  assertEquals(req.fileExtensions?.length, 2);
  assertEquals(req.languages?.length, 1);
});

// ---------------------------------------------------------------------------
// Git history search
// ---------------------------------------------------------------------------

Deno.test("GitSearchResult has all expected fields", () => {
  const result: GitSearchResult = {
    commitHash: "abc123",
    commitMessage: "Test commit",
    author: "John Doe",
    authorEmail: "john@example.com",
    commitDate: 1234567890,
    score: 0.95,
    vectorScore: 0.92,
    keywordScore: 0.88,
    filesChanged: ["src/main.rs"],
    diffSnippet: "diff --git a/src/main.rs",
  };
  assertEquals(result.commitHash, "abc123");
  assertEquals(result.author, "John Doe");
});

Deno.test("SearchGitHistoryResponse structure", () => {
  const resp: SearchGitHistoryResponse = {
    results: [],
    commitsIndexed: 10,
    totalCachedCommits: 50,
    durationMs: 500,
  };
  assertEquals(resp.commitsIndexed, 10);
  assertEquals(resp.totalCachedCommits, 50);
});

// ---------------------------------------------------------------------------
// ChunkMetadata
// ---------------------------------------------------------------------------

Deno.test("ChunkMetadata has all expected fields", () => {
  const meta: ChunkMetadata = {
    filePath: "src/lib.rs",
    project: "test-project",
    startLine: 1,
    endLine: 50,
    language: "Rust",
    extension: "rs",
    fileHash: "abc123",
    indexedAt: 1234567890,
  };
  assertEquals(meta.startLine, 1);
  assertEquals(meta.endLine, 50);
  assertEquals(meta.fileHash, "abc123");
  assertEquals(meta.project, "test-project");
});

// ---------------------------------------------------------------------------
// ClearResponse
// ---------------------------------------------------------------------------

Deno.test("ClearResponse structure", () => {
  const resp: ClearResponse = {
    success: true,
    message: "Cleared successfully",
  };
  assertEquals(resp.success, true);
});

// ---------------------------------------------------------------------------
// StatisticsResponse
// ---------------------------------------------------------------------------

Deno.test("StatisticsResponse with language breakdown", () => {
  const stats: StatisticsResponse = {
    totalFiles: 100,
    totalChunks: 500,
    totalEmbeddings: 500,
    databaseSizeBytes: 1024 * 1024,
    languageBreakdown: [
      { language: "Rust", fileCount: 80, chunkCount: 400 },
      { language: "TOML", fileCount: 20, chunkCount: 100 },
    ],
  };
  assertEquals(stats.totalFiles, 100);
  assertEquals(stats.languageBreakdown.length, 2);
  assertEquals(stats.languageBreakdown[0].language, "Rust");
});
