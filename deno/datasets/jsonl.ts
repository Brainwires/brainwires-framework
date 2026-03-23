/**
 * JSONL reader and writer for streaming I/O.
 * Equivalent to Rust's `brainwires_datasets::jsonl` module.
 */

import type { PreferencePair, TrainingExample } from "./types.ts";

// -- Reading ------------------------------------------------------------------

/**
 * Streaming JSONL reader -- memory-efficient, reads one line at a time.
 * Implements the async iterable protocol for use with `for await`.
 */
export class JsonlReader<T = TrainingExample> {
  #lines: string[];
  #index: number;
  #lineNumber: number;

  constructor(text: string) {
    this.#lines = text.split("\n");
    this.#index = 0;
    this.#lineNumber = 0;
  }

  /** Read the next item from the JSONL stream. */
  next(): T | null {
    while (this.#index < this.#lines.length) {
      const line = this.#lines[this.#index++];
      this.#lineNumber++;
      const trimmed = line.trim();
      if (trimmed === "") continue;
      try {
        return JSON.parse(trimmed) as T;
      } catch (e) {
        throw new Error(
          `JSONL parse error at line ${this.#lineNumber}: ${
            e instanceof Error ? e.message : String(e)
          }`,
        );
      }
    }
    return null;
  }

  /** Read all items into an array. */
  readAll(): T[] {
    const items: T[] = [];
    let item: T | null;
    while ((item = this.next()) !== null) {
      items.push(item);
    }
    return items;
  }

  /** Current line number (1-based). */
  get lineNumber(): number {
    return this.#lineNumber;
  }

  /** Async iterable interface. */
  async *[Symbol.asyncIterator](): AsyncIterableIterator<T> {
    let item: T | null;
    while ((item = this.next()) !== null) {
      yield item;
    }
  }

  /** Sync iterable interface. */
  *[Symbol.iterator](): IterableIterator<T> {
    let item: T | null;
    while ((item = this.next()) !== null) {
      yield item;
    }
  }
}

/**
 * Convenience: read all training examples from a JSONL string.
 */
export function readJsonl<T = TrainingExample>(text: string): T[] {
  return new JsonlReader<T>(text).readAll();
}

/**
 * Convenience: read all training examples from a JSONL file path.
 */
export async function readJsonlFile<T = TrainingExample>(
  path: string,
): Promise<T[]> {
  const text = await Deno.readTextFile(path);
  return readJsonl<T>(text);
}

// -- Writing ------------------------------------------------------------------

/**
 * Buffered JSONL writer. Collects lines and can flush to string or file.
 */
export class JsonlWriter<T = TrainingExample> {
  #lines: string[];
  #count: number;

  constructor() {
    this.#lines = [];
    this.#count = 0;
  }

  /** Write a single item as a JSONL line. */
  write(item: T): void {
    this.#lines.push(JSON.stringify(item));
    this.#count++;
  }

  /** Write multiple items. */
  writeAll(items: T[]): void {
    for (const item of items) {
      this.write(item);
    }
  }

  /** Number of items written. */
  get count(): number {
    return this.#count;
  }

  /** Get the accumulated JSONL string. */
  toString(): string {
    return this.#lines.join("\n") + (this.#lines.length > 0 ? "\n" : "");
  }

  /** Write the accumulated JSONL to a file. */
  async toFile(path: string): Promise<number> {
    await Deno.writeTextFile(path, this.toString());
    return this.#count;
  }
}

/**
 * Convenience: serialize training examples to a JSONL string.
 */
export function writeJsonl<T = TrainingExample>(items: T[]): string {
  const writer = new JsonlWriter<T>();
  writer.writeAll(items);
  return writer.toString();
}

/**
 * Convenience: write training examples to a JSONL file.
 */
export async function writeJsonlFile<T = TrainingExample>(
  path: string,
  items: T[],
): Promise<number> {
  const writer = new JsonlWriter<T>();
  writer.writeAll(items);
  return await writer.toFile(path);
}

/**
 * Convenience: write preference pairs to a JSONL string.
 */
export function writeJsonlPreferences(pairs: PreferencePair[]): string {
  return writeJsonl(pairs);
}

/**
 * Convenience: read preference pairs from a JSONL string.
 */
export function readJsonlPreferences(text: string): PreferencePair[] {
  return readJsonl<PreferencePair>(text);
}
