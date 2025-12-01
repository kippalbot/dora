use dora_node_api::{self, DoraNode, Event, Parameter};
use dora_node_api::arrow::array::{StringArray, AsArray};
use dora_node_api::arrow;
use dora_conference_controller::policies::{Policy, UnifiedRatioPolicy};
use dora_core::config::DataId;
use eyre::Result;
use std::collections::HashMap;
use std::env;

// Enhanced Question ID (16-bit: 8-4-4 layout)
// Bits 15-8: Round number (0-255)
// Bits 7-4: Total participants (1-16, stored as total-1)
// Bits 3-0: Current participant (0-15)
fn encode_enhanced_question_id(round: u8, participant: u8, total_participants: u8) -> u16 {
    let round_bits = (round as u16) << 8;
    let total_bits = ((total_participants - 1) as u16) << 4;
    let participant_bits = participant as u16;

    round_bits | total_bits | participant_bits
}

fn decode_enhanced_question_id(question_id: u16) -> (u8, u8, u8, bool) {
    let round = (question_id >> 8) as u8;
    let total_participants = ((question_id >> 4) & 0xF) + 1;
    let participant = (question_id & 0xF) as u8;
    let is_last_participant = participant + 1 == total_participants as u8;

    (round, participant, total_participants as u8, is_last_participant)
}

fn enhanced_id_debug_string(question_id: u16) -> String {
    let (round, participant, total, is_last) = decode_enhanced_question_id(question_id);
    format!("R{}P{}/{}{}", round + 1, participant + 1, total, if is_last {"[LAST]"} else {""})
}

fn is_last_participant(question_id: u16) -> bool {
    let (_, _, _, is_last) = decode_enhanced_question_id(question_id);
    is_last
}

fn get_round_number(question_id: u16) -> u8 {
    (question_id >> 8) as u8 + 1
}


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
    participant_index_map: HashMap<String, u8>,  // Maps participant ID -> index (0-based)
    current_question_id: u16,  // Track current conversation question ID (enhanced 16-bit format)

    // New session-start based resume control
    waiting_for_session_start: Option<u16>,  // Question ID we're waiting for
    pending_next_speaker: bool,              // Flag that next speaker should be determined after session_start
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

        // Initialize participant index mapping (0-based)
        let mut participant_index_map = HashMap::new();
        for (index, participant_id) in participants.iter().enumerate() {
            participant_index_map.insert(participant_id.clone(), index as u8);
            send_log(node, LogLevel::Debug, log_level,
                &format!("📍 Participant index mapping: {} -> {}", participant_id, index));
        }

        // Initialize enhanced question_id for round 1
        let initial_round = 0; // 0-based for encoding
        let total_participants = participants.len() as u8;
        // Start with first participant (index 0)
        let initial_enhanced_id = encode_enhanced_question_id(initial_round, 0, total_participants);

        send_log(node, LogLevel::Info, log_level,
            &format!("🏷️ Starting with enhanced question_id: {} ({})",
                initial_enhanced_id, enhanced_id_debug_string(initial_enhanced_id)));

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
            participant_index_map,
            current_question_id: initial_enhanced_id,
            waiting_for_session_start: None,  // Cold start - no waiting initially
            pending_next_speaker: false,
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

            // Process next speaker (will wait for session_start if needed)
            self.process_next_speaker(node)?;
        }

        Ok(())
    }

    /// Generate new question_id for next conversation round
    /// Handle session_start signals from audio player
    fn handle_session_start(&mut self, question_id: u16, node: &mut DoraNode, log_level: LogLevel) -> Result<()> {
        let (cycle, participant, total, _) = decode_enhanced_question_id(question_id);

        send_log(node, LogLevel::Info, log_level,
            &format!("🎬 Session start: {} ({})",
                question_id, enhanced_id_debug_string(question_id)));

        // Check if this is the session_start we're waiting for
        if self.waiting_for_session_start == Some(question_id) {
            send_log(node, LogLevel::Info, log_level,
                "✅ Participant audio started - ready for next speaker");

            // Clear waiting state
            self.waiting_for_session_start = None;

            // If we have a pending next speaker request, process it now
            if self.pending_next_speaker {
                send_log(node, LogLevel::Info, log_level,
                    "🔄 Processing pending next speaker");
                self.pending_next_speaker = false;
                self.process_next_speaker(node)?;
            }
        } else {
            send_log(node, LogLevel::Debug, log_level,
                &format!("📝 Session start for question_id={} (not waiting for this one)", question_id));
        }

        Ok(())
    }

    fn process_next_speaker(&mut self, node: &mut DoraNode) -> Result<()> {
        // Check if we're waiting for a session_start
        if self.waiting_for_session_start.is_some() {
            send_log(node, LogLevel::Debug, self.log_level,
                "⏳ Waiting for session_start - marking pending");
            // Mark that we need to process next speaker after session_start arrives
            self.pending_next_speaker = true;
            return Ok(());
        }

        // Cold start or session_start already received - proceed immediately
        if let Some(next_speaker) = self.policy.determine_next_speaker() {
            // Map the participant ID to the correct control output (convert to owned String)
            let control_output = self.get_control_output(&next_speaker).to_string();

            // Generate question_id for this participant using cycle# as round#
            let cycle = self.policy.get_current_cycle() as u8;
            let participant_index = self.get_participant_index(&next_speaker);
            let total_participants = self.policy.get_participants().len() as u8;
            self.current_question_id = encode_enhanced_question_id(cycle, participant_index, total_participants);

            // Increment cycle counter
            self.policy.increment_cycle();

            // Prepare metadata with enhanced question_id
            let mut metadata = std::collections::BTreeMap::new();
            metadata.insert("question_id".to_string(),
                dora_node_api::Parameter::String(self.current_question_id.to_string()));

            send_log(node, LogLevel::Info, self.log_level,
                &format!("🎯 Resume: {} → {} (question_id: {}, cycle: {})",
                    next_speaker, control_output, self.current_question_id, cycle));

            // Send resume WITH controller's question_id
            node.send_output(
                DataId::from(control_output.to_string()),
                metadata,
                StringArray::from(vec!["resume"]),
            )?;

            // Now wait for this participant's session_start before next resume
            self.waiting_for_session_start = Some(self.current_question_id);

            send_log(node, LogLevel::Debug, self.log_level,
                &format!("⏳ Now waiting for session_start for question_id={}", self.current_question_id));
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

    /// Get control output name for a participant
    fn get_control_output(&self, participant: &str) -> &str {
        if self.participant_name_map.contains_key("judge") &&
           self.participant_name_map.get("judge") == Some(&participant.to_string()) {
            "control_judge"
        } else if self.participant_name_map.contains_key("llm2") &&
                  self.participant_name_map.get("llm2") == Some(&participant.to_string()) {
            "control_llm2"
        } else if self.participant_name_map.contains_key("llm1") &&
                  self.participant_name_map.get("llm1") == Some(&participant.to_string()) {
            "control_llm1"
        } else {
            // Fallback: try to guess based on naming patterns
            if participant.contains("judge") || participant.contains("tutor") {
                "control_judge"
            } else if participant.contains("llm2") || participant.contains("student2") {
                "control_llm2"
            } else {
                "control_llm1"
            }
        }
    }

    /// Get participant index (0-based) for question_id encoding
    fn get_participant_index(&self, participant: &str) -> u8 {
        *self.participant_index_map.get(participant).unwrap_or(&0)
    }

    /// Advance to next round after session start of first participant
    fn reset(&mut self, node: &mut DoraNode) -> Result<()> {
        // Generate new question_id for fresh conversation - start with round 0, participant 0
        self.current_question_id = encode_enhanced_question_id(0, 0, 1);

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
        self.waiting_for_session_start = None;
        self.pending_next_speaker = false;
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
                // Debug: Log all incoming event IDs
                send_log(&mut node, LogLevel::Debug, log_level, &format!("📨 Received event from input: '{}'", id.as_str()));

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
                            // Forward prompt to judge via llm_control with question_id metadata
                            send_log(&mut node, LogLevel::Info, log_level,
                                &format!("📤 Forwarding user prompt to judge with question_id={} ({}): {}",
                                    controller.current_question_id,
                                    enhanced_id_debug_string(controller.current_question_id),
                                    prompt));

                            // Create metadata with question_id
                            let mut metadata = std::collections::BTreeMap::new();
                            metadata.insert(
                                "question_id".to_string(),
                                Parameter::String(controller.current_question_id.to_string())
                            );

                            node.send_output(
                                DataId::from("judge_prompt".to_string()),
                                metadata,
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
                } else if id.as_str() == "session_start" {
                    // Handle session start signal from audio player
                    // When we receive session_start for first participant of a round,
                    // it means we can advance to that round
                    send_log(&mut node, LogLevel::Info, log_level, "🎬 Received session_start input from audio player");

                    // Read the data (session_status string) - we don't use it but need to consume it
                    let _session_status_array = data.as_string::<i32>();

                    // Get question_id from metadata
                    let question_id = if let Some(Parameter::String(qid_str)) = metadata.parameters.get("question_id") {
                        match qid_str.parse::<u16>() {
                            Ok(qid) => {
                                if qid == 0 {
                                    send_log(&mut node, LogLevel::Error, log_level, "❌ Invalid question_id=0 in session_start signal - ignoring");
                                    continue;
                                }
                                qid
                            }
                            Err(e) => {
                                send_log(&mut node, LogLevel::Error, log_level, &format!("❌ Failed to parse question_id '{}' in session_start signal: {}", qid_str, e));
                                continue;
                            }
                        }
                    } else {
                        send_log(&mut node, LogLevel::Warn, log_level, "⚠️ Session start signal missing question_id metadata");
                        continue;
                    };

                    // Handle session start for round advancement
                    if let Err(e) = controller.handle_session_start(question_id, &mut node, log_level) {
                        send_log(&mut node, LogLevel::Error, log_level, &format!("❌ Error handling session start: {}", e));
                    }
                } else if id.as_str() == "buffer_status" {
                    // Buffer status from audio player - we don't use this anymore
                    // Just consume it to avoid crashes
                    let _buffer_data = data.as_primitive::<arrow::datatypes::Float64Type>();
                    send_log(&mut node, LogLevel::Debug, log_level, "📊 Received buffer_status (ignored)");
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