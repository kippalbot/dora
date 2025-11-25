use dora_node_api::{self, DoraNode, Event, Parameter};
use dora_node_api::arrow::array::{StringArray, AsArray};
use dora_node_api::arrow::datatypes::Float64Type;
use dora_conference_controller::policies::{Policy, UnifiedRatioPolicy};
use dora_core::config::DataId;
use eyre::Result;
use std::collections::{HashMap, BTreeMap};
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
    current_question_id: u32,  // Track current conversation question ID
    round_completed: bool,       // Track if current round is complete
    // 🎵 Audio buffer backpressure control
    audio_buffer_paused: bool,  // Whether audio playback is paused due to buffer overflow
    audio_buffer_threshold: f64,  // Pause when buffer > this percentage
    audio_buffer_resume_threshold: f64,  // Resume when buffer < this percentage
    pending_tutor_activation: Option<String>,  // Track deferred tutor turn (control_output name)
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

        // Initialize question_id
        let initial_question_id = env::var("INITIAL_QUESTION_ID")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1); // Default: start at 1

        send_log(node, LogLevel::Info, log_level,
            &format!("🏷️ Starting conversation with question_id: {}", initial_question_id));

        // Log the ready message after all initialization is complete
        send_log(node, LogLevel::Info, log_level, "🚀 all nodes are ready, starting dataflow");

        // 🎵 Load audio buffer thresholds from environment
        let audio_buffer_threshold = env::var("AUDIO_BUFFER_THRESHOLD")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(80.0); // Default: pause at 80% buffer

        let audio_buffer_resume_threshold = env::var("AUDIO_BUFFER_RESUME_THRESHOLD")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(30.0); // Default: resume at 30% buffer

        send_log(node, LogLevel::Info, log_level,
            &format!("🎵 Audio buffer thresholds: pause > {:.1}%, resume < {:.1}%",
                audio_buffer_threshold, audio_buffer_resume_threshold));

        Ok(Self {
            state: ControllerState::Waiting,
            policy,
            participant_inputs: HashMap::new(),
            streaming_accumulators: HashMap::new(),
            pattern,
            log_level,
            reset_pending: false,
            participant_name_map,
            current_question_id: initial_question_id,
            round_completed: false,
            audio_buffer_paused: false,
            audio_buffer_threshold,
            audio_buffer_resume_threshold,
            pending_tutor_activation: None,
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

            // Check if this completes a round (all participants have spoken)
            self.check_round_completion(node);

            self.process_next_speaker(node)?;
        }

        Ok(())
    }

    /// Generate new question_id for next conversation round
    fn generate_new_question_id(&mut self, node: &mut DoraNode) -> u32 {
        self.current_question_id += 1;
        send_log(node, LogLevel::Info, self.log_level,
            &format!("🏷️ New conversation round - question_id: {}", self.current_question_id));
        self.current_question_id
    }

    /// Check if current round is completed and prepare for next round
    fn check_round_completion(&mut self, node: &mut DoraNode) {
        // Check if all participants have completed in this round
        if self.policy.all_participants_completed() {
            if !self.round_completed {
                send_log(node, LogLevel::Info, self.log_level,
                    "📋 All participants completed - round finished");
                self.round_completed = true;

                // Increment cycle counter
                self.policy.increment_cycle();
                let current_cycle = self.policy.get_current_cycle();
                send_log(node, LogLevel::Info, self.log_level,
                    &format!("🔄 Advanced to cycle: {}", current_cycle));

                // Generate new question_id for NEXT round
                let next_question_id = self.current_question_id + 1;
                send_log(node, LogLevel::Info, self.log_level,
                    &format!("🏷️ Next round will use question_id: {}", next_question_id));
            }
        }
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

            // If starting a new round, increment question_id
            if self.round_completed {
                self.generate_new_question_id(node);
                self.round_completed = false;
                // Reset round tracking for the new round
                self.policy.reset_round_tracking();
            }

            // Prepare metadata with current question_id
            let mut metadata = std::collections::BTreeMap::new();
            metadata.insert("question_id".to_string(),
                dora_node_api::Parameter::String(self.current_question_id.to_string()));

            // 🎵 Check audio buffer backpressure for tutor
            let is_tutor = control_output == "control_judge" || next_speaker.contains("tutor") || next_speaker.contains("judge");

            if is_tutor && self.should_pause_tutor_output() {
                // Audio buffer is full - defer tutor activation for retry when buffer drains
                self.pending_tutor_activation = Some(control_output.to_string());

                send_log(node, LogLevel::Info, self.log_level,
                    &format!("🎵 🛑 DEFERRED BRIDGE {}: Audio buffer backpressure (threshold: {:.1}%), will retry when buffer < {:.1}% (question_id: {})",
                        control_output, self.audio_buffer_threshold, self.audio_buffer_resume_threshold, self.current_question_id));

                // Don't send resume yet - wait for buffer to drain
                return Ok(());
            }

            // Clear pending activation when successfully sending resume
            if is_tutor {
                self.pending_tutor_activation = None;
                send_log(node, LogLevel::Info, self.log_level,
                    &format!("🎵 ✅ RESUME BRIDGE {}: Audio buffer safe (question_id: {})",
                        control_output, self.current_question_id));
            }

            send_log(node, LogLevel::Info, self.log_level,
                &format!("🎯 {}: {} → {} (question_id: {})",
                    if self.round_completed { "New Round" } else { "Continue Round" },
                    next_speaker, control_output, self.current_question_id));

            // Send resume WITH controller's question_id
            node.send_output(
                DataId::from(control_output.to_string()),
                metadata,
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
        // Generate new question_id for fresh conversation
        self.generate_new_question_id(node);

        send_log(node, LogLevel::Info, self.log_level, "🔄 Resetting controller");
        self.reset_pending = true;
        self.round_completed = false;

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
        self.policy.reset_round_tracking();  // Reset round tracking
        self.state = ControllerState::Waiting;
        self.pending_tutor_activation = None;  // Clear any pending activation

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

    /// Handle audio buffer status for backpressure control
    fn handle_audio_buffer_status(&mut self, buffer_percentage: f64, node: &mut DoraNode, log_level: LogLevel) -> Result<()> {
        let was_paused = self.audio_buffer_paused;

        // Check if buffer exceeded threshold (need to pause)
        if buffer_percentage > self.audio_buffer_threshold && !self.audio_buffer_paused {
            self.audio_buffer_paused = true;
            send_log(node, LogLevel::Info, log_level,
                &format!("🎵 Audio buffer {:.1}% > {:.1}%: PAUSING tutor output to prevent overflow",
                    buffer_percentage, self.audio_buffer_threshold));
        }
        // Check if buffer dropped below resume threshold (can resume)
        else if buffer_percentage < self.audio_buffer_resume_threshold && self.audio_buffer_paused {
            self.audio_buffer_paused = false;
            send_log(node, LogLevel::Info, log_level,
                &format!("🎵 Audio buffer {:.1}% < {:.1}%: RESUMING tutor output",
                    buffer_percentage, self.audio_buffer_resume_threshold));

            // ✅ Retry pending tutor activation that was deferred due to backpressure
            if let Some(control_output) = &self.pending_tutor_activation {
                let control_output = control_output.clone();
                self.pending_tutor_activation = None;  // Clear before retry

                send_log(node, LogLevel::Info, log_level,
                    &format!("🎵 ✅ RETRY DEFERRED BRIDGE {}: Sending resume (buffer: {:.1}%, question_id: {})",
                        control_output, buffer_percentage, self.current_question_id));

                // Prepare metadata with current question_id
                let mut metadata = std::collections::BTreeMap::new();
                metadata.insert("question_id".to_string(),
                    dora_node_api::Parameter::String(self.current_question_id.to_string()));

                // Send resume signal
                if let Err(e) = node.send_output(
                    DataId::from(control_output.clone()),
                    metadata,
                    StringArray::from(vec!["resume"]),
                ) {
                    send_log(node, LogLevel::Error, log_level,
                        &format!("❌ Failed to send deferred resume to {}: {}", control_output, e));
                }
            }
        }

        // Log status changes for debugging
        if was_paused != self.audio_buffer_paused {
            send_log(node, LogLevel::Info, log_level,
                &format!("🎵 Audio backpressure status changed: {} -> {} (buffer: {:.1}%)",
                    if was_paused { "PAUSED" } else { "ACTIVE" },
                    if self.audio_buffer_paused { "PAUSED" } else { "ACTIVE" },
                    buffer_percentage));
        }

        Ok(())
    }

    /// Check if audio buffer backpressure is preventing tutor output
    fn should_pause_tutor_output(&self) -> bool {
        self.audio_buffer_paused
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
    send_log(&mut node, LogLevel::Info, log_level, "🎵 Audio buffer backpressure control enabled - expecting buffer_status input");
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
                } else if id.as_str() == "buffer_status" {
                    // Handle audio buffer status for backpressure control
                    send_log(&mut node, LogLevel::Info, log_level, "🎵 Received buffer_status input from audio-player");

                    // Audio player sends buffer percentage as a float array with metadata
                    let mut buffer_percentage = 0.0;

                    // Try to get buffer percentage from metadata first (more reliable)
                    if let Some(buffer_val) = metadata.parameters.get("buffer_percentage") {
                        send_log(&mut node, LogLevel::Debug, log_level, &format!("🎵 Found buffer_percentage in metadata: {:?}", buffer_val));
                        if let dora_node_api::Parameter::Float(val) = buffer_val {
                            buffer_percentage = *val;
                        }
                    }

                    // If metadata doesn't have it, try to parse from the data array
                    if buffer_percentage == 0.0 {
                        // The audio player sends a float array: pa.array([buffer_percentage])
                        send_log(&mut node, LogLevel::Debug, log_level, "🎵 Trying to parse buffer from data array");
                        if let Some(buffer_array) = data.as_primitive_opt::<Float64Type>() {
                            if buffer_array.len() > 0 {
                                buffer_percentage = buffer_array.value(0) as f64;
                                send_log(&mut node, LogLevel::Debug, log_level, &format!("🎵 Parsed buffer from array: {}", buffer_percentage));
                            }
                        } else {
                            send_log(&mut node, LogLevel::Warn, log_level, "🎵 Failed to parse float array from audio_buffer_status");
                        }
                    }

                    send_log(&mut node, LogLevel::Info, log_level, &format!("🎵 Audio buffer status: {:.1}%", buffer_percentage));
                    controller.handle_audio_buffer_status(buffer_percentage, &mut node, log_level)?;
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