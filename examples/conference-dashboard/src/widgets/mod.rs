//! Custom widgets for the Conference Dashboard

pub mod theme;
pub mod participant_panel;
pub mod log_panel;
pub mod sidebar;
pub mod mofa_fm_screen;
pub mod mofa_hero;
pub mod provider_view;
pub mod providers_panel;
pub mod add_provider_modal;
pub mod settings_screen;

use makepad_widgets::Cx;

/// Register all custom widgets with Makepad
pub fn live_design(cx: &mut Cx) {
    // Theme must be registered first (contains shared fonts)
    theme::live_design(cx);
    participant_panel::live_design(cx);
    log_panel::live_design(cx);
    sidebar::live_design(cx);
    mofa_fm_screen::live_design(cx);
    mofa_hero::live_design(cx);
    provider_view::live_design(cx);
    providers_panel::live_design(cx);
    add_provider_modal::live_design(cx);
    settings_screen::live_design(cx);
}
