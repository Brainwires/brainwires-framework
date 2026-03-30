//! Suspend/Background dialog state management.
//!
//! This module contains the state and logic for the Ctrl+Z suspend dialog,
//! which allows users to either background (keep running) or suspend (stop)
//! the application.

use ratatui::layout::Rect;
use ratatui_interact::components::CheckBoxState;

/// Available actions in the suspend dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendAction {
    /// Keep process running, restore terminal, return control to shell.
    /// Agents continue working in the background.
    Background,
    /// Stop process completely with SIGTSTP.
    /// Process is suspended until user runs `fg`.
    Suspend,
}

/// Focusable elements in the suspend dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SuspendFocus {
    /// The "Background" button (default focus)
    #[default]
    BackgroundButton,
    /// The "Suspend" button
    SuspendButton,
    /// The "Exit when done" checkbox
    ExitWhenDoneCheckbox,
}

impl SuspendFocus {
    /// Move to next focusable element.
    pub fn next(&self) -> Self {
        match self {
            SuspendFocus::BackgroundButton => SuspendFocus::SuspendButton,
            SuspendFocus::SuspendButton => SuspendFocus::ExitWhenDoneCheckbox,
            SuspendFocus::ExitWhenDoneCheckbox => SuspendFocus::BackgroundButton,
        }
    }

    /// Move to previous focusable element.
    pub fn prev(&self) -> Self {
        match self {
            SuspendFocus::BackgroundButton => SuspendFocus::ExitWhenDoneCheckbox,
            SuspendFocus::SuspendButton => SuspendFocus::BackgroundButton,
            SuspendFocus::ExitWhenDoneCheckbox => SuspendFocus::SuspendButton,
        }
    }
}

/// Clickable region for mouse interaction.
#[derive(Debug, Clone)]
pub struct ClickRegion {
    /// The area this click region covers
    pub area: Rect,
    /// The element this region corresponds to
    pub element: SuspendFocus,
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

/// State for the suspend dialog.
#[derive(Debug, Clone)]
pub struct SuspendDialogState {
    /// Currently focused element
    pub focus: SuspendFocus,
    /// Clickable regions (populated during render)
    pub click_regions: Vec<ClickRegion>,
    /// "Exit when done" checkbox state
    pub exit_when_done: CheckBoxState,
}

impl Default for SuspendDialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspendDialogState {
    /// Create a new suspend dialog state with default focus on Background button.
    pub fn new() -> Self {
        Self {
            focus: SuspendFocus::BackgroundButton,
            click_regions: Vec::new(),
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
    pub fn set_focus(&mut self, element: SuspendFocus) {
        self.focus = element;
        self.update_checkbox_focus();
    }

    /// Update the checkbox's focused state based on current focus.
    pub fn update_checkbox_focus(&mut self) {
        self.exit_when_done.set_focused(self.focus == SuspendFocus::ExitWhenDoneCheckbox);
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
    pub fn add_click_region(&mut self, area: Rect, element: SuspendFocus) {
        self.click_regions.push(ClickRegion { area, element });
    }

    /// Handle click at position, returns the clicked element if any.
    pub fn handle_click(&self, col: u16, row: u16) -> Option<SuspendFocus> {
        for region in &self.click_regions {
            if region.contains(col, row) {
                return Some(region.element);
            }
        }
        None
    }

    /// Get the action for the currently focused button.
    /// Returns None if checkbox is focused (checkbox is not an action).
    pub fn selected_action(&self) -> Option<SuspendAction> {
        match self.focus {
            SuspendFocus::BackgroundButton => Some(SuspendAction::Background),
            SuspendFocus::SuspendButton => Some(SuspendAction::Suspend),
            SuspendFocus::ExitWhenDoneCheckbox => None,
        }
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
    fn test_suspend_focus_navigation() {
        let mut focus = SuspendFocus::BackgroundButton;

        // Next: Background -> Suspend -> ExitWhenDone -> Background
        focus = focus.next();
        assert_eq!(focus, SuspendFocus::SuspendButton);

        focus = focus.next();
        assert_eq!(focus, SuspendFocus::ExitWhenDoneCheckbox);

        focus = focus.next();
        assert_eq!(focus, SuspendFocus::BackgroundButton);

        // Prev: Background -> ExitWhenDone -> Suspend -> Background
        focus = focus.prev();
        assert_eq!(focus, SuspendFocus::ExitWhenDoneCheckbox);

        focus = focus.prev();
        assert_eq!(focus, SuspendFocus::SuspendButton);

        focus = focus.prev();
        assert_eq!(focus, SuspendFocus::BackgroundButton);
    }

    #[test]
    fn test_dialog_state_new() {
        let state = SuspendDialogState::new();
        assert_eq!(state.focus, SuspendFocus::BackgroundButton);
        assert!(state.click_regions.is_empty());
    }

    #[test]
    fn test_selected_action() {
        let mut state = SuspendDialogState::new();

        assert_eq!(state.selected_action(), Some(SuspendAction::Background));

        state.focus_next();
        assert_eq!(state.selected_action(), Some(SuspendAction::Suspend));
    }

    #[test]
    fn test_click_region() {
        let region = ClickRegion {
            area: Rect::new(10, 5, 20, 3),
            element: SuspendFocus::BackgroundButton,
        };

        // Inside region
        assert!(region.contains(10, 5));
        assert!(region.contains(29, 7));
        assert!(region.contains(15, 6));

        // Outside region
        assert!(!region.contains(9, 5));   // left of region
        assert!(!region.contains(30, 5));  // right of region
        assert!(!region.contains(10, 4));  // above region
        assert!(!region.contains(10, 8));  // below region
    }

    #[test]
    fn test_handle_click() {
        let mut state = SuspendDialogState::new();
        state.add_click_region(Rect::new(10, 5, 10, 1), SuspendFocus::BackgroundButton);
        state.add_click_region(Rect::new(25, 5, 10, 1), SuspendFocus::SuspendButton);

        assert_eq!(state.handle_click(15, 5), Some(SuspendFocus::BackgroundButton));
        assert_eq!(state.handle_click(30, 5), Some(SuspendFocus::SuspendButton));
        assert_eq!(state.handle_click(22, 5), None); // Between buttons
    }
}
