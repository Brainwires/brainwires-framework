//! Hotkey dialog UI rendering.
//!
//! This module renders the hotkey configuration dialog overlay using the
//! generic HotkeyDialog component from ratatui_interact.

use ratatui::{Frame, layout::Rect};

use ratatui_interact::components::hotkey_dialog::{HotkeyDialog, HotkeyDialogStyle};

use crate::tui::{app::App, hotkey_content::BrainwiresHotkeyProvider};

/// Draw the hotkey dialog overlay.
pub fn draw_hotkey_dialog(f: &mut Frame, app: &mut App, _area: Rect) {
    let Some(state) = &mut app.hotkey_dialog_state else {
        return;
    };

    let provider = BrainwiresHotkeyProvider;
    let style = HotkeyDialogStyle::default();

    let dialog = HotkeyDialog::new(state, &provider, &style);
    dialog.render(f, f.area());
}
