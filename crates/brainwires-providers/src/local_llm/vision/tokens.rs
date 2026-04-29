//! Image-token splicing helpers.
//!
//! Gemma 3/4 represent image attachments inline in the token stream:
//!
//! ```text
//! \n <start_of_image> [256 image-token placeholders] <end_of_image> \n
//! ```
//!
//! The vocabulary IDs come from the Gemma 3/4 tokenizer:
//!   - `\n` = 108
//!   - `<start_of_image>` = 255999
//!   - `<end_of_image>`   = 256000
//!
//! The 256 placeholder slots are filled at decode time by the projector's
//! output (Stage C) — at the token level we just reserve space with the
//! `<start_of_image>` token id (any non-special id works since the
//! decoder swaps in soft embeddings, but using SOI id keeps parsers happy
//! and simplifies bounded-search for the splice point).

/// Vocabulary id for `\n` in the Gemma 3/4 tokenizer.
pub const TOKEN_NEWLINE: u32 = 108;

/// Vocabulary id for `<start_of_image>`.
pub const TOKEN_START_OF_IMAGE: u32 = 255_999;

/// Vocabulary id for `<end_of_image>`.
pub const TOKEN_END_OF_IMAGE: u32 = 256_000;

/// Number of soft image tokens emitted by the SigLIP encoder per single-crop image
/// (paligemma_3b_896: 64×64 patches → 256 patches → 256 image tokens).
pub const GEMMA_IMAGE_TOKEN_COUNT: usize = 256;

/// Splice an image-token block at the given position in `tokens`.
/// Inserts `\n <SOI> [256 × placeholder] <EOI> \n` and returns the
/// `(start, end)` range of the 256 placeholders so callers can later
/// overwrite their embeddings with the projector's output.
///
/// `placeholder` defaults to `TOKEN_START_OF_IMAGE`.
///
/// `insert_at` is clamped to `tokens.len()` if out of bounds, so this
/// helper never panics on bad input.
pub fn splice_image_token_block(
    tokens: &mut Vec<u32>,
    insert_at: usize,
    placeholder: Option<u32>,
) -> std::ops::Range<usize> {
    let insert_at = insert_at.min(tokens.len());
    let placeholder = placeholder.unwrap_or(TOKEN_START_OF_IMAGE);

    // Block layout: NL, SOI, placeholder × 256, EOI, NL  →  total 260 tokens.
    let mut block: Vec<u32> = Vec::with_capacity(GEMMA_IMAGE_TOKEN_COUNT + 4);
    block.push(TOKEN_NEWLINE);
    block.push(TOKEN_START_OF_IMAGE);
    block.extend(std::iter::repeat_n(placeholder, GEMMA_IMAGE_TOKEN_COUNT));
    block.push(TOKEN_END_OF_IMAGE);
    block.push(TOKEN_NEWLINE);

    tokens.splice(insert_at..insert_at, block);

    // Placeholder slots sit after the leading `\n SOI` (2 tokens).
    let start = insert_at + 2;
    start..start + GEMMA_IMAGE_TOKEN_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(GEMMA_IMAGE_TOKEN_COUNT, 256);
        assert_eq!(TOKEN_START_OF_IMAGE, 255_999);
        assert_eq!(TOKEN_END_OF_IMAGE, 256_000);
        assert_eq!(TOKEN_NEWLINE, 108);
    }

    #[test]
    fn test_splice_inserts_at_position_zero() {
        let mut tokens: Vec<u32> = Vec::new();
        let range = splice_image_token_block(&mut tokens, 0, None);
        assert_eq!(range, 2..258);
        assert_eq!(tokens.len(), 260);
        assert_eq!(tokens[0], TOKEN_NEWLINE);
        assert_eq!(tokens[1], TOKEN_START_OF_IMAGE);
        for i in 2..258 {
            assert_eq!(tokens[i], TOKEN_START_OF_IMAGE);
        }
        assert_eq!(tokens[258], TOKEN_END_OF_IMAGE);
        assert_eq!(tokens[259], TOKEN_NEWLINE);
    }

    #[test]
    fn test_splice_at_middle_preserves_tail() {
        let mut tokens: Vec<u32> = vec![1, 2, 3];
        let range = splice_image_token_block(&mut tokens, 1, None);
        assert_eq!(range, 3..259);
        assert_eq!(tokens.len(), 263);
        // Head preserved.
        assert_eq!(tokens[0], 1);
        // Block follows.
        assert_eq!(tokens[1], TOKEN_NEWLINE);
        assert_eq!(tokens[2], TOKEN_START_OF_IMAGE);
        assert_eq!(tokens[259], TOKEN_END_OF_IMAGE);
        assert_eq!(tokens[260], TOKEN_NEWLINE);
        // Tail preserved at shifted positions.
        assert_eq!(tokens[261], 2);
        assert_eq!(tokens[262], 3);
    }

    #[test]
    fn test_splice_clamps_out_of_bounds_insert_at() {
        let mut tokens: Vec<u32> = vec![1, 2];
        let range = splice_image_token_block(&mut tokens, 99, None);
        assert_eq!(range, 4..260);
        assert_eq!(tokens.len(), 262);
        assert_eq!(tokens[0], 1);
        assert_eq!(tokens[1], 2);
        assert_eq!(tokens[2], TOKEN_NEWLINE);
        assert_eq!(tokens[3], TOKEN_START_OF_IMAGE);
        assert_eq!(tokens[260], TOKEN_END_OF_IMAGE);
        assert_eq!(tokens[261], TOKEN_NEWLINE);
    }

    #[test]
    fn test_splice_with_custom_placeholder() {
        let mut tokens: Vec<u32> = Vec::new();
        let range = splice_image_token_block(&mut tokens, 0, Some(42));
        for i in range {
            assert_eq!(tokens[i], 42);
        }
    }
}
