//! MoFA Settings App - Provider configuration and preferences

pub mod data;
pub mod screen;
pub mod providers_panel;
pub mod provider_view;
pub mod add_provider_modal;

pub use screen::SettingsScreenRef;

use makepad_widgets::Cx;
use mofa_widgets::{MofaApp, AppInfo};

/// MoFA Settings app descriptor
pub struct MoFaSettingsApp;

impl MofaApp for MoFaSettingsApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "Settings",
            id: "mofa-settings",
            description: "Provider configuration and preferences",
        }
    }

    fn live_design(cx: &mut Cx) {
        providers_panel::live_design(cx);
        provider_view::live_design(cx);
        add_provider_modal::live_design(cx);
        screen::live_design(cx);
    }
}

/// Register all Settings widgets with Makepad
/// (Kept for backwards compatibility - calls DoraApp::live_design)
pub fn live_design(cx: &mut Cx) {
    MoFaSettingsApp::live_design(cx);
}
