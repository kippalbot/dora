//! MoFA Widgets - Shared reusable UI components for MoFA Studio apps

pub mod theme;
pub mod waveform_view;
pub mod participant_panel;
pub mod log_panel;
pub mod led_gauge;
pub mod audio_player;

use makepad_widgets::Cx;

/// Register all shared widgets with Makepad
/// IMPORTANT: Theme must be registered first as other widgets depend on it
pub fn live_design(cx: &mut Cx) {
    // Theme provides fonts and base styles - must be first
    theme::live_design(cx);

    // Register widgets in dependency order
    waveform_view::live_design(cx);
    participant_panel::live_design(cx);
    log_panel::live_design(cx);
    led_gauge::live_design(cx);
}

// Re-export commonly used types
pub use audio_player::*;
