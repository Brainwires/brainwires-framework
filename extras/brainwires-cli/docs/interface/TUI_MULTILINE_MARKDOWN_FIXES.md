# TUI Multi-line Input and Markdown Rendering Fixes

## Overview
This document describes the fixes applied to enable proper multi-line input support and markdown rendering in the brainwires-cli TUI chat interface.

## Issues Fixed

### 1. Multi-line Input Key Bindings
**Problem:** The `is_enter()` function was matching Enter key presses regardless of modifiers, and many terminals don't properly send Shift+Enter combinations.

**Solution:** Modified key handling in [src/tui/events.rs:148-178](../src/tui/events.rs) to support multiple key combinations:
- Alt+Enter (most reliable across terminals)
- Ctrl+J (common terminal convention)
- Shift+Enter (if the terminal supports it)

```rust
// Enter without modifiers (for submitting)
pub fn is_enter(&self) -> bool {
    matches!(
        self,
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers,
            ..
        }) if !modifiers.contains(KeyModifiers::SHIFT) && !modifiers.contains(KeyModifiers::ALT)
    )
}

// Multi-line input (multiple key combinations for compatibility)
pub fn is_shift_enter(&self) -> bool {
    matches!(
        self,
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers,
            ..
        }) if modifiers.contains(KeyModifiers::SHIFT) || modifiers.contains(KeyModifiers::ALT)
    ) || matches!(
        self,
        Event::Key(KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::CONTROL,
            ..
        })
    )
}
```

### 2. Input Frame Height Not Growing with Content
**Problem:** The input frame had a fixed height of 3 lines, preventing users from seeing multiple lines of text they were typing.

**Additional Issue:** Using `lines().count()` doesn't count trailing newlines, so pressing Alt+Enter wouldn't increase the height until typing the first character on the new line.

**Solution:** Implemented dynamic height calculation in [src/tui/ui.rs:58-67](../src/tui/ui.rs):
- Counts newline characters directly (newlines + 1 = line count)
- Calculates needed height (lines + 2 for borders)
- Caps at 35% of screen height to prevent overwhelming the UI
- Minimum height of 3 lines for usability

```rust
// Calculate dynamic input height based on content
// Note: We count newlines instead of using lines() because lines() doesn't count trailing newlines
let newline_count = app.input.chars().filter(|&c| c == '\n').count();
let input_lines = (newline_count + 1).max(1) as u16;
let input_height_needed = input_lines + 2; // +2 for borders

// Cap at 35% of screen height
let max_input_height = (f.area().height * 35) / 100;
let input_height = input_height_needed.min(max_input_height).max(3);
```

### 3. Cursor Positioning and Scrolling
**Problem:** When input exceeded the visible area, the cursor position wasn't updated correctly.

**Solution:** Added scroll calculation in [src/tui/ui.rs:746-780](../src/tui/ui.rs):
- Tracks cursor line position within multi-line input
- Calculates scroll offset to keep cursor visible
- Adjusts cursor position accounting for scroll offset

```rust
// Calculate scroll offset to keep cursor visible
let cursor_line = (lines_before.len() as u16).saturating_sub(1);
let available_height = area.height.saturating_sub(2);

let scroll_offset = if cursor_line >= available_height {
    cursor_line.saturating_sub(available_height - 1)
} else {
    0
};

// Apply scroll to paragraph
.scroll((scroll_offset, 0))

// Adjust cursor position for scroll
let visible_line = (line_count as u16).saturating_sub(1).saturating_sub(scroll_offset);
```

### 4. Markdown Rendering
**Enhancement:** Added markdown rendering for conversation messages to improve readability.

**Solution:** Integrated `termimad` crate in [src/tui/ui.rs:16-43](../src/tui/ui.rs):
- Added `termimad` dependency to Cargo.toml
- Created `render_markdown_to_lines()` function to convert markdown to styled ratatui Lines
- Configured color scheme for markdown elements:
  - Headers: Cyan (H1), Blue (H2)
  - Bold: Yellow
  - Italic: Magenta
  - Inline code and code blocks: Green

```rust
fn render_markdown_to_lines(markdown: &str) -> Vec<Line<'static>> {
    let mut skin = MadSkin::default();

    skin.headers[0].set_fg(TermColor::Cyan);
    skin.headers[1].set_fg(TermColor::Blue);
    skin.bold.set_fg(TermColor::Yellow);
    skin.italic.set_fg(TermColor::Magenta);
    skin.inline_code.set_fg(TermColor::Green);
    skin.code_block.set_fg(TermColor::Green);

    // ... render and convert to Lines
}
```

## Files Modified

1. **Cargo.toml**
   - Added `termimad = "0.30"` dependency

2. **src/tui/events.rs**
   - Fixed `is_enter()` to exclude SHIFT modifier (line 148-158)

3. **src/tui/ui.rs**
   - Added markdown rendering function (line 16-43)
   - Implemented dynamic input height calculation (line 58-76)
   - Updated conversation rendering to use markdown (line 476-483)
   - Added scroll support for input (line 746-780)
   - Enhanced cursor positioning for multi-line (line 768-781)

## Testing

Run the test script to verify all features:

```bash
./test_tui_markdown.sh
```

### Manual Test Cases

1. **Multi-line Input:**
   - Type some text
   - Press Shift+Enter multiple times
   - Verify input box grows to show all lines
   - Verify input stops growing at 35% screen height
   - Verify scrolling works when exceeding max height
   - Press Enter alone to submit

2. **Markdown Formatting:**
   - Send a message with markdown syntax:
     ```
     # Heading
     **bold** and *italic*
     `code`
     - List item
     ```
   - Verify formatting is displayed with colors
   - Verify assistant responses also render markdown

3. **Cursor Behavior:**
   - Type multi-line text
   - Use arrow keys to navigate
   - Verify cursor position updates correctly
   - Verify cursor stays visible when scrolling

## Key Behavior

- **Alt+Enter or Ctrl+J:** Adds a newline to current input (Shift+Enter also works if your terminal supports it)
- **Enter:** Submits the message
- **Input Height:** Dynamically grows from 3 lines up to 35% of screen height
- **Scrolling:** Automatic when content exceeds visible area
- **Markdown:** Both user and assistant messages render with formatting

## Why Alt+Enter and Ctrl+J?

Many terminal emulators don't properly send the Shift modifier with the Enter key due to historical reasons and terminal protocol limitations. To ensure reliable multi-line input across all terminals, we support:

1. **Alt+Enter** - Most reliable across modern terminals
2. **Ctrl+J** - Traditional Unix/terminal convention for line feed
3. **Shift+Enter** - Works in terminals that properly support it (like some modern terminal emulators)

## Benefits

1. **Better UX:** Users can compose longer, more complex messages
2. **Visual Feedback:** See exactly what they're typing across multiple lines
3. **Readable Conversations:** Markdown formatting makes conversations easier to read
4. **Professional Appearance:** Code blocks, headers, and formatting look great in terminal
