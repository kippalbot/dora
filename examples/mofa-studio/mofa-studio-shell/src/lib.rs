//! MoFA Studio Shell - Main application shell

pub mod widgets;

use std::sync::Arc;
use parking_lot::Mutex;

/// Shared state for the application
#[derive(Default)]
pub struct SharedState {
    pub buffer_fill: f64,
    pub is_connected: bool,
    pub cpu_usage: f32,
    pub memory_usage: f32,
}

pub type SharedStateRef = Arc<Mutex<SharedState>>;

pub fn create_shared_state() -> SharedStateRef {
    Arc::new(Mutex::new(SharedState::default()))
}
