use dora_node_api::{self, DoraNode, Event, Parameter};
use dora_node_api::arrow::array::{StringArray, AsArray};
use dora_conference_controller::policies::{Policy, UnifiedRatioPolicy};
use dora_core::config::DataId;
use eyre::Result;
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    let node_name = std::env::var("DORA_NODE_NAME")
        .unwrap_or_else(|_| "conference-controller".to_string());

    let log_data = serde_json::json!({
        "level": format!("{:?}", level).to_uppercase(),
        "message": message,
        "node": node_name,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });

    match node.send_output(
        DataId::from("log".to_string()),
        Default::default(),
        StringArray::from(vec![log_data.to_string().as_str()]),
    ) {
        Ok(_) => {}
        Err(_) => {
            eprintln!("[Controller] Failed to send log: {}", message);
        }
    }
}

#[derive(Debug, Clone)]
struct ParticipantInput {
    id: String,
    text: String,
    timestamp: i64,
    word_count: usize,
    is_complete: bool,
}

#[derive(Debug, Clone)]
struct StreamingAccumulator {
    accumulated_text: String,
    accumulated_words: usize,
}

#[derive(Debug)]
enum ControllerState {
    Waiting,
    Processing,
}

struct ConferenceController {
    state: ControllerState,
    policy: UnifiedRatioPolicy,
    participant_inputs: HashMap<String, ParticipantInput>,
    streaming_accumulators: HashMap<String, StreamingAccumulator>,
    pattern: String,
    log_level: LogLevel,
    reset_pending: bool,  // Track if reset is in progress - ignore incoming "reset" status
    participant_name_map: HashMap<String, String>, // Maps role -> participant ID (e.g., "judge" -> "tutor")
}

impl ConferenceController {
    fn new(pattern: String, node: &mut DoraNode, log_level: LogLevel) -> Result<Self> {
        let mut policy = UnifiedRatioPolicy::new();
        policy.configure(&pattern)
            .map_err(|e| eyre::eyre!("Failed to configure policy from pattern: {}", e))?;

        send_log(node, LogLevel::Info, log_level, &format!("✅ Policy configured with participants: {:?}", policy.get_participants()));
        let stats = policy.get_stats();
        send_log(node, LogLevel::Info, log_level, &format!("📊 Policy configuration:\n{}", serde_json::to_string_pretty(&stats).unwrap()));

        // Initialize participant name mapping
        let participants = policy.get_participants();
        let mut participant_name_map = HashMap::new();

        // Create mapping from role to participant ID
        // This allows the controller to work with different naming schemes
        for participant_id in &participants {
            match participant_id.as_str() {
                "llm1" | "student1" => {
                    participant_name_map.insert("llm1".to_string(), participant_id.clone());
                    participant_name_map.insert("student1".to_string(), participant_id.clone());
                },
                "llm2" | "student2" => {
                    participant_name_map.insert("llm2".to_string(), participant_id.clone());
                    participant_name_map.insert("student2".to_string(), participant_id.clone());
                },
                "judge" | "tutor" => {
                    participant_name_map.insert("judge".to_string(), participant_id.clone());
                    participant_name_map.insert("tutor".to_string(), participant_id.clone());
                },
                _ => {
                    participant_name_map.insert(participant_id.clone(), participant_id.clone());
                }
            }
        }

        send_log(node, LogLevel::Info, log_level, &format!("🔄 Participant name mapping: {:?}", participant_name_map));

        // Log the ready message after all initialization is complete
        send_log(node, LogLevel::Info, log_level, "🚀 all nodes are ready, starting dataflow");

        Ok(Self {
            state: ControllerState::Waiting,
            policy,
            participant_inputs: HashMap::new(),
            streaming_accumulators: HashMap::new(),
            pattern,
            log_level,
            reset_pending: false,
            participant_name_map,
        })
    }

    /// Check if metadata indicates the message is complete
    fn is_message_complete(&self, metadata: &dora_node_api::Metadata) -> bool {
        if let Some(Parameter::String(status)) = metadata.parameters.get("session_status") {
            // ended/complete = normal completion
            // error/cancelled/reset = abnormal completion (also triggers next speaker)
            return status == "ended" || status == "complete"
                || status == "error" || status == "cancelled" || status == "reset";
        }

        // Default to complete if no metadata (non-streaming)
        true
    }

    /// Check if metadata indicates an error occurred
    fn is_error_status(&self, metadata: &dora_node_api::Metadata) -> bool {
        if let Some(Parameter::String(status)) = metadata.parameters.get("session_status") {
            return status == "error" || status == "cancelled" || status == "reset";
        }
        false
    }

    /// Accumulate streaming chunk and return whether message is now complete
    fn accumulate_streaming_input(
        &mut self,
        participant_id: &str,
        text: String,
        metadata: &dora_node_api::Metadata,
    ) -> (String, usize, bool) {
        let is_complete = self.is_message_complete(metadata);
        let word_count = text.split_whitespace().count();

        if !is_complete || self.streaming_accumulators.contains_key(participant_id) {
            // Streaming in progress or we have previous chunks
            let accumulator = self.streaming_accumulators.entry(participant_id.to_string())
                .or_insert_with(|| StreamingAccumulator {
                    accumulated_text: String::new(),
                    accumulated_words: 0,
                });

            if !accumulator.accumulated_text.is_empty() {
                accumulator.accumulated_text.push(' ');
            }
            accumulator.accumulated_text.push_str(&text);
            accumulator.accumulated_words += word_count;

            if is_complete {
                let complete_text = accumulator.accumulated_text.clone();
                let complete_words = accumulator.accumulated_words;
                self.streaming_accumulators.remove(participant_id);
                (complete_text, complete_words, true)
            } else {
                (accumulator.accumulated_text.clone(), accumulator.accumulated_words, false)
            }
        } else {
            // Non-streaming or first complete message
            (text, word_count, true)
        }
    }

    fn handle_participant_input(
        &mut self,
        participant_id: &str,
        text: String,
        metadata: &dora_node_api::Metadata,
        node: &mut DoraNode,
    ) -> Result<()> {
        // Check session_status to understand the input type
        let session_status = metadata.parameters.get("session_status")
            .and_then(|p| match p { Parameter::String(s) => Some(s.as_str()), _ => None });

        // CRITICAL: Check if this is a reset signal from participant output
        // session_status: "reset" from LLM output = LAST message from old debate
        if session_status == Some("reset") {
            send_log(node, LogLevel::Info, self.log_level,
                &format!("🔄 RESET SIGNAL from {} - discarding ALL inputs", participant_id));
            self.participant_inputs.clear();
            self.streaming_accumulators.clear();
            self.state = ControllerState::Waiting;
            self.reset_pending = true;
            return Ok(());
        }

        // If reset_pending is true, ignore ALL inputs EXCEPT "started" status
        if self.reset_pending {
            if session_status == Some("started") {
                send_log(node, LogLevel::Info, self.log_level,
                    &format!("🎬 New debate starting from {}", participant_id));
                self.reset_pending = false;
            } else {
                return Ok(());  // Ignore stale inputs while reset_pending
            }
        }

        // Check if this is an error status
        let is_error = session_status == Some("error") || session_status == Some("cancelled");

        if is_error {
            send_log(node, LogLevel::Warn, self.log_level,
                &format!("❌ {} had an error - proceeding to next speaker", participant_id));

            // Clear any accumulated streaming data for this participant
            self.streaming_accumulators.remove(participant_id);

            // Proceed to next speaker immediately
            self.process_next_speaker(node)?;
            return Ok(());
        }

        // Accumulate streaming chunks and check if message is complete
        let (complete_text, word_count, is_complete) =
            self.accumulate_streaming_input(participant_id, text, metadata);

        // Store the input
        let input = ParticipantInput {
            id: participant_id.to_string(),
            text: complete_text.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            word_count,
            is_complete,
        };
        self.participant_inputs.insert(participant_id.to_string(), input);

        // Always accumulate word counts, but only process when complete
        self.policy.update_word_count(participant_id, word_count);

        if is_complete {
            send_log(node, LogLevel::Info, self.log_level,
                &format!("📥 {} completed ({} words)", participant_id, word_count));
            self.process_next_speaker(node)?;
        }

        Ok(())
    }

    fn process_next_speaker(&mut self, node: &mut DoraNode) -> Result<()> {
        if let Some(next_speaker) = self.policy.determine_next_speaker() {
            // Map the participant ID to the correct control output
            let control_output = match next_speaker.as_str() {
                participant_id if self.participant_name_map.contains_key("judge") &&
                                 self.participant_name_map.get("judge") == Some(&next_speaker) => "control_judge",
                participant_id if self.participant_name_map.contains_key("llm2") &&
                                 self.participant_name_map.get("llm2") == Some(&next_speaker) => "control_llm2",
                participant_id if self.participant_name_map.contains_key("llm1") &&
                                 self.participant_name_map.get("llm1") == Some(&next_speaker) => "control_llm1",
                _ => {
                    // Fallback: try to guess based on naming patterns
                    if next_speaker.contains("judge") || next_speaker.contains("tutor") {
                        "control_judge"
                    } else if next_speaker.contains("llm2") || next_speaker.contains("student2") {
                        "control_llm2"
                    } else if next_speaker.contains("llm1") || next_speaker.contains("student1") {
                        "control_llm1"
                    } else {
                        send_log(node, LogLevel::Warn, self.log_level,
                            &format!("⚠️ Unknown speaker: {}, mapping: {:?}", next_speaker, self.participant_name_map));
                        return Ok(());
                    }
                }
            };

            send_log(node, LogLevel::Info, self.log_level,
                &format!("🎯 Next: {} → {}", next_speaker, control_output));
            node.send_output(
                DataId::from(control_output.to_string()),
                Default::default(),
                StringArray::from(vec!["resume"]),
            )?;
        } else {
            send_log(node, LogLevel::Warn, self.log_level, "⚠️ No next speaker");
        }

        // Send policy statistics
        node.send_output(
            DataId::from("status".to_string()),
            Default::default(),
            StringArray::from(vec![serde_json::to_string(&self.policy.get_stats())?.as_str()]),
        )?;

        Ok(())
    }

    fn reset(&mut self, node: &mut DoraNode) -> Result<()> {
        send_log(node, LogLevel::Info, self.log_level, "🔄 Resetting controller");
        self.reset_pending = true;

        // Send reset to all bridges - use dynamic control outputs
        let control_outputs = vec!["control_judge", "control_llm2", "control_llm1"];
        for output_name in control_outputs {
            node.send_output(
                DataId::from(output_name.to_string()),
                Default::default(),
                StringArray::from(vec!["reset"]),
            )?;
        }

        // Send reset to LLMs and judge
        node.send_output(DataId::from("llm_control".to_string()), Default::default(), StringArray::from(vec!["reset"]))?;
        node.send_output(DataId::from("judge_prompt".to_string()), Default::default(), StringArray::from(vec!["reset"]))?;

        // Reset internal state
        self.participant_inputs.clear();
        self.streaming_accumulators.clear();
        self.policy.reset_counts();
        self.state = ControllerState::Waiting;

        send_log(node, LogLevel::Info, self.log_level, "✅ Reset complete");
        Ok(())
    }

    fn get_stats(&self) -> serde_json::Value {
        let mut stats = self.policy.get_stats();

        if let serde_json::Value::Object(ref mut map) = stats {
            map.insert("input_count".to_string(), serde_json::Value::Number(self.participant_inputs.len().into()));
            map.insert(
                "controller_state".to_string(),
                serde_json::Value::String(format!("{:?}", self.state))
            );
        }

        stats
    }
}

/// Parse command line arguments and YAML configuration
fn load_pattern_from_env() -> Result<String> {
    if let Ok(pattern) = env::var("DORA_POLICY_PATTERN") {
        return Ok(pattern);
    }
    if let Ok(pattern) = env::var("PATTERN") {
        return Ok(pattern);
    }
    Ok("[Judge → Defense → Prosecution]".to_string())
}

fn main() -> Result<()> {
    let pattern = load_pattern_from_env()?;
    let (mut node, events) = DoraNode::init_from_env()?;

    let log_level = env::var("LOG_LEVEL").ok()
        .and_then(|s| LogLevel::parse(&s))
        .unwrap_or(LogLevel::Info);

    send_log(&mut node, LogLevel::Info, log_level, &format!("🚀 Controller started with pattern: {}", pattern));
    let mut controller = ConferenceController::new(pattern, &mut node, log_level)?;

    let mut events = dora_node_api::futures::executor::block_on_stream(events);

    loop {
        let event = events.next();
        match event {
            Some(Event::Input {
                id,
                metadata,
                data,
                ..
            }) => {
                if id.as_str() == "control" {
                    // Extract text from control input
                    let control_array = data.as_string::<i32>();
                    let control_text = control_array
                        .iter()
                        .filter_map(|s| s)
                        .collect::<Vec<_>>()
                        .join(" ");
                    let control_text = control_text.trim();

                    // Try to parse as JSON first
                    let parsed_json: Option<serde_json::Value> = serde_json::from_str(control_text).ok();

                    if let Some(json) = &parsed_json {
                        // Handle JSON control input
                        if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
                            // Forward prompt to judge via llm_control
                            send_log(&mut node, LogLevel::Info, log_level, &format!("📤 Forwarding user prompt to judge: {}", prompt));
                            node.send_output(
                                DataId::from("judge_prompt".to_string()),
                                Default::default(),
                                StringArray::from(vec![control_text]),  // Forward the full JSON
                            )?;
                        } else if let Some(command) = json.get("command").and_then(|v| v.as_str()) {
                            match command.to_lowercase().as_str() {
                                "reset" => controller.reset(&mut node)?,
                                "cancel" => {
                                    // Forward cancel to LLM1/LLM2
                                    node.send_output(
                                        DataId::from("llm_control".to_string()),
                                        Default::default(),
                                        StringArray::from(vec!["cancel"]),
                                    )?;
                                    // Forward cancel to judge
                                    node.send_output(
                                        DataId::from("judge_prompt".to_string()),
                                        Default::default(),
                                        StringArray::from(vec!["cancel"]),
                                    )?;
                                    send_log(&mut node, LogLevel::Info, log_level, "🛑 Sent cancel command to all LLMs");
                                }
                                "stats" => {
                                    let stats = controller.get_stats();
                                    node.send_output(
                                        DataId::from("status".to_string()),
                                        Default::default(),
                                        StringArray::from(vec![serde_json::to_string(&stats)?.as_str()]),
                                    )?;
                                }
                                _ => {
                                    send_log(&mut node, LogLevel::Warn, log_level, &format!("Unknown JSON command: {}", command));
                                }
                            }
                        }
                    } else {
                        // Plain text command (backward compatibility)
                        match control_text.to_lowercase().as_str() {
                            "reset" => {
                                controller.reset(&mut node)?;
                            }
                            "cancel" => {
                                node.send_output(
                                    DataId::from("llm_control".to_string()),
                                    Default::default(),
                                    StringArray::from(vec!["cancel"]),
                                )?;
                                node.send_output(
                                    DataId::from("judge_prompt".to_string()),
                                    Default::default(),
                                    StringArray::from(vec!["cancel"]),
                                )?;
                                send_log(&mut node, LogLevel::Info, log_level, "🛑 Sent cancel command to all LLMs");
                            }
                            "ready" => {
                                node.send_output(
                                    DataId::from("status".to_string()),
                                    Default::default(),
                                    StringArray::from(vec!["ready"]),
                                )?;
                            }
                            "stats" => {
                                let stats = controller.get_stats();
                                node.send_output(
                                    DataId::from("status".to_string()),
                                    Default::default(),
                                    StringArray::from(vec![serde_json::to_string(&stats)?.as_str()]),
                                )?;
                            }
                            _ => {
                                send_log(&mut node, LogLevel::Warn, log_level, &format!("Unknown control command: {}", control_text));
                            }
                        }
                    }
                } else {
                    // Participant input - extract text
                    let text_array = data.as_string::<i32>();
                    let text = text_array
                        .iter()
                        .filter_map(|s| s)
                        .collect::<Vec<_>>()
                        .join(" ");

                    send_log(&mut node, LogLevel::Debug, log_level, &format!("📨 Processing input from {}", id));

                    if let Err(e) = controller.handle_participant_input(id.as_str(), text, &metadata, &mut node) {
                        send_log(&mut node, LogLevel::Error, log_level, &format!("❌ Error handling input: {}", e));
                    }
                }
            }
            Some(Event::Stop(_cause)) => {
                send_log(&mut node, LogLevel::Info, log_level, "🛑 Received stop event, shutting down");
                break;
            }
            Some(Event::Error(e)) => {
                send_log(&mut node, LogLevel::Error, log_level, &format!("❌ Error: {}", e));
            }
            _ => {}
        }
    }

    Ok(())
}