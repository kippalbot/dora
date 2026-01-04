//! MoFA FM App - AI-powered audio streaming and voice interface

pub mod screen;
pub mod mofa_hero;
pub mod audio;

pub use screen::MoFaFMScreen;
pub use mofa_hero::{MofaHero, MofaHeroAction, ConnectionStatus};
pub use audio::AudioManager;

use makepad_widgets::Cx;

/// Register all MoFA FM widgets with Makepad
pub fn live_design(cx: &mut Cx) {
    mofa_hero::live_design(cx);
    screen::live_design(cx);
}
