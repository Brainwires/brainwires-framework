//! Exit/Background dialog state management.
//!
//! This module contains the state and logic for the Ctrl+C exit dialog,
//! which allows users to either exit the application or background it.

use ratatui::layout::Rect;
use ratatui_interact::components::CheckBoxState;

/// Available actions in the exit dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitAction {
    /// Exit the application completely.
    Exit,
    /// Keep process running, restore terminal, return control to shell.
    /// Agents continue working in the background.
    Background,
}

/// Focusable elements in the exit dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitFocus {
    /// The "Exit" button (default focus)
    #[default]
    ExitButton,
    /// The "Background" button
    BackgroundButton,
    /// The "Preserve chat" checkbox (keep chat visible instead of restoring previous terminal)
    PreserveChatCheckbox,
    /// The "Exit when done" checkbox (only shown when Background is an option)
    ExitWhenDoneCheckbox,
}

impl ExitFocus {
    /// Move to next focusable element.
    pub fn next(&self) -> Self {
        match self {
            ExitFocus::ExitButton => ExitFocus::BackgroundButton,
            ExitFocus::BackgroundButton => ExitFocus::PreserveChatCheckbox,
            ExitFocus::PreserveChatCheckbox => ExitFocus::ExitWhenDoneCheckbox,
            ExitFocus::ExitWhenDoneCheckbox => ExitFocus::ExitButton,
        }
    }

    /// Move to previous focusable element.
    pub fn prev(&self) -> Self {
        match self {
            ExitFocus::ExitButton => ExitFocus::ExitWhenDoneCheckbox,
            ExitFocus::BackgroundButton => ExitFocus::ExitButton,
            ExitFocus::PreserveChatCheckbox => ExitFocus::BackgroundButton,
            ExitFocus::ExitWhenDoneCheckbox => ExitFocus::PreserveChatCheckbox,
        }
    }
}

/// Clickable region for mouse interaction.
#[derive(Debug, Clone)]
pub struct ClickRegion {
    /// The area this click region covers
    pub area: Rect,
    /// The element this region corresponds to
    pub element: ExitFocus,
}

impl ClickRegion {
    /// Check if a point is within this click region.
    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.area.x
            && col < self.area.x + self.area.width
            && row >= self.area.y
            && row < self.area.y + self.area.height
    }
}

/// State for the exit dialog.
#[derive(Debug, Clone)]
pub struct ExitDialogState {
    /// Currently focused element
    pub focus: ExitFocus,
    /// Clickable regions (populated during render)
    pub click_regions: Vec<ClickRegion>,
    /// "Preserve chat" checkbox state (keep chat visible on exit instead of restoring terminal)
    pub preserve_chat: CheckBoxState,
    /// "Exit when done" checkbox state (for background mode)
    pub exit_when_done: CheckBoxState,
}

impl Default for ExitDialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExitDialogState {
    /// Create a new exit dialog state with default focus on Exit button.
    /// `preserve_chat_default` should be the last known value or true.
    pub fn new() -> Self {
        Self::with_preserve_chat(true)
    }

    /// Create a new exit dialog state with a specific preserve_chat default.
    pub fn with_preserve_chat(preserve_chat_default: bool) -> Self {
        Self {
            focus: ExitFocus::ExitButton,
            click_regions: Vec::new(),
            preserve_chat: CheckBoxState::new(preserve_chat_default),
            exit_when_done: CheckBoxState::new(false),
        }
    }

    /// Move focus to next element (Tab).
    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
        self.update_checkbox_focus();
    }

    /// Move focus to previous element (Shift+Tab).
    pub fn focus_prev(&mut self) {
        self.focus = self.focus.prev();
        self.update_checkbox_focus();
    }

    /// Set focus to a specific element.
    pub fn set_focus(&mut self, element: ExitFocus) {
        self.focus = element;
        self.update_checkbox_focus();
    }

    /// Update the checkbox's focused state based on current focus.
    pub fn update_checkbox_focus(&mut self) {
        self.preserve_chat
            .set_focused(self.focus == ExitFocus::PreserveChatCheckbox);
        self.exit_when_done
            .set_focused(self.focus == ExitFocus::ExitWhenDoneCheckbox);
    }

    /// Toggle the preserve_chat checkbox.
    pub fn toggle_preserve_chat(&mut self) {
        self.preserve_chat.toggle();
    }

    /// Toggle the exit_when_done checkbox.
    pub fn toggle_exit_when_done(&mut self) {
        self.exit_when_done.toggle();
    }

    /// Clear click regions (called before render).
    pub fn clear_click_regions(&mut self) {
        self.click_regions.clear();
    }

    /// Add a click region.
    pub fn add_click_region(&mut self, area: Rect, element: ExitFocus) {
        self.click_regions.push(ClickRegion { area, element });
    }

    /// Handle click at position, returns the clicked element if any.
    pub fn handle_click(&self, col: u16, row: u16) -> Option<ExitFocus> {
        for region in &self.click_regions {
            if region.contains(col, row) {
                return Some(region.element);
            }
        }
        None
    }

    /// Get the action for the currently focused button.
    /// Returns None if checkbox is focused (checkbox is not an action).
    pub fn selected_action(&self) -> Option<ExitAction> {
        match self.focus {
            ExitFocus::ExitButton => Some(ExitAction::Exit),
            ExitFocus::BackgroundButton => Some(ExitAction::Background),
            ExitFocus::PreserveChatCheckbox | ExitFocus::ExitWhenDoneCheckbox => None,
        }
    }

    /// Check if preserve_chat is enabled.
    pub fn preserve_chat(&self) -> bool {
        self.preserve_chat.checked
    }

    /// Check if exit_when_done is enabled.
    pub fn exit_when_done(&self) -> bool {
        self.exit_when_done.checked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_focus_navigation() {
        let mut focus = ExitFocus::ExitButton;

        // Next: Exit -> Background -> PreserveChat -> ExitWhenDone -> Exit
        focus = focus.next();
        assert_eq!(focus, ExitFocus::BackgroundButton);

        focus = focus.next();
        assert_eq!(focus, ExitFocus::PreserveChatCheckbox);

        focus = focus.next();
        assert_eq!(focus, ExitFocus::ExitWhenDoneCheckbox);

        focus = focus.next();
        assert_eq!(focus, ExitFocus::ExitButton);

        // Prev: Exit -> ExitWhenDone -> PreserveChat -> Background -> Exit
        focus = focus.prev();
        assert_eq!(focus, ExitFocus::ExitWhenDoneCheckbox);

        focus = focus.prev();
        assert_eq!(focus, ExitFocus::PreserveChatCheckbox);

        focus = focus.prev();
        assert_eq!(focus, ExitFocus::BackgroundButton);

        focus = focus.prev();
        assert_eq!(focus, ExitFocus::ExitButton);
    }

    #[test]
    fn test_dialog_state_new() {
        let state = ExitDialogState::new();
        assert_eq!(state.focus, ExitFocus::ExitButton);
        assert!(state.click_regions.is_empty());
        // Preserve chat defaults to true
        assert!(state.preserve_chat());
    }

    #[test]
    fn test_selected_action() {
        let mut state = ExitDialogState::new();

        assert_eq!(state.selected_action(), Some(ExitAction::Exit));

        state.focus_next();
        assert_eq!(state.selected_action(), Some(ExitAction::Background));

        state.focus_next();
        assert_eq!(state.selected_action(), None); // PreserveChat checkbox focused

        state.focus_next();
        assert_eq!(state.selected_action(), None); // ExitWhenDone checkbox focused
    }

    #[test]
    fn test_click_region() {
        let region = ClickRegion {
            area: Rect::new(10, 5, 20, 3),
            element: ExitFocus::ExitButton,
        };

        // Inside region
        assert!(region.contains(10, 5));
        assert!(region.contains(29, 7));
        assert!(region.contains(15, 6));

        // Outside region
        assert!(!region.contains(9, 5)); // left of region
        assert!(!region.contains(30, 5)); // right of region
        assert!(!region.contains(10, 4)); // above region
        assert!(!region.contains(10, 8)); // below region
    }

    #[test]
    fn test_handle_click() {
        let mut state = ExitDialogState::new();
        state.add_click_region(Rect::new(10, 5, 10, 1), ExitFocus::ExitButton);
        state.add_click_region(Rect::new(25, 5, 10, 1), ExitFocus::BackgroundButton);

        assert_eq!(state.handle_click(15, 5), Some(ExitFocus::ExitButton));
        assert_eq!(state.handle_click(30, 5), Some(ExitFocus::BackgroundButton));
        assert_eq!(state.handle_click(22, 5), None); // Between buttons
    }
}
