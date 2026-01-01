//! Conference Dashboard - Makepad-based visual UI for Dora conference system
//!
//! Features:
//! - Buffer visualization with fill percentage
//! - Participant status panels (student1, student2, tutor)
//! - Real-time waveform display
//! - Integrated audio playback with circular buffer
//! - Control output for conference-controller commands

pub mod app;
pub mod audio_player;
pub mod dora_bridge;
pub mod widgets;

use std::sync::Arc;
use parking_lot::Mutex;

/// Control commands that can be sent from UI to Dora dataflow
#[derive(Clone, Debug)]
pub enum ControlCommand {
    /// Reset the current session
    Reset,
    /// Start a new question with the given ID
    StartQuestion(u32),
    /// Pause audio playback
    Pause,
    /// Resume audio playback
    Resume,
    /// Skip to next round
    NextRound,
    /// Send a custom prompt to the conference controller (like debate_monitor's Send button)
    SendPrompt(String),
}

/// Shared state between Dora bridge and Makepad UI
#[derive(Default)]
pub struct SharedState {
    /// Buffer fill percentage (0-100)
    pub buffer_fill: f64,
    /// Buffer available seconds
    pub buffer_seconds: f64,
    /// Participant states
    pub participants: [ParticipantState; 3],
    /// Audio waveform data for visualization (last N samples)
    pub waveform_data: Vec<f32>,
    /// Playback status
    pub playback_status: PlaybackStatus,
    /// Log messages
    pub log_messages: Vec<LogMessage>,
    /// Chat messages for conversation history display
    pub chat_messages: Vec<ChatMessage>,
    /// Pending control commands from UI (processed by dora_bridge)
    pub control_commands: Vec<ControlCommand>,
    /// Prompt input text (synced between UI and state)
    pub prompt_input: String,
    /// Whether connected to Dora dataflow
    pub is_connected: bool,
    /// OpenAI API key (from environment)
    pub openai_api_key: Option<String>,
    /// DeepSeek API key (from environment)
    pub deepseek_api_key: Option<String>,
    /// System CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// System memory usage percentage (0-100)
    pub memory_usage: f32,
    /// Total system memory in GB
    pub total_memory_gb: f32,
    /// Used system memory in GB
    pub used_memory_gb: f32,
}

/// Chat message for conversation history
#[derive(Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub text: String,
    pub timestamp: String,
}

#[derive(Clone, Default)]
pub struct ParticipantState {
    pub name: String,
    pub is_speaking: bool,
    pub segment_count: u32,
    pub last_text: String,
    pub conversation_history: String,  // Full conversation history
    pub status: String,
    pub audio_level: f32,  // Current audio level (0.0 - 1.0) for LED bar
}

#[derive(Clone, Default)]
pub struct PlaybackStatus {
    pub is_playing: bool,
    pub current_speaker: String,
    pub total_played_seconds: f64,
    pub active_participant_idx: Option<usize>,  // 0=student1, 1=student2, 2=tutor
}

#[derive(Clone)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

pub type SharedStateRef = Arc<Mutex<SharedState>>;

/// Initialize default shared state
pub fn create_shared_state() -> SharedStateRef {
    // Check if running in AEC mode (single user + assistant) - requires explicit AEC_MODE=true
    let aec_mode = std::env::var("AEC_MODE")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    let participants = if aec_mode {
        [
            ParticipantState { name: "User".into(), ..Default::default() },
            ParticipantState { name: "Assistant".into(), ..Default::default() },
            ParticipantState { name: "TTS".into(), ..Default::default() },
        ]
    } else {
        // Read participant names from environment variables (configurable in YAML)
        let p1 = std::env::var("PARTICIPANT1_NAME").unwrap_or_else(|_| "Daniu".into());
        let p2 = std::env::var("PARTICIPANT2_NAME").unwrap_or_else(|_| "Yifei".into());
        let p3 = std::env::var("PARTICIPANT3_NAME").unwrap_or_else(|_| "Laoshi".into());
        [
            ParticipantState { name: p1, ..Default::default() },
            ParticipantState { name: p2, ..Default::default() },
            ParticipantState { name: p3, ..Default::default() },
        ]
    };

    // Read API keys from environment variables
    let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
    let deepseek_api_key = std::env::var("DEEPSEEK_API_KEY").ok();

    // Log API key status (show masked key for verification)
    match &openai_api_key {
        Some(key) if key.len() > 8 => {
            log::info!("OPENAI_API_KEY: {}...{} (len={})", &key[..4], &key[key.len()-4..], key.len());
        }
        Some(key) => log::warn!("OPENAI_API_KEY: too short (len={})", key.len()),
        None => log::warn!("OPENAI_API_KEY: NOT SET"),
    }
    match &deepseek_api_key {
        Some(key) if key.len() > 8 => {
            log::info!("DEEPSEEK_API_KEY: {}...{} (len={})", &key[..4], &key[key.len()-4..], key.len());
        }
        Some(key) => log::warn!("DEEPSEEK_API_KEY: too short (len={})", key.len()),
        None => log::warn!("DEEPSEEK_API_KEY: NOT SET"),
    }

    Arc::new(Mutex::new(SharedState {
        buffer_fill: 0.0,
        buffer_seconds: 0.0,
        participants,
        waveform_data: vec![0.0; 512],
        playback_status: PlaybackStatus::default(),
        log_messages: Vec::new(),
        chat_messages: Vec::new(),
        control_commands: Vec::new(),
        prompt_input: String::new(),
        is_connected: false,
        openai_api_key,
        deepseek_api_key,
        cpu_usage: 0.0,
        memory_usage: 0.0,
        total_memory_gb: 0.0,
        used_memory_gb: 0.0,
    }))
}
