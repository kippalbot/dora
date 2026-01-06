//! MoFA FM App - AI-powered audio streaming and voice interface

pub mod screen;
pub mod mofa_hero;
pub mod audio;
pub mod audio_player;
pub mod dora_integration;
pub mod log_bridge;

pub use screen::MoFaFMScreen;
pub use screen::MoFaFMScreenWidgetRefExt;  // Export WidgetRefExt for timer control
pub use mofa_hero::{MofaHero, MofaHeroAction, ConnectionStatus};
pub use audio::AudioManager;
pub use dora_integration::{DoraIntegration, DoraCommand, DoraEvent, DoraState};

use makepad_widgets::Cx;
use mofa_widgets::{MofaApp, AppInfo};

/// MoFA FM app descriptor
pub struct MoFaFMApp;

impl MofaApp for MoFaFMApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "MoFA FM",
            id: "mofa-fm",
            description: "AI-powered audio streaming and voice interface",
        }
    }

    fn live_design(cx: &mut Cx) {
        mofa_hero::live_design(cx);
        screen::live_design(cx);
    }
}

/// Register all MoFA FM widgets with Makepad
/// (Kept for backwards compatibility - calls DoraApp::live_design)
pub fn live_design(cx: &mut Cx) {
    MoFaFMApp::live_design(cx);
}
