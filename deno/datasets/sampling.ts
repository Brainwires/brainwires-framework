/**
 * Train/eval splitting, curriculum ordering, and sampling utilities.
 * Equivalent to Rust's `brainwires_datasets::sampling` module.
 */

import type { TrainingExample } from "./types.ts";
import { exampleTokens } from "./types.ts";

// PCG constants (matching the Rust implementation)
const PCG_MULTIPLIER = 6_364_136_223_846_793_005n;
const PCG_INCREMENT = 1_442_695_040_888_963_407n;

/** Configuration for train/eval splitting. */
export interface SplitConfig {
  /** Fraction of data for training (0.0 - 1.0). Default 0.9. */
  trainRatio: number;
  /** Random seed for reproducible splits. Default 42. */
  seed: number;
  /** Whether to shuffle before splitting. Default true. */
  shuffle: boolean;
}

/** Default split configuration. */
export function defaultSplitConfig(): SplitConfig {
  return { trainRatio: 0.9, seed: 42, shuffle: true };
}

/** Result of a train/eval split. */
export interface SplitResult<T> {
  /** The training split. */
  train: T[];
  /** The evaluation split. */
  eval: T[];
}

/**
 * Split examples into train/eval sets.
 *
 * Uses a deterministic shuffle when config.shuffle is true.
 */
export function trainEvalSplit<T>(
  examples: T[],
  config?: Partial<SplitConfig>,
): SplitResult<T> {
  const cfg: SplitConfig = { ...defaultSplitConfig(), ...config };
  let items = [...examples];

  if (cfg.shuffle) {
    items = deterministicShuffle(items, cfg.seed);
  }

  const splitIdx = Math.round(items.length * cfg.trainRatio);
  return {
    train: items.slice(0, splitIdx),
    eval: items.slice(splitIdx),
  };
}

/**
 * Sort examples by estimated token count (ascending) for curriculum learning.
 * Shorter examples first, building up to longer ones.
 */
export function curriculumOrder(examples: TrainingExample[]): TrainingExample[] {
  return [...examples].sort(
    (a, b) => exampleTokens(a) - exampleTokens(b),
  );
}

/**
 * Sample `n` examples uniformly using a deterministic seed.
 * If n >= examples.length, returns all examples.
 */
export function sampleN<T>(items: T[], n: number, seed: number = 42): T[] {
  if (n >= items.length) return [...items];

  // Fisher-Yates partial shuffle (matching Rust implementation)
  const indices = Array.from({ length: items.length }, (_, i) => i);
  let state = BigInt(seed);

  for (let i = 0; i < n; i++) {
    state = (state * PCG_MULTIPLIER + PCG_INCREMENT) & 0xFFFFFFFFFFFFFFFFn;
    const shifted = Number(state >> 33n);
    const j = i + (shifted % (items.length - i));
    [indices[i], indices[j]] = [indices[j], indices[i]];
  }

  return indices.slice(0, n).map((idx) => items[idx]);
}

// -- Internal -----------------------------------------------------------------

function deterministicShuffle<T>(items: T[], seed: number): T[] {
  const result = [...items];
  let state = BigInt(seed);

  for (let i = result.length - 1; i > 0; i--) {
    state = (state * PCG_MULTIPLIER + PCG_INCREMENT) & 0xFFFFFFFFFFFFFFFFn;
    const shifted = Number(state >> 33n);
    const j = shifted % (i + 1);
    [result[i], result[j]] = [result[j], result[i]];
  }

  return result;
}
