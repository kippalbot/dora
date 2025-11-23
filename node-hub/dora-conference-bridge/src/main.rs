use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::env;

use dora_node_api::{
    DoraNode, Event, Parameter,
    arrow::array::{Array, AsArray, StringArray},
    dora_core::config::DataId,
};
use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const NODE_NAME: &str = "dora-conference-bridge";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl LogLevel {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "error" => Some(LogLevel::Error),
            "warn" | "warning" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            _ => None,
        }
    }

    fn allows(self, other: LogLevel) -> bool {
        other as i32 <= self as i32
    }
}

fn get_friendly_node_name(node_id: &str) -> String {
    // Convert technical node IDs to user-friendly names
    match node_id {
        "bridge-to-judge" => "Bridge to Judge".to_string(),
        "bridge-to-llm1" => "Bridge to LLM1".to_string(),
        "bridge-to-llm2" => "Bridge to LLM2".to_string(),
        _ => {
            // Fallback: make technical ID more readable
            node_id.replace("-", " ")
                .replace("bridge", "Bridge")
                .replace("llm", "LLM")
        }
    }
}

fn send_log(node: &mut DoraNode, level: LogLevel, config_level: LogLevel, message: &str) {
    if !config_level.allows(level) {
        return;
    }

    // Priority: Env var → Built-in node ID → Default constant
    let node_identifier = std::env::var("DORA_NODE_NAME")
        .or_else(|_| std::env::var("DORA_NODE_ID"))
        .unwrap_or_else(|_| get_friendly_node_name(&node.id().to_string()));

    let level_str = match level {
        LogLevel::Error => "ERROR",
        LogLevel::Warn => "WARNING",
        LogLevel::Info => "INFO",
        LogLevel::Debug => "DEBUG",
    };

    let log_data = serde_json::json!({
        "node": node_identifier,
        "level": level_str,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp()
    });

    if let Err(err) = node.send_output(
        DataId::from("log".to_string()),
        Default::default(),
        StringArray::from(vec![log_data.to_string().as_str()]),
    ) {
        eprintln!("[Conference Bridge] Failed to send log output: {:?}", err);
    }
}


#[derive(Debug, Clone, PartialEq)]
enum SignalType {
    ResetSignal,      // session_status: "reset" - control signal, drop silently
    CancelledSignal,  // session_status: "cancelled" - control signal, drop silently
    TechnicalError,   // session_status: "error" - actual error, may need notification
    ContentError,     // Text-based errors like "Error:" - forward template if configured
    NormalContent,    // Regular content - forward as-is
}

/// Classify the type of signal from metadata and text content
fn classify_signal(metadata: &BTreeMap<String, Parameter>, text: &str) -> SignalType {
    // First check session_status in metadata
    if let Some(Parameter::String(status)) = metadata.get("session_status") {
        match status.as_str() {
            "reset" => return SignalType::ResetSignal,
            "cancelled" => return SignalType::CancelledSignal,
            "error" => return SignalType::TechnicalError,
            _ => {} // Fall through to text-based detection
        }
    }

    // Check for text-based error patterns
    if text.starts_with("Error:") || text.starts_with("error:") {
        SignalType::ContentError
    } else {
        // Default to normal content
        SignalType::NormalContent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundledMessage {
    participant: String,
    content: String,
    complete: bool,
}

#[derive(Debug, Clone)]
enum MessageState {
    Streaming {
        chunks: Vec<String>,
        metadata: BTreeMap<String, Parameter>,
    },
    Complete {
        content: String,
    },
}

impl MessageState {
    fn new_streaming() -> Self {
        MessageState::Streaming {
            chunks: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    fn add_chunk(&mut self, chunk: String, metadata: BTreeMap<String, Parameter>) {
        match self {
            MessageState::Streaming { chunks, metadata: meta } => {
                chunks.push(chunk);
                meta.extend(metadata);
            }
            _ => {}
        }
    }

    fn complete(&mut self) -> String {
        match self {
            MessageState::Streaming { chunks, .. } => {
                let content = chunks.join("");
                *self = MessageState::Complete {
                    content: content.clone(),
                };
                content
            }
            MessageState::Complete { content } => content.clone(),
        }
    }

    fn get_content(&self) -> String {
        match self {
            MessageState::Streaming { chunks, .. } => chunks.join(""),
            MessageState::Complete { content, .. } => content.clone(),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self, MessageState::Complete { .. })
    }

}

#[derive(Debug)]
struct InputPort {
    port_name: String,
    is_streaming: bool,  // Explicitly configured
    message_state: Option<MessageState>,
    ready: bool,
    draining: bool,
    was_already_ready: bool, // Track if we already logged this as ready
    signal_type: Option<SignalType>,  // Type of signal detected, if any
    should_forward: bool,  // Whether this input should be forwarded (false for control signals)
}

impl InputPort {
    fn new(port_name: String, is_streaming: bool) -> Self {
        Self {
            port_name,
            is_streaming,
            message_state: None,
            ready: false,
            draining: false,
            was_already_ready: false,
            signal_type: None,
            should_forward: true,  // Default to forwarding
        }
    }

    fn is_message_complete(&self, metadata: &BTreeMap<String, Parameter>) -> bool {
        // Check session_status
        if let Some(Parameter::String(status)) = metadata.get("session_status") {
            // ended = normal completion
            // error = error occurred
            // cancelled = user cancelled streaming (history preserved in LLM)
            // reset = user reset (streaming cancelled + history cleared)
            if status == "ended" || status == "error" || status == "cancelled" || status == "reset" {
                return true;
            }
        }

        // Check is_complete
        if let Some(Parameter::Bool(true)) = metadata.get("is_complete") {
            return true;
        }

        false
    }

    fn is_drop_status(metadata: &BTreeMap<String, Parameter>) -> bool {
        if let Some(Parameter::String(status)) = metadata.get("session_status") {
            // error, cancelled, and reset should all be dropped during forwarding
            // (they represent incomplete/interrupted responses)
            return status == "error" || status == "cancelled" || status == "reset";
        }
        false
    }

    /// Check if text content indicates an error (fallback when metadata isn't set)
    fn is_error_text(text: &str) -> bool {
        // Check for common error prefixes from maas-client
        text.starts_with("Error:") || text.starts_with("error:")
    }

    fn handle_input(&mut self, text: String, metadata: BTreeMap<String, Parameter>) -> bool {
        // Classify the signal type from metadata and text
        let signal_type = classify_signal(&metadata, &text);
        self.signal_type = Some(signal_type.clone());

        // Determine if this should be forwarded based on signal type
        self.should_forward = match signal_type {
            SignalType::ResetSignal => {
                println!("[BRIDGE-STDOUT] 🔄 RESET SIGNAL DETECTED: port='{}', dropping silently", self.port_name);
                false  // Don't forward reset signals
            }
            SignalType::CancelledSignal => {
                println!("[BRIDGE-STDOUT] 🛑 CANCELLED SIGNAL DETECTED: port='{}', dropping silently", self.port_name);
                false  // Don't forward cancelled signals
            }
            SignalType::TechnicalError => {
                println!("[BRIDGE-STDOUT] ⚠️ TECHNICAL ERROR DETECTED: port='{}', will forward template message", self.port_name);
                true   // Forward template message for technical errors
            }
            SignalType::ContentError => {
                println!("[BRIDGE-STDOUT] ❌ CONTENT ERROR DETECTED: port='{}', text='{}'", self.port_name, text);
                true   // Forward template message for content errors
            }
            SignalType::NormalContent => {
                // Check if this is an ending signal (empty content with ending status)
                let is_ending_signal = text.trim().is_empty() &&
                    metadata.get("session_status")
                        .and_then(|p| match p {
                            Parameter::String(s) => Some(s.as_str()),
                            _ => None,
                        })
                        .map_or(false, |status| status == "ended" || status == "error" || status == "cancelled" || status == "reset");

                if is_ending_signal {
                    println!("[BRIDGE-STDOUT] 🏁 ENDING SIGNAL DETECTED: port='{}', completing existing message", self.port_name);
                } else {
                    println!("[BRIDGE-STDOUT] ✅ NORMAL CONTENT DETECTED: port='{}', text='{}'", self.port_name, &text.chars().take(50).collect::<String>());
                }
                true   // Forward normal content as-is
            }
        };

        // Check if this is the start of a new message (session_status: "started")
        let is_new_start = metadata.get("session_status")
            .and_then(|p| match p {
                Parameter::String(s) => Some(s.as_str()),
                _ => None,
            })
            .map_or(false, |status| status == "started");

        // Reset message state if this is a new start
        if is_new_start {
            println!("[BRIDGE-STDOUT] 🆕 NEW MESSAGE START: port='{}', resetting message state", self.port_name);
            self.message_state = Some(MessageState::new_streaming());
            self.ready = false;
            self.was_already_ready = false;
        }

        if self.is_streaming {
            // Streaming input - accumulate chunks
            if self.message_state.is_none() {
                self.message_state = Some(MessageState::new_streaming());
            }

            // Only add chunk if it's not an empty ending signal
            if let Some(state) = &mut self.message_state {
                let is_ending_signal = text.trim().is_empty() &&
                    metadata.get("session_status")
                        .and_then(|p| match p {
                            Parameter::String(s) => Some(s.as_str()),
                            _ => None,
                        })
                        .map_or(false, |status| status == "ended" || status == "error" || status == "cancelled" || status == "reset");

                if !is_ending_signal {
                    state.add_chunk(text, metadata.clone());
                } else {
                    println!("[BRIDGE-STDOUT] 🏁 SKIPPING EMPTY ENDING CHUNK - preserving accumulated content");
                }
            }

            // Check if complete (includes error, cancelled, reset status)
            if self.is_message_complete(&metadata) {
                if let Some(state) = &mut self.message_state {
                    state.complete();
                }
                self.ready = true;
                return true;  // Ready to forward (will be filtered in forward_bundle)
            }
        } else {
            // Non-streaming input - complete immediately
            self.message_state = Some(MessageState::Complete {
                content: text,
            });
            self.ready = true;
            return true;  // Ready to forward (will be filtered in forward_bundle)
        }

        false  // Not ready yet
    }

    fn get_bundled_message(&self) -> Option<BundledMessage> {
        self.message_state.as_ref().map(|state| BundledMessage {
            participant: self.port_name.clone(),
            content: state.get_content(),
            complete: state.is_complete(),
        })
    }

    fn reset(&mut self) {
        self.message_state = None;
        self.ready = false;
        self.draining = false;
        self.was_already_ready = false;
        self.signal_type = None;
        self.should_forward = true;
    }

    fn reset_with_drain(&mut self, drain: bool) {
        self.message_state = None;
        self.ready = false;
        self.draining = drain;
        self.was_already_ready = false;
        self.signal_type = None;
        self.should_forward = true;
    }

    fn is_streaming_active(&self) -> bool {
        matches!(self.message_state, Some(MessageState::Streaming { .. }))
    }

}

fn metadata_indicates_completion(metadata: &BTreeMap<String, Parameter>) -> bool {
    match metadata.get("session_status") {
        Some(Parameter::String(status)) if status == "ended" || status == "error" || status == "cancelled" || status == "reset" => return true,
        _ => {}
    }

    matches!(metadata.get("is_complete"), Some(Parameter::Bool(true)))
}

struct ConferenceBridge {
    inputs: HashMap<String, InputPort>,
    streaming_ports: HashSet<String>,
    expected_ports: HashSet<String>,
    log_level: LogLevel,
    arrival_queue: VecDeque<String>,
    current_question_id: u32,
    increment_question_id: bool,
    last_status: String,  // Track last status to avoid duplicate logs
    resume_mode: bool,     // Track if bridge is in resume mode
    error_message_template: Option<String>,  // Template for error messages, {participant} will be replaced
  }

impl ConferenceBridge {
    fn new(
        streaming_ports: HashSet<String>,
        expected_ports: HashSet<String>,
        log_level: LogLevel,
        increment_question_id: bool,
        error_message_template: Option<String>,
    ) -> Self {
        let mut bridge = Self {
            inputs: HashMap::new(),
            streaming_ports,
            expected_ports,
            log_level,
            arrival_queue: VecDeque::new(),
            current_question_id: 0,
            increment_question_id,
            last_status: String::new(),
            resume_mode: false,  // Start in paused mode
            error_message_template,
        };

        let preset_ports: Vec<String> = bridge
            .expected_ports
            .iter()
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .collect();
        for port in preset_ports {
            bridge.register_input(port);
        }

        bridge
    }

    fn register_input(&mut self, port_name: String) {
        if !self.inputs.contains_key(&port_name) {
            let is_streaming = self.streaming_ports.contains(&port_name);
            self.inputs.insert(port_name.clone(), InputPort::new(port_name, is_streaming));
        }
    }

    /// Send status output, but only if it has changed from last time (deduplication)
    fn send_status(&mut self, node: &mut DoraNode, status: &str) -> Result<()> {
        // Only send status if it has changed from last time
        if self.last_status != status {
            node.send_output(
                DataId::from("status".to_string()),
                Default::default(),
                StringArray::from(vec![status]),
            )
            .context("Failed to send status output")?;
            self.last_status = status.to_string();
        }
        Ok(())
    }

    fn handle_input(&mut self, port_name: &str, text: String, metadata: BTreeMap<String, Parameter>) -> bool {
        // Register input if not known
        self.register_input(port_name.to_string());

        // Track arrival order in FIFO queue (only add if not already present)
        if !self.arrival_queue.iter().any(|p| p == port_name) {
            self.arrival_queue.push_back(port_name.to_string());
        }

        // Extract question_id from metadata (use first arrival's question_id)
        if self.current_question_id == 0 {
            if let Some(Parameter::String(qid_str)) = metadata.get("question_id") {
                if let Ok(qid) = qid_str.parse::<u32>() {
                    self.current_question_id = qid;
                }
            }
        }

        // Handle the input
        if let Some(input) = self.inputs.get_mut(port_name) {
            input.handle_input(text, metadata)
        } else {
            false
        }
    }


    fn get_ready_inputs(&self) -> HashSet<String> {
        self.inputs.iter()
            .filter(|(_, input)| input.ready)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn handle_drain(
        &mut self,
        port_name: &str,
        is_starting: bool,
        is_complete: bool,
        has_session_status: bool,
    ) -> bool {
        if let Some(input) = self.inputs.get_mut(port_name) {
            if input.draining {
                if is_complete {
                    input.draining = false;
                    return true;
                }

                if is_starting || !has_session_status {
                    input.draining = false;
                    return false;
                }

                return true;
            }
        }

        false
    }

    fn finalize_cycle(&mut self, node: &mut DoraNode, status: &str) -> Result<()> {

        for (_, input) in self.inputs.iter_mut() {
            input.reset();
        }

        self.arrival_queue.clear();
        self.current_question_id = 0;

        self.send_status(node, status)?;
        Ok(())
    }

    fn reset_state(&mut self, node: &mut DoraNode) -> Result<()> {
        // Force clear all inputs - don't drain, just reset immediately
        // Any in-flight streaming chunks will be dropped
        for (port_name, input) in self.inputs.iter_mut() {
            if input.is_streaming_active() {
                send_log(
                    node,
                    LogLevel::Info,
                    self.log_level,
                    &format!("🔄 Force clearing active streaming input: {}", port_name),
                );
            }
            input.reset();  // Force clear, don't drain
        }

        self.arrival_queue.clear();
        self.current_question_id = 0;
        self.resume_mode = false;  // Reset to pause mode

        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            "✅ Bridge reset complete - all inputs cleared, ready for new conversation",
        );
        self.send_status(node, "reset")?;
        Ok(())
    }

    fn forward_bundle(&mut self, node: &mut DoraNode) -> Result<()> {
        println!("[BRIDGE-STDOUT] 🚀 forward_bundle() called!");

        // DEBUG: Show all input states
        println!("[BRIDGE-STDOUT] 🔍 INPUT STATES:");
        for (port_name, input) in &self.inputs {
            println!("[BRIDGE-STDOUT]   {}: ready={}, has_message={}",
                     port_name, input.ready, input.get_bundled_message().is_some());
        }

        let ready_inputs = self.get_ready_inputs();
        println!("[BRIDGE-STDOUT] 📋 READY INPUTS: {:?}", ready_inputs);

        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!("🚀 FORWARDING BUNDLE - queue order: {:?}", self.arrival_queue),
        );

        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!("📊 Buffer state: {} inputs accumulated, {} expected ports", self.get_ready_inputs().len(), self.expected_ports.len()),
        );

        // Step 1: Collect messages in FIFO order and concatenate
        let mut concatenated_content = String::new();
        let mut forwarded_count = 0;

        println!("[BRIDGE-STDOUT] 🚀 ARRIVAL QUEUE: {:?}", self.arrival_queue);
        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!("🚀 FORWARDING BUNDLE - queue order: {:?}", self.arrival_queue),
        );

        // Iterate in FIFO queue order (not arbitrary HashMap order)
        for port_name in &self.arrival_queue {
            println!("[BRIDGE-STDOUT] 🔍 Processing queue item: {}", port_name);
            if let Some(input) = self.inputs.get(port_name) {
                println!("[BRIDGE-STDOUT]   Found input - ready: {}, has_message: {}",
                         input.ready, input.get_bundled_message().is_some());
                send_log(
                    node,
                    LogLevel::Info,
                    self.log_level,
                    &format!("🔍 Checking input {} - ready: {}", port_name, input.ready),
                );

                if !input.ready {
                    println!("[BRIDGE-STDOUT]   ⏭️ Skipping - not ready");
                    continue; // Skip if not ready (cold start case)
                }

                // Handle signals based on type - either drop silently or forward template message
                if !input.should_forward {
                    println!("[BRIDGE-STDOUT]   🚫 DROPPING CONTROL SIGNAL from {}: {:?}", port_name, input.signal_type);
                    send_log(
                        node,
                        LogLevel::Info,
                        self.log_level,
                        &format!("🚫 Silently dropping {:?} signal from {}",
                                input.signal_type,
                                port_name),
                    );
                    continue;  // Skip control signals (reset, cancelled)
                }

                // Handle error signals that should be forwarded with template message
                if let Some(signal_type) = &input.signal_type {
                    if matches!(signal_type, SignalType::TechnicalError | SignalType::ContentError) {
                        println!("[BRIDGE-STDOUT]   ⚠️ ERROR SIGNAL from {}: {:?}", port_name, signal_type);

                        // If we have an error message template, create and forward the error message
                        if let Some(template) = &self.error_message_template {
                            // Convert port_name to friendly participant name
                            let participant_name = port_name
                                .replace("llm1", "LLM1")
                                .replace("llm2", "LLM2")
                                .replace("judge", "Judge");

                            let error_message = template.replace("{participant}", &participant_name);

                            println!("[BRIDGE-STDOUT]   📢 Sending error message: '{}'", error_message);
                            send_log(
                                node,
                                LogLevel::Warn,
                                self.log_level,
                                &format!("📢 {} had an error - sending notification: {}", port_name, error_message),
                            );

                            // Add error message to concatenated content
                            if !concatenated_content.is_empty() {
                                concatenated_content.push('\n');
                            }
                            concatenated_content.push_str(&error_message);
                            forwarded_count += 1;
                        } else {
                            // No template - just skip
                            send_log(
                                node,
                                LogLevel::Warn,
                                self.log_level,
                                &format!("❌ Dropping error input from {} - no error message template", port_name),
                            );
                        }
                        continue;
                    }

                    // Skip NormalContent - it will be handled by regular content processing below
                    if matches!(signal_type, SignalType::NormalContent) {
                        // Continue to regular content processing
                    }
                }

                if let Some(message) = input.get_bundled_message() {
                    println!("[BRIDGE-STDOUT]   📦 Got message: {} chars", message.content.len());
                    println!("[BRIDGE-STDOUT]   🔍 Message details: participant='{}', complete={}, content='{}'",
                             message.participant, message.complete, message.content);
                    println!("[BRIDGE-STDOUT]   🔍 Content trimmed: '{}' (empty: {})",
                             message.content.trim(), message.content.trim().is_empty());
                    send_log(
                        node,
                        LogLevel::Info,
                        self.log_level,
                        &format!("📦 Got message from {}: {} chars (complete: {})",
                                message.participant, message.content.len(), message.complete),
                    );

                    // Skip empty messages (completion signals with no content)
                    if message.content.trim().is_empty() {
                        println!("[BRIDGE-STDOUT]   ⚠️ SKIPPING EMPTY MESSAGE from {}", message.participant);
                        send_log(
                            node,
                            LogLevel::Info,
                            self.log_level,
                            &format!("⚠️ Skipping empty message from {}", message.participant),
                        );
                        continue;
                    }

                    send_log(
                        node,
                        LogLevel::Info,
                        self.log_level,
                        &format!(
                            "Adding {} to bundle ({} chars)",
                            message.participant,
                            message.content.len()
                        ),
                    );

                    // Add content with newline separator
                    if !concatenated_content.is_empty() {
                        concatenated_content.push('\n');
                    }
                    concatenated_content.push_str(&message.content);
                    forwarded_count += 1;

                    // Debug log what we're forwarding
                    send_log(
                        node,
                        LogLevel::Info,
                        self.log_level,
                        &format!("📦 Forwarding from {}: {} chars, content: '{}...'", message.participant, message.content.len(), &message.content.chars().take(50).collect::<String>()),
                    );
                }
            }
        }

        if forwarded_count == 0 {
            println!("[BRIDGE-STDOUT] ❌ forward_bundle() FAILED: No messages to forward!");
            send_log(node, LogLevel::Warn, self.log_level, "No messages ready to forward");
            return Ok(());
        }

        // Step 3: Clear the arrival queue and reset input states after forwarding
        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!("🧹 Clearing arrival queue and resetting {} input states", forwarded_count),
        );

        // Clear the arrival queue
        self.arrival_queue.clear();

        // Reset all input states
        for input in self.inputs.values_mut() {
            input.reset();
        }

        // Step 2: Prepare question_id metadata
        let output_question_id = if self.increment_question_id {
            self.current_question_id + 1
        } else {
            self.current_question_id
        };

        let mut output_metadata = BTreeMap::new();
        output_metadata.insert(
            "question_id".to_string(),
            Parameter::String(output_question_id.to_string()),
        );

        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!(
                "🚀 ABOUT TO SEND BUNDLED MESSAGE: {} ports, {} chars, question_id={}",
                forwarded_count,
                concatenated_content.len(),
                output_question_id
            ),
        );

        // CRITICAL DEBUG: Show first 500 chars of what we're about to send
        let content_preview = if concatenated_content.chars().count() > 500 {
            let truncated_chars: String = concatenated_content.chars().take(500).collect();
            format!("{}...(truncated, total {} chars)",
                   truncated_chars,
                   concatenated_content.chars().count())
        } else {
            concatenated_content.clone()
        };
        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!("📤 CONTENT PREVIEW: {}", content_preview),
        );

        // Step 3: Send concatenated output with metadata
        node.send_output(
            DataId::from("text".to_string()),
            output_metadata,
            StringArray::from(vec![concatenated_content.as_str()]),
        )
        .context("Failed to send bundled text output")?;

        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!("✅ SENT: Successfully sent {} chars to text output", concatenated_content.len()),
        );

        // Step 4: Update state
        if self.increment_question_id {
            self.current_question_id = output_question_id;
        }

        self.finalize_cycle(node, "forwarded")
    }
}

fn main() -> Result<()> {
    // Load configuration from environment
    let streaming_ports = env::var("STREAMING_PORTS").ok()
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect::<HashSet<String>>();

    let log_level = env::var("LOG_LEVEL").ok()
        .and_then(|s| LogLevel::parse(&s))
        .unwrap_or(LogLevel::Info);

    let increment_question_id = env::var("INC_QUESTION_ID").ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let (mut node, mut events) =
        DoraNode::init_from_env().context("Failed to initialize Dora node from environment")?;

    let mut expected_ports: HashSet<String> = node
        .node_config()
        .inputs
        .keys()
        .map(|data_id| data_id.to_string())
        .filter(|name| name != "control")
        .collect();
    if expected_ports.is_empty() {
        expected_ports = streaming_ports.clone();
    }

    // Read error message template - {participant} will be replaced with the participant name
    // Example: "{participant} is experiencing technical difficulties. We will proceed without their response."
    let error_message_template = env::var("ERROR_MESSAGE_TEMPLATE").ok();

    let mut bridge = ConferenceBridge::new(
        streaming_ports.clone(),
        expected_ports,
        log_level,
        increment_question_id,
        error_message_template,
    );

    send_log(
        &mut node,
        LogLevel::Info,
        log_level,
        "Conference bridge initialized - forwarding controlled by controller",
    );

    send_log(
        &mut node,
        LogLevel::Info,
        log_level,
        &format!("Increment question_id: {}", increment_question_id),
    );

    if !streaming_ports.is_empty() {
        send_log(
            &mut node,
            LogLevel::Info,
            log_level,
            &format!("Streaming ports: {:?}", streaming_ports),
        );
    } else {
        send_log(
            &mut node,
            LogLevel::Info,
            log_level,
            "No streaming ports - all inputs treated as non-streaming",
        );
    }

    bridge.send_status(&mut node, "waiting")?;

    while let Some(event) = events.recv() {
        // DEBUG: Log EVERY event received at the top of the loop
        match &event {
            Event::Input { id, .. } => {
                println!("[BRIDGE-STDOUT] ⚡ EVENT RECEIVED: Input from '{}'", id.as_str());
            }
            Event::Stop(cause) => {
                println!("[BRIDGE-STDOUT] ⚡ EVENT RECEIVED: Stop({:?})", cause);
            }
            Event::InputClosed { id } => {
                println!("[BRIDGE-STDOUT] ⚡ EVENT RECEIVED: InputClosed({})", id.as_str());
            }
            _ => {
                println!("[BRIDGE-STDOUT] ⚡ EVENT RECEIVED: Other event");
            }
        }

        match event {
            Event::Input { id, data, metadata } => {
                let port_name = id.as_str().to_string();

                if port_name == "control" {
                    println!("[BRIDGE-STDOUT] 🎮 CONTROL EVENT PROCESSING: port={}", port_name);
                    let control_array = data.as_string::<i32>();
                    println!("[BRIDGE-STDOUT] 🎮 CONTROL: control_array len={}", control_array.len());

                    let control_payload = control_array
                        .iter()
                        .filter_map(|value| value.map(str::to_string))
                        .collect::<Vec<String>>()
                        .join(" ");

                    let trimmed = control_payload.trim();
                    println!("[BRIDGE-STDOUT] 🎮 CONTROL PAYLOAD: '{}' (length: {})", trimmed, trimmed.len());

                    // CRITICAL DEBUG: Log ALL control commands
                    send_log(
                        &mut node,
                        LogLevel::Info,
                        log_level,
                        &format!("🎮 CONTROL COMMAND RECEIVED: '{}' (length: {})", trimmed, trimmed.len()),
                    );
                    let mut command: Option<String> = None;

                    if trimmed.is_empty() {
                        println!("[BRIDGE-STDOUT] 🎮 CONTROL: EMPTY PAYLOAD - ignoring!");
                        send_log(
                            &mut node,
                            LogLevel::Warn,
                            log_level,
                            "Received empty control message; ignoring",
                        );
                    } else if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                        if let Some(cmd) = value.get("command").and_then(|v| v.as_str()) {
                            command = Some(cmd.to_ascii_lowercase());
                        }
                    } else {
                        command = Some(trimmed.to_ascii_lowercase());
                    }

                    println!("[BRIDGE-STDOUT] 🎮 CONTROL: command={:?}", command);
                    match command.as_deref() {
                        Some("reset") => {
                            bridge.reset_state(&mut node)?;
                            send_log(
                                &mut node,
                                LogLevel::Info,
                                log_level,
                                "Reset command received - state restored to initial configuration",
                            );
                        }
                        Some("resume") => {
                            // Bridge enters resume mode - will forward completed inputs as they arrive
                            bridge.resume_mode = true;
                            println!("[BRIDGE-STDOUT] 🚀 RESUME: entering resume mode");
                            send_log(
                                &mut node,
                                LogLevel::Info,
                                log_level,
                                "🚀 Resume command received - entering resume mode, will forward completed inputs as they arrive",
                            );

                            // Check if any input is currently streaming (not complete yet)
                            let any_streaming = bridge.inputs.values().any(|input| input.is_streaming_active());

                            let ready_inputs = bridge.get_ready_inputs();
                            println!("[BRIDGE-STDOUT] 🚀 RESUME: ready_inputs={:?}, any_streaming={}, expected_ports={:?}",
                                ready_inputs, any_streaming, bridge.expected_ports);
                            send_log(
                                &mut node,
                                LogLevel::Info,
                                log_level,
                                &format!("🔍 CONTROL INPUT DEBUG - ready_inputs: {:?}, any_streaming: {}, resume_mode: {}", ready_inputs, any_streaming, bridge.resume_mode),
                            );

                            // DEBUG: Show all input states
                            for (port_name, input) in &bridge.inputs {
                                println!("[BRIDGE-STDOUT] 🚀 RESUME INPUT STATE: {}  ready={}, streaming={}",
                                    port_name, input.ready, input.is_streaming_active());
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    &format!("🔍 INPUT STATE - {}: ready={}, streaming={}", port_name, input.ready, input.is_streaming_active()),
                                );
                            }

                            // Forward if there are ready inputs AND no ongoing streaming
                            // If streaming is ongoing, stay in resume_mode and let the input text loop forward when streaming completes
                            if !ready_inputs.is_empty() && !any_streaming {
                                println!("[BRIDGE-STDOUT] 🚀 RESUME: READY TO FORWARD - calling forward_bundle()");
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    &format!("🚀 All inputs complete, calling forward_bundle()"),
                                );
                                match bridge.forward_bundle(&mut node) {
                                    Ok(_) => {
                                        // Only set resume_mode = false AFTER successful forwarding
                                        bridge.resume_mode = false;
                                        send_log(
                                            &mut node,
                                            LogLevel::Info,
                                            log_level,
                                            "✅ forward_bundle() completed successfully on resume - switching to pause mode",
                                        );
                                    }
                                    Err(e) => {
                                        // Keep resume_mode = true on error so we can retry
                                        send_log(
                                            &mut node,
                                            LogLevel::Error,
                                            log_level,
                                            &format!("❌ forward_bundle() failed on resume: {} - staying in resume mode", e),
                                        );
                                    }
                                }
                            } else {
                                println!("[BRIDGE-STDOUT] 🚀 RESUME: NOT READY TO FORWARD (ready={}, streaming={}) - staying in resume mode",
                                    !ready_inputs.is_empty(), any_streaming);
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    &format!("⚠️ Not ready to forward (ready_inputs={}, any_streaming={}) - staying in resume mode", !ready_inputs.is_empty(), any_streaming),
                                );

                                // DON'T switch to pause mode - stay in resume mode to wait for inputs to complete
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    "🔄 Remaining in resume mode - waiting for inputs to complete",
                                );
                            }

                            // resume_mode is set to false inside forward_bundle success block above
                            // Do NOT reset here - only reset after actual forwarding succeeds
                        }
                        Some(other) => {
                            send_log(
                                &mut node,
                                LogLevel::Warn,
                                log_level,
                                &format!("Unknown control command: {}", other),
                            );
                        }
                        None => {}
                    }

                    println!("[BRIDGE-STDOUT] 🎮 CONTROL EVENT DONE - continuing to next event");
                    continue;
                }

                let parameters = metadata.parameters;

                let text_array = data.as_string::<i32>();
                let text = text_array
                    .iter()
                    .filter_map(|value| value.map(str::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");

                println!("[BRIDGE-STDOUT] 📝 TEXT INPUT PROCESSING: port={}, resume_mode={}", port_name, bridge.resume_mode);

                bridge.register_input(port_name.clone());
                let completion_signal = metadata_indicates_completion(&parameters);

                let session_status_value = parameters
                    .get("session_status")
                    .and_then(|param| match param {
                        Parameter::String(status) => Some(status.as_str()),
                        _ => None,
                    });
                let is_starting = session_status_value
                    .map(|status| status.eq_ignore_ascii_case("started"))
                    .unwrap_or(false);
                let has_session_status = session_status_value.is_some();

                if bridge.handle_drain(&port_name, is_starting, completion_signal, has_session_status) {
                    continue;
                }

                if text.trim().is_empty() && !completion_signal {
                    send_log(
                        &mut node,
                        LogLevel::Debug,
                        log_level,
                        &format!("Received empty text from {}, skipping", port_name),
                    );
                    continue;
                }

                send_log(
                    &mut node,
                    LogLevel::Debug,
                    log_level,
                    &format!("Received input from {}: {} chars", port_name, text.len()),
                );

                // Handle the input
                println!("[BRIDGE-STDOUT] 📥 RAW INPUT RECEIVED from {}: '{}' ({} chars)", port_name, text, text.len());

                // Debug: print session_status metadata value
                let session_status_debug = parameters
                    .get("session_status")
                    .map(|p| format!("{:?}", p))
                    .unwrap_or_else(|| "NONE".to_string());
                println!("[BRIDGE-STDOUT] 📋 METADATA session_status from {}: {}", port_name, session_status_debug);

                let input_ready = bridge.handle_input(&port_name, text, parameters);

                if input_ready {
                    // Only log if this input wasn't already ready (deduplication)
                    if let Some(input) = bridge.inputs.get_mut(&port_name) {
                        if !input.was_already_ready {
                            input.was_already_ready = true;

                            // Log differently based on signal type
                            match &input.signal_type {
                                Some(SignalType::ResetSignal) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Info,
                                        log_level,
                                        &format!("🔄 Input {} completed with RESET signal - will be dropped silently (resume_mode: {})", port_name, bridge.resume_mode),
                                    );
                                }
                                Some(SignalType::CancelledSignal) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Info,
                                        log_level,
                                        &format!("🛑 Input {} completed with CANCELLED signal - will be dropped silently (resume_mode: {})", port_name, bridge.resume_mode),
                                    );
                                }
                                Some(SignalType::TechnicalError) | Some(SignalType::ContentError) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Warn,
                                        log_level,
                                        &format!("❌ Input {} completed with ERROR signal ({:?}) - will forward template message (resume_mode: {})", port_name, input.signal_type, bridge.resume_mode),
                                    );
                                }
                                Some(SignalType::NormalContent) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Info,
                                        log_level,
                                        &format!("✅ Input {} completed with normal content (resume_mode: {})", port_name, bridge.resume_mode),
                                    );
                                }
                                None => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Info,
                                        log_level,
                                        &format!("✅ Input {} marked as ready (resume_mode: {})", port_name, bridge.resume_mode),
                                    );
                                }
                            }

                            // DEBUG: Input ready status and resume mode
                            if !bridge.resume_mode {
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    &format!("⏳ Input {} ready but not in resume mode - will forward when resume is received", port_name),
                                );
                            } else {
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    &format!("✅ Input {} ready and resume_mode is true - will forward immediately", port_name),
                                );
                            }
                        } else {
                            // Log that we're skipping the duplicate
                            send_log(
                                &mut node,
                                LogLevel::Debug,
                                log_level,
                                &format!("🔄 Input {} already ready - skipping duplicate log (resume_mode: {})", port_name, bridge.resume_mode),
                            );
                        }
                    }
                } else {
                    // Input not ready - log for debugging
                    send_log(
                        &mut node,
                        LogLevel::Debug,
                        log_level,
                        &format!("⏳ Input {} not ready yet (resume_mode: {})", port_name, bridge.resume_mode),
                    );
                }

                // CRITICAL: If input just completed (became ready) and bridge is in resume mode, check if we can forward
                // Only forward when: resume_mode=true AND this input just became ready AND no other inputs are still streaming
                if bridge.resume_mode && input_ready {
                    let any_streaming = bridge.inputs.values().any(|input| input.is_streaming_active());
                    let ready_inputs = bridge.get_ready_inputs();

                    println!("[BRIDGE-STDOUT] 📥 INPUT COMPLETE in resume_mode: port={}, ready_inputs={:?}, any_streaming={}",
                        port_name, ready_inputs, any_streaming);
                    send_log(
                        &mut node,
                        LogLevel::Info,
                        log_level,
                        &format!("🚀 Input {} completed in resume mode - checking if ready to forward", port_name),
                    );

                    // Only forward if no other inputs are still streaming
                    if !any_streaming && !ready_inputs.is_empty() {
                        println!("[BRIDGE-STDOUT] 📥 FORWARDING from input loop: no streaming, {} ready inputs", ready_inputs.len());
                        send_log(
                            &mut node,
                            LogLevel::Info,
                            log_level,
                            &format!("🔍 Ready to forward: {:?}", ready_inputs),
                        );

                        match bridge.forward_bundle(&mut node) {
                            Ok(_) => {
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    "✅ forward_bundle() completed successfully in input loop",
                                );

                                // Switch back to pause mode only after successful forwarding
                                bridge.resume_mode = false;
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    "⏸️ Forwarding complete - switching back to pause mode (controller will send next resume when appropriate)",
                                );
                            }
                            Err(e) => {
                                send_log(
                                    &mut node,
                                    LogLevel::Error,
                                    log_level,
                                    &format!("❌ forward_bundle() failed in input loop: {}", e),
                                );
                            }
                        }
                    } else {
                        println!("[BRIDGE-STDOUT] 📥 NOT FORWARDING from input loop: any_streaming={}", any_streaming);
                    }
                }

                // Update status
                if bridge.resume_mode {
                    bridge.send_status(&mut node, "resume")?;
                } else {
                    bridge.send_status(&mut node, "waiting")?;
                }
            }
            Event::Stop(_) => {
                send_log(&mut node, LogLevel::Info, log_level, "Received stop event, shutting down");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
