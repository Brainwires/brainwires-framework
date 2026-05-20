/**
 * @module @brainwires/providers
 *
 * @deprecated Renamed in v0.11.0:
 *   - LLM chat providers → `@brainwires/provider`
 *   - Speech (TTS/STT/ASR) clients → `@brainwires/provider-speech`
 *
 * This package re-exports the new ones for one minor version. Update
 * your imports — this name receives no further updates.
 */

export * from "jsr:@brainwires/provider@^0.11.0";
export * as speech from "jsr:@brainwires/provider-speech@^0.11.0";
