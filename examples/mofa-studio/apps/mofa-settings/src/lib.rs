//! MoFA Settings App - Provider configuration and preferences

pub mod data;
pub mod screen;
pub mod providers_panel;
pub mod provider_view;
pub mod add_provider_modal;

pub use screen::SettingsScreenRef;

use makepad_widgets::Cx;

/// Register all Settings widgets with Makepad
pub fn live_design(cx: &mut Cx) {
    providers_panel::live_design(cx);
    provider_view::live_design(cx);
    add_provider_modal::live_design(cx);
    screen::live_design(cx);
}
