//! Custom widgets for the Conference Dashboard

pub mod participant_panel;
pub mod log_panel;

use makepad_widgets::Cx;

/// Register all custom widgets with Makepad
pub fn live_design(cx: &mut Cx) {
    participant_panel::live_design(cx);
    log_panel::live_design(cx);
}
