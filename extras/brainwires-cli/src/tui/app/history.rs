//! Prompt History Navigation
//!
//! Handles prompt history and search functionality.
//!
//! When navigating history, the current input is saved as a "draft" so it can
//! be restored when the user navigates back down past the most recent history entry.

use super::state::App;

pub(super) trait HistoryOps {
    fn navigate_history_up(&mut self);
    fn navigate_history_down(&mut self);
    fn update_search_results(&mut self);
    fn get_current_search_result(&self) -> Option<String>;
}

impl HistoryOps for App {
    /// Navigate history up
    ///
    /// On first navigation up, saves the current input as a draft.
    /// If there's a draft saved (from pressing Down first), restores it.
    fn navigate_history_up(&mut self) {
        // If we have a draft and we're not navigating history, restore the draft
        // This handles: paste -> Down (saves draft, clears input) -> Up (restore draft)
        if !self.prompt_history.is_navigating()
            && let Some(draft) = self.input_draft.take()
        {
            self.input_state.set_text(draft);
            return;
        }

        // If we're not already navigating history, save current input as draft
        if self.input_draft.is_none() && !self.input_is_empty() {
            self.input_draft = Some(self.input_text());
        }

        if let Some(prev) = self.prompt_history.previous() {
            self.input_state.set_text(prev);
        }
    }

    /// Navigate history down
    ///
    /// When navigating past the most recent history entry, restores the draft.
    /// If not navigating history but input exists, saves it as draft and clears input.
    fn navigate_history_down(&mut self) {
        // If we're not navigating history...
        if !self.prompt_history.is_navigating() {
            // If there's input, save it as draft and give user empty line
            // This allows: paste -> Down (empty line) -> Up (restore paste)
            if !self.input_is_empty() {
                self.input_draft = Some(self.input_text());
                self.clear_input();
            }
            return;
        }

        if let Some(next) = self.prompt_history.next_prompt() {
            self.input_state.set_text(next);
        } else {
            // Reached the end - restore draft if we have one, otherwise clear
            if let Some(draft) = self.input_draft.take() {
                self.input_state.set_text(draft);
                // Put cursor at beginning so user can navigate down through multiline draft
                self.input_state.move_to_start();
            } else {
                self.clear_input();
            }
            self.prompt_history.reset();
        }
    }

    /// Update search results based on current query
    fn update_search_results(&mut self) {
        self.search_results = self.prompt_history.search(&self.search_query);
        self.search_result_index = 0;
    }

    /// Get current search result
    fn get_current_search_result(&self) -> Option<String> {
        self.search_results.get(self.search_result_index).cloned()
    }
}
