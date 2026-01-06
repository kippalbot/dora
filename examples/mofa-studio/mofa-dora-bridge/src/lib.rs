//! # MoFA Dora Bridge
//!
//! Modular bridge system for connecting MoFA widgets to Dora dataflows.
//! Each widget (audio player, system log, prompt input) connects as a separate
//! dynamic node, enabling fine-grained control and independent lifecycle management.
//!
//! ## Architecture
//!
//! ```text
//! MoFA App
//!   ├── mofa-audio-player (dynamic node)
//!   ├── mofa-system-log (dynamic node)
//!   └── mofa-prompt-input (dynamic node)
//!          ↓
//!     Dora Dataflow
//! ```
//!
//! ## Usage
//!
//! 1. Parse dataflow to discover mofa-xxx nodes
//! 2. Create bridges for each discovered node
//! 3. Connect bridges as dynamic nodes
//! 4. Route data between widgets and dora

pub mod bridge;
pub mod data;
pub mod parser;
pub mod controller;
pub mod dispatcher;
pub mod error;

// Widget-specific bridges
pub mod widgets;

// Re-exports
pub use bridge::{DoraBridge, BridgeState, BridgeEvent};
pub use data::{DoraData, AudioData, LogEntry, ChatMessage, ControlCommand};
pub use parser::{DataflowParser, ParsedDataflow, ParsedNode, EnvRequirement, LogSource};
pub use controller::{DataflowController, DataflowState};
pub use dispatcher::{DynamicNodeDispatcher, WidgetBinding};
pub use error::{BridgeError, BridgeResult};

/// Prefix for MoFA built-in dynamic nodes in dataflow YAML
pub const MOFA_NODE_PREFIX: &str = "mofa-";

/// Known MoFA widget node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MofaNodeType {
    /// Audio player widget - receives audio, plays through speaker
    AudioPlayer,
    /// System log widget - receives logs from multiple nodes
    SystemLog,
    /// Prompt input widget - sends user prompts to LLM
    PromptInput,
    /// Mic input widget - captures audio from microphone
    MicInput,
    /// Chat viewer widget - displays conversation
    ChatViewer,
    /// Participant panel widget - receives audio and calculates levels for visualization
    ParticipantPanel,
}

impl MofaNodeType {
    /// Get the node ID for this widget type
    pub fn node_id(&self) -> &'static str {
        match self {
            MofaNodeType::AudioPlayer => "mofa-audio-player",
            MofaNodeType::SystemLog => "mofa-system-log",
            MofaNodeType::PromptInput => "mofa-prompt-input",
            MofaNodeType::MicInput => "mofa-mic-input",
            MofaNodeType::ChatViewer => "mofa-chat-viewer",
            MofaNodeType::ParticipantPanel => "mofa-participant-panel",
        }
    }

    /// Parse node type from node ID
    pub fn from_node_id(node_id: &str) -> Option<Self> {
        match node_id {
            "mofa-audio-player" => Some(MofaNodeType::AudioPlayer),
            "mofa-system-log" => Some(MofaNodeType::SystemLog),
            "mofa-prompt-input" => Some(MofaNodeType::PromptInput),
            "mofa-mic-input" => Some(MofaNodeType::MicInput),
            "mofa-chat-viewer" => Some(MofaNodeType::ChatViewer),
            "mofa-participant-panel" => Some(MofaNodeType::ParticipantPanel),
            _ => None,
        }
    }

    /// Check if a node ID is a MoFA widget node
    pub fn is_mofa_node(node_id: &str) -> bool {
        node_id.starts_with(MOFA_NODE_PREFIX)
    }
}
