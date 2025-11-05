use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::env;

use dora_node_api::{
    DoraNode, Event, Parameter,
    arrow::array::{AsArray, StringArray},
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

fn send_log(node: &mut DoraNode, level: LogLevel, config_level: LogLevel, message: &str) {
    if !config_level.allows(level) {
        return;
    }

    let node_identifier = std::env::var("DORA_NODE_NAME")
        .or_else(|_| std::env::var("DORA_NODE_ID"))
        .unwrap_or_else(|_| node.id().to_string());
    let node_identifier = if node_identifier.is_empty() {
        NODE_NAME.to_string()
    } else {
        node_identifier
    };

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

fn send_status(node: &mut DoraNode, status: &str) -> Result<()> {
    node.send_output(
        DataId::from("status".to_string()),
        Default::default(),
        StringArray::from(vec![status]),
    )
    .context("Failed to send status output")?;
    Ok(())
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

    fn force_complete(&mut self) -> String {
        match self {
            MessageState::Streaming { chunks, .. } => {
                let content = chunks.join("");
                *self = MessageState::Complete {
                    content: content.clone(),
                };
                content
            }
            MessageState::Complete { content, .. } => content.clone(),
        }
    }
}

#[derive(Debug)]
struct InputPort {
    port_name: String,
    is_streaming: bool,  // Explicitly configured
    message_state: Option<MessageState>,
    ready: bool,
}

impl InputPort {
    fn new(port_name: String, is_streaming: bool) -> Self {
        Self {
            port_name,
            is_streaming,
            message_state: None,
            ready: false,
        }
    }

    fn is_message_complete(&self, metadata: &BTreeMap<String, Parameter>) -> bool {
        // Check session_status
        if let Some(Parameter::String(status)) = metadata.get("session_status") {
            if status == "ended" {
                return true;
            }
        }

        // Check is_complete
        if let Some(Parameter::Bool(true)) = metadata.get("is_complete") {
            return true;
        }

        false
    }

    fn handle_input(&mut self, text: String, metadata: BTreeMap<String, Parameter>) -> bool {
        if self.is_streaming {
            // Streaming input - wait for complete
            if self.message_state.is_none() {
                self.message_state = Some(MessageState::new_streaming());
            }

            if let Some(state) = &mut self.message_state {
                state.add_chunk(text, metadata.clone());
            }

            // Check if complete
            if self.is_message_complete(&metadata) {
                if let Some(state) = &mut self.message_state {
                    state.complete();
                }
                self.ready = true;
                return true;  // Ready to forward
            }
        } else {
            // Non-streaming input - complete immediately
            self.message_state = Some(MessageState::Complete {
                content: text,
            });
            self.ready = true;
            return true;  // Ready to forward
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
    }

    fn force_ready(&mut self) {
        if let Some(state) = &mut self.message_state {
            state.force_complete();
        } else {
            self.message_state = Some(MessageState::Complete {
                content: String::new(),
            });
        }
        self.ready = true;
    }
}

fn metadata_indicates_completion(metadata: &BTreeMap<String, Parameter>) -> bool {
    match metadata.get("session_status") {
        Some(Parameter::String(status)) if status == "ended" => return true,
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
    cold_start_enabled: bool,
    cold_start_used: bool,
    current_question_id: u32,
   increment_question_id: bool,
    drop_next_bundle: bool,
}

impl ConferenceBridge {
    fn new(
        streaming_ports: HashSet<String>,
        expected_ports: HashSet<String>,
        log_level: LogLevel,
        cold_start_enabled: bool,
        increment_question_id: bool,
    ) -> Self {
        let mut bridge = Self {
            inputs: HashMap::new(),
            streaming_ports,
            expected_ports,
            log_level,
            arrival_queue: VecDeque::new(),
            cold_start_enabled,
            cold_start_used: false,
            current_question_id: 0,
            increment_question_id,
            drop_next_bundle: false,
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
            let ready = input.handle_input(text, metadata);
            if self.drop_next_bundle {
                input.force_ready();
                return true;
            }
            ready
        } else {
            false
        }
    }

    fn should_forward(&self) -> bool {
        // Cold start mode: forward if ANY input is ready (only once)
        if self.cold_start_enabled && !self.cold_start_used {
            return self.inputs.iter().any(|(_, input)| input.ready);
        }

        // Normal bundled mode: ALL inputs must be ready
        if self.inputs.is_empty() {
            return false;
        }

        self.inputs.iter().all(|(_, input)| input.ready)
    }

    fn get_ready_inputs(&self) -> HashSet<String> {
        self.inputs.iter()
            .filter(|(_, input)| input.ready)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn has_active_inputs(&self) -> bool {
        !self.arrival_queue.is_empty()
            || self.inputs
                .values()
                .any(|input| input.ready || input.message_state.is_some())
    }

    fn force_ready_for_drop(&mut self) {
        for (_, input) in self.inputs.iter_mut() {
            input.force_ready();
        }
    }

    fn request_drop(&mut self) -> bool {
        let was_pending = self.drop_next_bundle;
        self.drop_next_bundle = true;
        !was_pending
    }

    fn is_drop_pending(&self) -> bool {
        self.drop_next_bundle
    }

    fn drop_ready_bundle(&mut self, node: &mut DoraNode, ready_inputs: &HashSet<String>) -> Result<()> {
        let mut ordered_inputs = ready_inputs.iter().cloned().collect::<Vec<_>>();
        ordered_inputs.sort();

        send_log(
            node,
            LogLevel::Info,
            self.log_level,
            &format!(
                "Dropping ready bundle instead of forwarding; inputs: {:?}",
                ordered_inputs
            ),
        );

        self.finalize_cycle(node, "dropped", true)
    }

    fn finalize_cycle(&mut self, node: &mut DoraNode, status: &str, retain_drop_flag: bool) -> Result<()> {
        if self.cold_start_enabled && !self.cold_start_used {
            self.cold_start_used = true;
            send_log(
                node,
                LogLevel::Info,
                self.log_level,
                "Cold start used, switching to bundled mode",
            );
        }

        for (_, input) in self.inputs.iter_mut() {
            input.reset();
        }

        self.arrival_queue.clear();
        self.current_question_id = 0;
        if !retain_drop_flag {
            self.drop_next_bundle = false;
        }

        send_status(node, status)?;
        Ok(())
    }

    fn forward_bundle(&mut self, node: &mut DoraNode) -> Result<()> {
        // Step 1: Collect messages in FIFO order and concatenate
        let mut concatenated_content = String::new();
        let mut forwarded_count = 0;

        // Iterate in FIFO queue order (not arbitrary HashMap order)
        for port_name in &self.arrival_queue {
            if let Some(input) = self.inputs.get(port_name) {
                if !input.ready {
                    continue; // Skip if not ready (cold start case)
                }

                if let Some(message) = input.get_bundled_message() {
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
                }
            }
        }

        if forwarded_count == 0 {
            send_log(node, LogLevel::Warn, self.log_level, "No messages ready to forward");
            return Ok(());
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
                "Forwarding bundled message: {} ports, {} chars, question_id={}",
                forwarded_count,
                concatenated_content.len(),
                output_question_id
            ),
        );

        // Step 3: Send concatenated output with metadata
        node.send_output(
            DataId::from("text".to_string()),
            output_metadata,
            StringArray::from(vec![concatenated_content.as_str()]),
        )
        .context("Failed to send bundled text output")?;

        // Step 4: Update state
        if self.increment_question_id {
            self.current_question_id = output_question_id;
        }

        self.finalize_cycle(node, "forwarded", false)
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

    let expected_ports = env::var("EXPECTED_PORTS").ok()
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect::<HashSet<String>>();

    let expected_ports = if expected_ports.is_empty() {
        streaming_ports.clone()
    } else {
        expected_ports
    };

    let log_level = env::var("LOG_LEVEL").ok()
        .and_then(|s| LogLevel::parse(&s))
        .unwrap_or(LogLevel::Info);

    let cold_start = env::var("COLD_START").ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    let increment_question_id = env::var("INC_QUESTION_ID").ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    let mut bridge = ConferenceBridge::new(
        streaming_ports.clone(),
        expected_ports.clone(),
        log_level,
        cold_start,
        increment_question_id,
    );

    let (mut node, mut events) =
        DoraNode::init_from_env().context("Failed to initialize Dora node from environment")?;

    send_log(
        &mut node,
        LogLevel::Info,
        log_level,
        "Conference bridge initialized",
    );

    send_log(
        &mut node,
        LogLevel::Info,
        log_level,
        &format!("Cold start: {}", cold_start),
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

    send_status(&mut node, "waiting")?;

    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, metadata } => {
                let port_name = id.as_str().to_string();

                if port_name == "control" {
                    let control_array = data.as_string::<i32>();
                    let control_payload = control_array
                        .iter()
                        .filter_map(|value| value.map(str::to_string))
                        .collect::<Vec<String>>()
                        .join(" ");

                    let trimmed = control_payload.trim();
                    let mut command: Option<String> = None;

                    if trimmed.is_empty() {
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

                    match command.as_deref() {
                        Some("reset") => {
                            let new_request = bridge.request_drop();
                            if new_request {
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    "Reset command received - dropping next bundle",
                                );
                            } else {
                                send_log(
                                    &mut node,
                                    LogLevel::Info,
                                    log_level,
                                    "Reset command already pending - drop will occur when ready",
                                );
                            }

                            if bridge.has_active_inputs() {
                                bridge.force_ready_for_drop();
                                let ready_inputs = bridge.get_ready_inputs();
                                bridge.drop_ready_bundle(&mut node, &ready_inputs)?;
                            } else {
                                send_status(&mut node, "reset-requested")?;
                            }
                        }
                        Some("resume") => {
                            bridge.drop_next_bundle = false;
                            send_log(
                                &mut node,
                                LogLevel::Info,
                                log_level,
                                "Resume command received - forwarding re-enabled",
                            );
                            let ready_count = bridge.get_ready_inputs().len();
                            let total_count = bridge.inputs.len();
                            send_status(&mut node, &format!("waiting ({}/{})", ready_count, total_count))?;
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

                    continue;
                }

                let parameters = metadata.parameters;

                let text_array = data.as_string::<i32>();
                let text = text_array
                    .iter()
                    .filter_map(|value| value.map(str::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");
                let completion_signal = metadata_indicates_completion(&parameters);

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
                let input_ready = bridge.handle_input(&port_name, text, parameters);

                if input_ready {
                    send_log(
                        &mut node,
                        LogLevel::Debug,
                        log_level,
                        &format!("Input {} marked as ready", port_name),
                    );
                }

                // Check if we should forward
                if bridge.should_forward() {
                    let ready_inputs = bridge.get_ready_inputs();
                    send_log(
                        &mut node,
                        LogLevel::Info,
                        log_level,
                        &format!("All conditions met, ready inputs: {:?}", ready_inputs),
                    );

                    if bridge.is_drop_pending() {
                        bridge.drop_ready_bundle(&mut node, &ready_inputs)?;
                    } else {
                        bridge.forward_bundle(&mut node)?;
                    }
                } else if !bridge.is_drop_pending() {
                    let ready_count = bridge.get_ready_inputs().len();
                    let total_count = bridge.inputs.len();
                    send_status(&mut node, &format!("waiting ({}/{})", ready_count, total_count))?;
                } else {
                    send_log(
                        &mut node,
                        LogLevel::Debug,
                        log_level,
                        "Drop pending - waiting for inputs to complete before clearing",
                    );
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
