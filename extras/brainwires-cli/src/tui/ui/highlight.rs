//! Search Highlight Utilities
//!
//! Provides functions to apply search match highlighting to rendered text.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Style for non-current search matches (yellow background)
pub fn match_style() -> Style {
    Style::default().bg(Color::Yellow).fg(Color::Black)
}

/// Style for the current/selected search match (green background)
pub fn current_match_style() -> Style {
    Style::default()
        .bg(Color::Green)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

/// Apply search highlighting to a vector of Lines by searching within the rendered text.
///
/// This function extracts plain text from the spans, finds matches of the search query,
/// and applies highlight styling to matching regions.
///
/// # Arguments
/// * `query` - The search query (plain text, not regex)
/// * `case_sensitive` - Whether the search is case-sensitive
/// * `current_match_index` - Which match is the "current" one (highlighted differently)
/// * `lines` - The rendered lines to apply highlighting to
///
/// # Returns
/// Tuple of (highlighted lines, total match count, matches with positions for scrolling)
pub fn apply_highlights_to_lines(
    query: &str,
    case_sensitive: bool,
    current_match_index: usize,
    lines: Vec<Line<'static>>,
) -> (Vec<Line<'static>>, usize, Vec<usize>) {
    if query.is_empty() {
        return (lines, 0, vec![]);
    }

    // First pass: find all matches and their line numbers
    let mut match_line_numbers: Vec<usize> = Vec::new();
    let mut total_matches = 0;

    // Track byte offset across all lines for matching
    let query_lower = query.to_lowercase();

    for (line_idx, line) in lines.iter().enumerate() {
        // Extract plain text from this line
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        // Count matches in this line
        let search_text = if case_sensitive {
            line_text.clone()
        } else {
            line_text.to_lowercase()
        };

        let search_query = if case_sensitive { query } else { &query_lower };

        let mut pos = 0;
        while let Some(found_pos) = search_text[pos..].find(search_query) {
            match_line_numbers.push(line_idx);
            total_matches += 1;
            pos = pos + found_pos + search_query.len();
        }
    }

    if total_matches == 0 {
        return (lines, 0, match_line_numbers);
    }

    // Second pass: apply highlighting
    let mut result_lines = Vec::with_capacity(lines.len());
    let mut match_counter = 0;

    for line in lines {
        // Extract plain text from this line
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        // Find all match positions in this line
        let search_text = if case_sensitive {
            line_text.clone()
        } else {
            line_text.to_lowercase()
        };

        let search_query = if case_sensitive { query } else { &query_lower };

        let mut match_positions: Vec<(usize, usize, bool)> = Vec::new(); // (start, end, is_current)
        let mut pos = 0;
        while let Some(found_pos) = search_text[pos..].find(search_query) {
            let actual_pos = pos + found_pos;
            let is_current = match_counter == current_match_index;
            match_positions.push((actual_pos, actual_pos + query.len(), is_current));
            match_counter += 1;
            pos = actual_pos + search_query.len();
        }

        if match_positions.is_empty() {
            // No matches in this line
            result_lines.push(line);
        } else {
            // Apply highlighting to this line
            let highlighted_line = highlight_line_with_positions(&line, &match_positions);
            result_lines.push(highlighted_line);
        }
    }

    (result_lines, total_matches, match_line_numbers)
}

/// Apply highlighting to a single line at the given character positions
fn highlight_line_with_positions(
    line: &Line<'static>,
    match_positions: &[(usize, usize, bool)], // (start_char, end_char, is_current)
) -> Line<'static> {
    let mut new_spans: Vec<Span<'static>> = Vec::new();
    let mut char_offset = 0;

    for span in &line.spans {
        let span_text: String = span.content.to_string();
        let span_style = span.style;
        let span_char_len = span_text.chars().count();
        let span_start = char_offset;
        let span_end = span_start + span_char_len;

        // Find matches that overlap with this span
        let mut overlapping: Vec<(usize, usize, bool)> = Vec::new();

        for &(match_start, match_end, is_current) in match_positions {
            if match_start < span_end && match_end > span_start {
                // Calculate overlap within the span (in characters)
                let overlap_start = match_start.saturating_sub(span_start);
                let overlap_end = (match_end - span_start).min(span_char_len);
                overlapping.push((overlap_start, overlap_end, is_current));
            }
        }

        if overlapping.is_empty() {
            new_spans.push(Span::styled(span_text, span_style));
        } else {
            // Sort by start position
            overlapping.sort_by_key(|&(start, _, _)| start);

            // Merge overlapping regions
            let mut merged: Vec<(usize, usize, bool)> = Vec::new();
            for (start, end, is_current) in overlapping {
                if let Some(last) = merged.last_mut()
                    && start <= last.1
                {
                    last.1 = last.1.max(end);
                    last.2 = last.2 || is_current;
                    continue;
                }
                merged.push((start, end, is_current));
            }

            // Split the span based on highlighted regions
            // Need to work with char indices, not byte indices
            let chars: Vec<char> = span_text.chars().collect();
            let mut char_pos = 0;

            for (hl_start, hl_end, is_current) in merged {
                // Non-highlighted portion before
                if char_pos < hl_start {
                    let unhighlighted: String = chars[char_pos..hl_start].iter().collect();
                    if !unhighlighted.is_empty() {
                        new_spans.push(Span::styled(unhighlighted, span_style));
                    }
                }

                // Highlighted portion
                let hl_end_clamped = hl_end.min(span_char_len);
                let hl_start_clamped = hl_start.min(span_char_len);
                if hl_start_clamped < hl_end_clamped {
                    let highlighted: String =
                        chars[hl_start_clamped..hl_end_clamped].iter().collect();
                    let hl_style = if is_current {
                        current_match_style()
                    } else {
                        match_style()
                    };
                    new_spans.push(Span::styled(highlighted, hl_style));
                }

                char_pos = hl_end_clamped;
            }

            // Remaining non-highlighted portion
            if char_pos < span_char_len {
                let remaining: String = chars[char_pos..].iter().collect();
                if !remaining.is_empty() {
                    new_spans.push(Span::styled(remaining, span_style));
                }
            }
        }

        char_offset = span_end;
    }

    Line::from(new_spans)
}

/// Apply search highlighting to input text, returning styled Lines.
///
/// # Arguments
/// * `text` - The input text to highlight
/// * `query` - The search query
/// * `case_sensitive` - Whether the search is case-sensitive
/// * `current_match_index` - Which match is the "current" one
///
/// # Returns
/// Tuple of (highlighted lines, total match count, line numbers with matches)
pub fn highlight_input_text(
    text: &str,
    query: &str,
    case_sensitive: bool,
    current_match_index: usize,
) -> (Vec<Line<'static>>, usize, Vec<usize>) {
    if query.is_empty() {
        return (
            text.lines().map(|l| Line::from(l.to_string())).collect(),
            0,
            vec![],
        );
    }

    let query_lower = query.to_lowercase();
    let mut result_lines = Vec::new();
    let mut match_line_numbers: Vec<usize> = Vec::new();
    let mut match_counter = 0;

    for (line_idx, line_text) in text.split('\n').enumerate() {
        let search_text = if case_sensitive {
            line_text.to_string()
        } else {
            line_text.to_lowercase()
        };

        let search_query = if case_sensitive { query } else { &query_lower };

        // Find all match positions in this line
        let mut match_positions: Vec<(usize, usize, bool)> = Vec::new();
        let mut pos = 0;
        while let Some(found_pos) = search_text[pos..].find(search_query) {
            let actual_pos = pos + found_pos;
            let is_current = match_counter == current_match_index;
            match_positions.push((actual_pos, actual_pos + query.len(), is_current));
            match_line_numbers.push(line_idx);
            match_counter += 1;
            pos = actual_pos + search_query.len();
        }

        if match_positions.is_empty() {
            result_lines.push(Line::from(line_text.to_string()));
        } else {
            // Build spans for this line
            let chars: Vec<char> = line_text.chars().collect();
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut char_pos = 0;

            for (hl_start, hl_end, is_current) in &match_positions {
                // Non-highlighted portion
                if char_pos < *hl_start {
                    let unhighlighted: String = chars[char_pos..*hl_start].iter().collect();
                    spans.push(Span::raw(unhighlighted));
                }

                // Highlighted portion
                let hl_style = if *is_current {
                    current_match_style()
                } else {
                    match_style()
                };
                let highlighted: String = chars[*hl_start..*hl_end].iter().collect();
                spans.push(Span::styled(highlighted, hl_style));

                char_pos = *hl_end;
            }

            // Remaining portion
            if char_pos < chars.len() {
                let remaining: String = chars[char_pos..].iter().collect();
                spans.push(Span::raw(remaining));
            }

            result_lines.push(Line::from(spans));
        }
    }

    let total_matches = match_counter;
    (result_lines, total_matches, match_line_numbers)
}
