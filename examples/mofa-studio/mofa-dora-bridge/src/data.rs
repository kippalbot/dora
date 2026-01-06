//! Data types for MoFA-Dora communication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unified data type for all MoFA-Dora communication
#[derive(Debug, Clone)]
pub enum DoraData {
    /// Audio samples (f32, mono or stereo)
    Audio(AudioData),
    /// Text string
    Text(String),
    /// Structured JSON data
    Json(serde_json::Value),
    /// Raw binary data
    Binary(Vec<u8>),
    /// Control command
    Control(ControlCommand),
    /// Log entry
    Log(LogEntry),
    /// Chat message
    Chat(ChatMessage),
    /// Empty/signal data
    Empty,
}

impl DoraData {
    /// Create audio data from f32 samples
    pub fn audio(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        DoraData::Audio(AudioData {
            samples,
            sample_rate,
            channels,
            participant_id: None,
        })
    }

    /// Create text data
    pub fn text(s: impl Into<String>) -> Self {
        DoraData::Text(s.into())
    }

    /// Create log entry
    pub fn log(level: LogLevel, message: impl Into<String>, node_id: impl Into<String>) -> Self {
        DoraData::Log(LogEntry {
            level,
            message: message.into(),
            node_id: node_id.into(),
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        })
    }

    /// Create control command
    pub fn control(command: impl Into<String>) -> Self {
        DoraData::Control(ControlCommand {
            command: command.into(),
            params: HashMap::new(),
        })
    }
}

/// Audio data with metadata
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Audio samples in f32 format (-1.0 to 1.0)
    pub samples: Vec<f32>,
    /// Sample rate in Hz (e.g., 32000, 44100, 48000)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Optional participant ID for multi-speaker scenarios
    pub participant_id: Option<String>,
}

impl AudioData {
    /// Duration in seconds
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }

    /// Convert to mono if stereo
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            return self.samples.clone();
        }
        self.samples
            .chunks(self.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect()
    }
}

/// Log entry from dora nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Source node ID
    pub node_id: String,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl LogEntry {
    /// Create a new log entry with current timestamp
    pub fn new(level: LogLevel, message: impl Into<String>, node_id: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            node_id: node_id.into(),
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Log level for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug = 10,
    Info = 20,
    Warning = 30,
    Error = 40,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl LogLevel {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARNING" | "WARN" => LogLevel::Warning,
            "ERROR" | "ERR" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

/// Chat message for conversation display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message content
    pub content: String,
    /// Sender ID (participant name or "user")
    pub sender: String,
    /// Message role (user, assistant, system)
    pub role: MessageRole,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// Whether this is a streaming/partial message
    pub is_streaming: bool,
    /// Session/conversation ID
    pub session_id: Option<String>,
}

impl ChatMessage {
    /// Create user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            sender: "user".to_string(),
            role: MessageRole::User,
            timestamp: current_timestamp(),
            is_streaming: false,
            session_id: None,
        }
    }

    /// Create assistant message
    pub fn assistant(content: impl Into<String>, sender: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            sender: sender.into(),
            role: MessageRole::Assistant,
            timestamp: current_timestamp(),
            is_streaming: false,
            session_id: None,
        }
    }
}

/// Message role in conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Control command for dataflow orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCommand {
    /// Command name (e.g., "start", "stop", "pause", "reset")
    pub command: String,
    /// Command parameters
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

impl ControlCommand {
    /// Create a simple command without parameters
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            params: HashMap::new(),
        }
    }

    /// Add parameter
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Create "start" command
    pub fn start() -> Self {
        Self::new("start")
    }

    /// Create "stop" command
    pub fn stop() -> Self {
        Self::new("stop")
    }

    /// Create "reset" command
    pub fn reset() -> Self {
        Self::new("reset")
    }

    /// Create "send_prompt" command with message
    pub fn send_prompt(message: impl Into<String>) -> Self {
        Self::new("send_prompt").with_param("message", serde_json::Value::String(message.into()))
    }
}

/// Metadata from dora events
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    /// Key-value pairs
    pub values: HashMap<String, String>,
}

impl EventMetadata {
    /// Get value by key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// Get session status
    pub fn session_status(&self) -> Option<&str> {
        self.get("session_status")
    }

    /// Get question ID
    pub fn question_id(&self) -> Option<&str> {
        self.get("question_id")
    }

    /// Get participant ID
    pub fn participant_id(&self) -> Option<&str> {
        self.get("participant_id")
    }
}

/// Get current unix timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
