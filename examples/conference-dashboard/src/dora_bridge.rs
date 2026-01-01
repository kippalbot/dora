//! Dora Bridge - Receives data from Dora dataflow and updates shared state
//!
//! Handles inputs from mac-aec voice chat pipeline:
//! - audio: TTS audio for playback and waveform visualization
//! - speech_started/speech_ended: VAD signals from mac-aec
//! - transcription: ASR output
//! - llm_text/llm_status: LLM response streaming
//! - reset: Reset signal from mac-aec/question_ended
//! - *_log: Log messages from all nodes
//!
//! Sends outputs:
//! - buffer_status: Audio buffer fill percentage for backpressure

use crate::{SharedStateRef, LogMessage, ControlCommand};
use crate::audio_player::AudioPlayerRef;
use dora_node_api::{ArrowData, Metadata, IntoArrow, Parameter};
use dora_node_api::arrow::array::{Array, Float64Array, Float32Array, Int64Array, Int32Array, StringArray, LargeStringArray, UInt8Array};
use dora_node_api::arrow::datatypes::DataType;
use dora_node_api::dora_core::config::{NodeId, DataId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use std::collections::{HashMap, BTreeMap};

/// Shutdown flag for graceful termination
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Signal shutdown to the bridge
pub fn shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

/// Run the Dora bridge that receives events and updates shared state
///
/// If audio_player is Some, audio will be played through it and buffer_status will be sent.
/// If audio_player is None, runs in visualization-only mode (no audio playback).
pub fn run_dora_bridge(
    shared_state: SharedStateRef,
    audio_player: Option<AudioPlayerRef>,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting Dora bridge...");

    if audio_player.is_some() {
        log::info!("Audio playback enabled");
    } else {
        log::info!("Running in visualization-only mode (no audio playback)");
    }

    // Check for node name from environment
    let node_name = std::env::var("DORA_NODE_ID")
        .or_else(|_| std::env::var("DORA_NODE_NAME"))
        .ok();

    // Try to create Dora node
    let result = if let Some(name) = node_name {
        log::info!("Connecting as dynamic node: {}", name);
        dora_node_api::DoraNode::init_from_node_id(NodeId::from(name))
    } else {
        dora_node_api::DoraNode::init_from_env()
    };

    match result {
        Ok((node, events)) => {
            log::info!("Dora node initialized, listening for events...");
            // Set connected status
            {
                let mut state = shared_state.lock();
                state.is_connected = true;
            }
            run_dora_loop(shared_state, audio_player, node, events)
        }
        Err(e) => {
            log::warn!("Failed to initialize Dora node: {}. Running in demo mode.", e);
            // Not connected in demo mode
            {
                let mut state = shared_state.lock();
                state.is_connected = false;
            }
            run_demo_mode(shared_state, audio_player);
            Ok(())
        }
    }
}

fn run_dora_loop(
    shared_state: SharedStateRef,
    audio_player: Option<AudioPlayerRef>,
    mut node: dora_node_api::DoraNode,
    mut events: dora_node_api::EventStream,
) -> Result<(), Box<dyn std::error::Error>> {
    use dora_node_api::Event;

    let mut segment_counts: HashMap<String, u32> = HashMap::new();
    let mut last_buffer_status_time = Instant::now();
    let buffer_status_interval = std::time::Duration::from_millis(50); // Fast updates for speaker switching

    // Track if we've sent session_start for current audio session
    let mut session_start_sent = false;
    let mut current_question_id: Option<String> = None;

    // Smart reset: filter audio by question_id after reset
    let mut filtering_mode = false;
    let mut reset_question_id: Option<String> = None;

    // Streaming state tracking per participant (for timeout auto-complete)
    struct StreamingState {
        content: String,
        active: bool,
        last_chunk_time: Instant,
    }
    let mut streaming_state: HashMap<String, StreamingState> = HashMap::new();
    let streaming_timeout = std::time::Duration::from_secs(2);

    // Detect study mode vs debate mode
    let study_mode = std::env::var("DORA_STUDY_MODE")
        .map(|v| v.to_lowercase() == "true" || v == "1" || v.to_lowercase() == "yes")
        .unwrap_or(false);

    // Read participant names from environment (can be configured in YAML)
    let participant1_name = std::env::var("PARTICIPANT1_NAME")
        .unwrap_or_else(|_| if study_mode { "Student 1".to_string() } else { "Daniu".to_string() });
    let participant2_name = std::env::var("PARTICIPANT2_NAME")
        .unwrap_or_else(|_| if study_mode { "Student 2".to_string() } else { "Yifei".to_string() });
    let participant3_name = std::env::var("PARTICIPANT3_NAME")
        .unwrap_or_else(|_| if study_mode { "Tutor".to_string() } else { "Laoshi".to_string() });

    // Get log level threshold from environment (default: INFO)
    let log_level_threshold = std::env::var("LOG_LEVEL")
        .map(|v| match v.to_uppercase().as_str() {
            "DEBUG" => 10,
            "INFO" => 20,
            "WARNING" | "WARN" => 30,
            "ERROR" => 40,
            _ => 20, // default to INFO
        })
        .unwrap_or(20);

    // Check if console output is enabled
    let console_output = std::env::var("DASHBOARD_CONSOLE_LOG")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(true); // Default to true

    log::info!("Participants: {}, {}, {}", participant1_name, participant2_name, participant3_name);
    log::info!("Log level threshold: {} (DEBUG=10, INFO=20, WARNING=30, ERROR=40)", log_level_threshold);

    loop {
        // Use recv_timeout to allow processing commands even without incoming events
        let event = events.recv_timeout(std::time::Duration::from_millis(100));

        // Check for streaming timeouts (auto-complete after 2s of no chunks)
        let now = Instant::now();
        let timed_out: Vec<String> = streaming_state
            .iter()
            .filter(|(_, state)| state.active && now.duration_since(state.last_chunk_time) > streaming_timeout)
            .map(|(k, _)| k.clone())
            .collect();

        for participant in timed_out {
            if let Some(state) = streaming_state.get_mut(&participant) {
                if !state.content.is_empty() {
                    log::debug!("Streaming timeout for {}, auto-completing", participant);
                    // Update UI with final content
                    let idx = extract_participant_index(&participant);
                    let mut shared = shared_state.lock();
                    if let Some(p) = shared.participants.get_mut(idx) {
                        p.is_speaking = false;
                        p.status = "Complete".to_string();
                        let count = segment_counts.entry(participant.clone()).or_insert(0);
                        *count += 1;
                        p.segment_count = *count;
                    }
                }
                state.content.clear();
                state.active = false;
            }
        }

        // Periodically send buffer status if we have an audio player
        if let Some(ref player) = audio_player {
            if last_buffer_status_time.elapsed() >= buffer_status_interval {
                let buffer_pct = player.buffer_fill_percentage();
                let buffer_secs = player.buffer_seconds();

                // Update shared state
                {
                    let mut state = shared_state.lock();
                    state.buffer_fill = buffer_pct;
                    state.buffer_seconds = buffer_secs;
                    state.playback_status.is_playing = player.is_playing();
                    // Update active participant from audio player (tracks what's actually playing)
                    state.playback_status.active_participant_idx = player.current_participant_idx();

                    // Update waveform from audio player
                    state.waveform_data = player.get_waveform_data(512);
                }

                // Send buffer_status output for backpressure control
                if let Err(e) = node.send_output(
                    DataId::from("buffer_status".to_string()),
                    Default::default(),
                    vec![buffer_pct].into_arrow(),
                ) {
                    log::warn!("Failed to send buffer_status: {}", e);
                } else {
                    log::debug!("Buffer status: {:.1}% ({:.1}s)", buffer_pct, buffer_secs);
                }

                last_buffer_status_time = Instant::now();
            }
        }

        // Process control commands from UI
        let commands: Vec<ControlCommand> = {
            let mut state = shared_state.lock();
            std::mem::take(&mut state.control_commands)
        };

        for cmd in commands {
            let control_msg = match cmd {
                ControlCommand::Reset => {
                    // Reset audio player
                    if let Some(ref player) = audio_player {
                        player.reset();
                    }
                    session_start_sent = false;
                    // Clear conversation history, logs, and chat messages
                    {
                        let mut state = shared_state.lock();
                        for p in state.participants.iter_mut() {
                            p.conversation_history.clear();
                            p.last_text.clear();
                            p.segment_count = 0;
                        }
                        state.log_messages.clear();
                        state.chat_messages.clear();
                    }
                    current_question_id = None;
                    "reset".to_string()
                }
                ControlCommand::StartQuestion(qid) => {
                    // Reset for new question
                    if let Some(ref player) = audio_player {
                        player.reset();
                    }
                    session_start_sent = false;
                    current_question_id = Some(qid.to_string());
                    format!("start_question:{}", qid)
                }
                ControlCommand::Pause => {
                    if let Some(ref player) = audio_player {
                        player.pause();
                    }
                    "pause".to_string()
                }
                ControlCommand::Resume => {
                    if let Some(ref player) = audio_player {
                        player.resume();
                    }
                    "resume".to_string()
                }
                ControlCommand::NextRound => {
                    "next_round".to_string()
                }
                ControlCommand::SendPrompt(prompt) => {
                    // Format as JSON like debate_monitor.py does
                    format!("{{\"prompt\": \"{}\"}}", prompt.replace("\"", "\\\""))
                }
            };

            // Send control output to conference-controller
            if let Err(e) = node.send_output(
                DataId::from("control".to_string()),
                Default::default(),
                vec![control_msg.clone()].into_arrow(),
            ) {
                log::warn!("Failed to send control command '{}': {}", control_msg, e);
            } else {
                log::info!("Sent control command: {}", control_msg);
            }
        }

        // Handle event if we received one (not a timeout)
        let Some(event) = event else {
            // Timeout - just continue to process commands and check shutdown
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }
            continue;
        };

        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.as_str();

                // Log all incoming inputs at INFO level for debugging
                log::info!("📥 Input: {} (type: {:?}, len: {})",
                    input_id,
                    data.data_type(),
                    data.len());

                // === Reset signal (from control_judge in conference mode) ===
                if input_id == "reset" || input_id.contains("question_ended") || input_id == "control" {
                    // Check if this is a reset command
                    let is_reset = if input_id == "control" {
                        extract_string(&data)
                            .map(|s| s.to_lowercase().contains("reset") || s.to_lowercase().contains("cancel"))
                            .unwrap_or(false)
                    } else {
                        true
                    };

                    if is_reset {
                        // Extract new question_id from metadata for smart reset
                        let new_question_id = get_metadata_string(&metadata, "question_id");

                        if let Some(ref qid) = new_question_id {
                            // Smart reset - enable filtering mode
                            log::info!("Smart reset with question_id={}", qid);
                            reset_question_id = Some(qid.clone());
                            filtering_mode = true;
                        } else {
                            // Full reset - no question_id filtering
                            log::info!("Full reset - clearing everything");
                            filtering_mode = false;
                            reset_question_id = None;
                        }

                        if let Some(ref player) = audio_player {
                            player.reset();
                        }

                        // Reset session tracking
                        session_start_sent = false;
                        current_question_id = None;

                        // Clear streaming state
                        streaming_state.clear();

                        // Clear all participant text on reset
                        let mut state = shared_state.lock();
                        for p in state.participants.iter_mut() {
                            p.last_text.clear();
                            p.is_speaking = false;
                            p.status = "Idle".to_string();
                        }
                    }
                }
                // === Speech Detection (from mac-aec) ===
                else if input_id == "speech_started" {
                    let mut state = shared_state.lock();
                    // User is participant 0 in single-user mode
                    if let Some(p) = state.participants.get_mut(0) {
                        p.is_speaking = true;
                        p.status = "Speaking".to_string();
                        p.name = "User".to_string();
                    }
                }
                else if input_id == "speech_ended" {
                    let mut state = shared_state.lock();
                    if let Some(p) = state.participants.get_mut(0) {
                        p.is_speaking = false;
                        p.status = "Idle".to_string();
                    }
                }
                // === ASR Transcription (from asr) ===
                else if input_id == "transcription" {
                    if let Some(text) = extract_string(&data) {
                        let mut state = shared_state.lock();
                        // User transcription goes to participant 0
                        if let Some(p) = state.participants.get_mut(0) {
                            p.last_text = text.clone();
                            let count = segment_counts.entry("user".to_string()).or_insert(0);
                            *count += 1;
                            p.segment_count = *count;
                        }
                        // Also add to log
                        state.log_messages.push(LogMessage {
                            timestamp: get_timestamp(),
                            level: "INFO".to_string(),
                            source: "ASR".to_string(),
                            message: format!("User: {}", text),
                        });
                        if state.log_messages.len() > 8000 {
                            state.log_messages.remove(0);
                        }
                    }
                }
                // === LLM Text (from maas-client) ===
                else if input_id == "llm_text" {
                    if let Some(text) = extract_string(&data) {
                        let session_status = get_metadata_string(&metadata, "session_status");
                        let mut state = shared_state.lock();
                        // LLM response goes to participant 1 (Assistant)
                        if let Some(p) = state.participants.get_mut(1) {
                            p.name = "Assistant".to_string();
                            match session_status.as_deref() {
                                Some("started") => {
                                    p.last_text = text;
                                    p.is_speaking = true;
                                    p.status = "Generating".to_string();
                                }
                                Some("ongoing") => {
                                    p.last_text.push_str(&text);
                                    p.is_speaking = true;
                                }
                                Some("ended") => {
                                    if !text.is_empty() {
                                        p.last_text = text;
                                    }
                                    p.is_speaking = false;
                                    p.status = "Complete".to_string();
                                    let count = segment_counts.entry("llm".to_string()).or_insert(0);
                                    *count += 1;
                                    p.segment_count = *count;
                                }
                                _ => {
                                    p.last_text = text;
                                }
                            }
                        }
                    }
                }
                // === LLM Status (from maas-client) ===
                else if input_id == "llm_status" {
                    if let Some(status) = extract_string(&data) {
                        let mut state = shared_state.lock();
                        if let Some(p) = state.participants.get_mut(1) {
                            p.status = status.clone();
                            p.is_speaking = status == "generating" || status == "streaming";
                        }
                    }
                }
                // === Participant text ===
                // Handles both naming conventions:
                // - Conference style: student1_text, student2_text, tutor_text
                // - Debate style: llm1_text, llm2_text, judge_text
                else if input_id.ends_with("_text") && !input_id.contains("bridge") {
                    if let Some(text) = extract_string(&data) {
                        let session_status = get_metadata_string(&metadata, "session_status");
                        let idx = extract_participant_index(input_id);
                        let participant_key = input_id.to_string();

                        log::info!("👤 Participant text [{}] idx={} status={:?}: {}",
                            input_id, idx, session_status, truncate_text(&text, 50));

                        // Track streaming state for timeout handling
                        let stream_state = streaming_state.entry(participant_key.clone()).or_insert_with(|| {
                            StreamingState {
                                content: String::new(),
                                active: false,
                                last_chunk_time: Instant::now(),
                            }
                        });

                        // Collect chat message info before borrowing state
                        let mut chat_msg_info: Option<(String, String)> = None;

                        {
                            let mut state = shared_state.lock();
                            if let Some(p) = state.participants.get_mut(idx) {
                                // Set participant name from environment variables
                                if input_id.contains("student1") || input_id.contains("llm1") {
                                    p.name = participant1_name.clone();
                                } else if input_id.contains("student2") || input_id.contains("llm2") {
                                    p.name = participant2_name.clone();
                                } else if input_id.contains("tutor") || input_id.contains("judge") {
                                    p.name = participant3_name.clone();
                                }

                                // Handle streaming status
                                match session_status.as_deref() {
                                    Some("started") => {
                                        p.last_text = text.clone();
                                        p.is_speaking = true;
                                        p.status = "Speaking".to_string();
                                        stream_state.content = text;
                                        stream_state.active = true;
                                        stream_state.last_chunk_time = Instant::now();
                                    }
                                    Some("ongoing") => {
                                        p.last_text.push_str(&text);
                                        p.is_speaking = true;
                                        stream_state.content.push_str(&text);
                                        stream_state.last_chunk_time = Instant::now();
                                    }
                                    Some("ended") => {
                                        if !text.is_empty() {
                                            p.last_text = text;
                                        }
                                        p.is_speaking = false;
                                        p.status = "Complete".to_string();
                                        let count = segment_counts.entry(input_id.to_string()).or_insert(0);
                                        *count += 1;
                                        p.segment_count = *count;
                                        // Append complete message to conversation history
                                        if !p.last_text.is_empty() {
                                            if !p.conversation_history.is_empty() {
                                                p.conversation_history.push_str("\n\n");
                                            }
                                            p.conversation_history.push_str(&p.last_text);
                                            // Collect info for chat message
                                            chat_msg_info = Some((p.name.clone(), p.last_text.clone()));
                                        }
                                        stream_state.content.clear();
                                        stream_state.active = false;
                                    }
                                    _ => {
                                        // No session_status - treat as complete message
                                        p.last_text = text.clone();
                                        p.is_speaking = true;
                                        stream_state.content = text.clone();
                                        stream_state.active = true;
                                        stream_state.last_chunk_time = Instant::now();
                                        let count = segment_counts.entry(input_id.to_string()).or_insert(0);
                                        *count += 1;
                                        p.segment_count = *count;
                                        // Append to conversation history
                                        if !text.is_empty() {
                                            if !p.conversation_history.is_empty() {
                                                p.conversation_history.push_str("\n\n");
                                            }
                                            p.conversation_history.push_str(&text);
                                        }
                                    }
                                }
                            }
                        }

                        // Add chat message after releasing participant borrow
                        if let Some((sender, msg_text)) = chat_msg_info {
                            let mut state = shared_state.lock();
                            state.chat_messages.push(crate::ChatMessage {
                                sender,
                                text: msg_text,
                                timestamp: get_timestamp(),
                            });
                            if state.chat_messages.len() > 500 {
                                state.chat_messages.remove(0);
                            }
                        }
                    }
                }
                // === Participant status (conference mode) ===
                else if input_id.ends_with("_status") && !input_id.contains("controller") && !input_id.contains("segmenter") && !input_id.contains("tts") {
                    if let Some(status) = extract_string(&data) {
                        let idx = extract_participant_index(input_id);
                        let mut state = shared_state.lock();
                        if let Some(p) = state.participants.get_mut(idx) {
                            p.status = status.clone();
                            p.is_speaking = status == "generating" || status == "streaming";
                        }
                    }
                }
                // === Participant prompts (from bridges) ===
                // Handles both naming conventions:
                // - Conference style: student1_prompt, student2_prompt, tutor_prompt
                // - Debate style: llm1_prompt, llm2_prompt, judge_prompt
                else if input_id.ends_with("_prompt") {
                    // Prompts show what was sent TO the participant, add to log
                    if let Some(text) = extract_string(&data) {
                        let participant = if input_id.contains("student1") || input_id.contains("llm1") {
                            participant1_name.as_str()
                        } else if input_id.contains("student2") || input_id.contains("llm2") {
                            participant2_name.as_str()
                        } else if input_id.contains("tutor") || input_id.contains("judge") {
                            participant3_name.as_str()
                        } else {
                            "Unknown"
                        };
                        let mut state = shared_state.lock();
                        state.log_messages.push(LogMessage {
                            timestamp: get_timestamp(),
                            level: "INFO".to_string(),
                            source: "Bridge".to_string(),
                            message: format!("→ {}: {}", participant, truncate_text(&text, 80)),
                        });
                        if state.log_messages.len() > 8000 {
                            state.log_messages.remove(0);
                        }

                        // Add to chat messages for tutor/judge prompts (conversation display)
                        if input_id.contains("tutor") || input_id.contains("judge") {
                            // This is what's being sent TO the tutor (context from other participants)
                            state.chat_messages.push(crate::ChatMessage {
                                sender: "Context".to_string(),
                                text: text.clone(),
                                timestamp: get_timestamp(),
                            });
                            // Keep chat messages limited
                            if state.chat_messages.len() > 500 {
                                state.chat_messages.remove(0);
                            }
                        }
                    }
                }
                // === Audio for playback and waveform (from TTS) ===
                // Handles: audio, audio_student1, audio_student2, audio_tutor
                else if input_id == "audio" || input_id.starts_with("audio_") {
                    log::debug!("Attempting to extract audio from {} (data type: {:?}, len: {})",
                        input_id, data.data_type(), data.len());

                    if let Some(samples) = extract_f32_array(&data) {
                        log::info!("🔊 Audio received: {} samples from {}", samples.len(), input_id);

                        // Extract metadata
                        let question_id_str = get_metadata_string(&metadata, "question_id");
                        let session_status = get_metadata_string(&metadata, "session_status");
                        // Get participant from metadata or derive from input_id
                        let participant = get_metadata_string(&metadata, "participant")
                            .unwrap_or_else(|| extract_participant_from_audio_input(input_id).to_string());
                        let question_id = question_id_str.as_ref()
                            .and_then(|s| s.parse::<u32>().ok());

                        // Smart reset filtering: reject audio with wrong question_id
                        if filtering_mode {
                            if let Some(ref expected_qid) = reset_question_id {
                                if let Some(ref incoming_qid) = question_id_str {
                                    if incoming_qid != expected_qid {
                                        log::debug!("Filtering out audio with question_id={} (expected {})",
                                            incoming_qid, expected_qid);
                                        continue;
                                    } else {
                                        // First chunk with matching question_id - exit filtering mode
                                        log::info!("Received matching question_id={}, exiting filtering mode", expected_qid);
                                        filtering_mode = false;
                                    }
                                }
                            }
                        }

                        // Send session_start when first audio arrives for a new session
                        // with proper metadata passthrough
                        if !session_start_sent || session_status.as_deref() == Some("started") {
                            if let Some(ref qid) = question_id_str {
                                current_question_id = Some(qid.clone());
                            }

                            // Build metadata for session_start
                            let mut session_metadata: BTreeMap<String, Parameter> = BTreeMap::new();
                            if let Some(ref qid) = question_id_str {
                                session_metadata.insert("question_id".to_string(), Parameter::String(qid.clone()));
                            }
                            session_metadata.insert("participant".to_string(), Parameter::String(participant.clone()));
                            session_metadata.insert("source".to_string(), Parameter::String("dashboard".to_string()));
                            if let Some(ref status) = session_status {
                                session_metadata.insert("session_status".to_string(), Parameter::String(status.clone()));
                            }

                            // Send session_start output to conference-controller
                            if let Err(e) = node.send_output(
                                DataId::from("session_start".to_string()),
                                session_metadata,
                                vec!["audio_started".to_string()].into_arrow(),
                            ) {
                                log::warn!("Failed to send session_start: {}", e);
                            } else {
                                log::info!("Session started - first audio received for question {:?}", current_question_id);
                            }
                            session_start_sent = true;
                        }

                        // Write to audio player if available
                        if let Some(ref player) = audio_player {
                            // Get participant index for audio player tracking
                            let participant_idx = if input_id.contains("student1") || input_id.contains("llm1") {
                                Some(0usize)
                            } else if input_id.contains("student2") || input_id.contains("llm2") {
                                Some(1usize)
                            } else if input_id.contains("tutor") || input_id.contains("judge") {
                                Some(2usize)
                            } else {
                                None
                            };
                            player.write_audio(&samples, question_id, participant_idx);
                            log::debug!("Audio written: {} samples from {} (idx={:?}, buffer: {:.1}%)",
                                samples.len(), participant, participant_idx, player.buffer_fill_percentage());
                        } else {
                            log::warn!("Audio received but no audio player available");
                        }

                        // Send audio_complete signal (replaces TTS segment_complete for flow control)
                        {
                            let mut complete_metadata: BTreeMap<String, Parameter> = BTreeMap::new();
                            complete_metadata.insert("participant".to_string(), Parameter::String(participant.clone()));
                            if let Some(ref qid) = question_id_str {
                                complete_metadata.insert("question_id".to_string(), Parameter::String(qid.clone()));
                            }
                            if let Some(ref status) = session_status {
                                complete_metadata.insert("session_status".to_string(), Parameter::String(status.clone()));
                            }

                            // Send audio_complete to multi-text-segmenter for flow control
                            if let Err(e) = node.send_output(
                                DataId::from("audio_complete".to_string()),
                                complete_metadata.clone(),
                                vec!["received".to_string()].into_arrow(),
                            ) {
                                log::warn!("Failed to send audio_complete: {}", e);
                            } else {
                                log::debug!("Audio complete: {} (question_id={:?})", participant, question_id_str);
                            }

                            // Send session_end when session_status is "complete" (for conference-controller)
                            if session_status.as_deref() == Some("complete") {
                                if let Err(e) = node.send_output(
                                    DataId::from("session_end".to_string()),
                                    complete_metadata,
                                    vec!["session_ended".to_string()].into_arrow(),
                                ) {
                                    log::warn!("Failed to send session_end: {}", e);
                                } else {
                                    log::info!("📢 Session ended: {} (question_id={:?})", participant, question_id_str);
                                }
                            }
                        }

                        // Update shared state
                        let mut state = shared_state.lock();

                        // Update waveform data (keep last 512 samples for display)
                        let waveform_size = 512;
                        if samples.len() >= waveform_size {
                            // Take evenly spaced samples for visualization
                            let step = samples.len() / waveform_size;
                            state.waveform_data = (0..waveform_size)
                                .map(|i| samples[i * step])
                                .collect();
                        } else {
                            // Pad with zeros if not enough samples
                            state.waveform_data = samples.clone();
                            state.waveform_data.resize(waveform_size, 0.0);
                        }

                        // Update playback status from audio player
                        if let Some(ref player) = audio_player {
                            state.playback_status.is_playing = player.is_playing();
                            state.buffer_fill = player.buffer_fill_percentage();
                            state.buffer_seconds = player.buffer_seconds();
                            // Get active participant from audio player (tracks what's being played)
                            state.playback_status.active_participant_idx = player.current_participant_idx();
                        } else {
                            state.playback_status.is_playing = true;
                        }

                        // Set current speaker from the participant variable (for logging)
                        state.playback_status.current_speaker = participant.clone();

                        // Calculate audio level (RMS with peak normalization) for the current speaker
                        let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, |a, b| a.max(b));
                        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / samples.len() as f32).sqrt();
                        let norm_factor = if peak > 0.01 { 1.0 / peak } else { 1.0 };
                        let audio_level = (rms * norm_factor * 1.5).clamp(0.0, 1.0);

                        // Update audio level for the active participant, reset others
                        let participant_lower = participant.to_lowercase();
                        for (i, p) in state.participants.iter_mut().enumerate() {
                            let is_active = match i {
                                0 => participant_lower.contains("student1") || participant_lower.contains("llm1"),
                                1 => participant_lower.contains("student2") || participant_lower.contains("llm2"),
                                2 => participant_lower.contains("tutor") || participant_lower.contains("judge"),
                                _ => false,
                            };
                            if is_active {
                                p.audio_level = audio_level;
                            } else {
                                // Decay non-active participants' levels
                                p.audio_level = (p.audio_level * 0.9).max(0.0);
                            }
                        }
                    } else {
                        // Audio extraction failed - log the data type for debugging
                        log::warn!("❌ Failed to extract audio from {} (data type: {:?}, len: {})",
                            input_id, data.data_type(), data.len());
                    }
                }
                // === Control commands from controller ===
                // Handles: control_judge, control_llm1, control_llm2
                else if input_id.starts_with("control_") && !input_id.contains("status") {
                    if let Some(cmd) = extract_string(&data) {
                        if !cmd.is_empty() {
                            let target = if input_id.contains("judge") || input_id.contains("tutor") {
                                participant3_name.as_str()
                            } else if input_id.contains("llm1") || input_id.contains("student1") {
                                participant1_name.as_str()
                            } else if input_id.contains("llm2") || input_id.contains("student2") {
                                participant2_name.as_str()
                            } else {
                                "Unknown"
                            };

                            let msg = LogMessage {
                                timestamp: get_timestamp(),
                                level: "INFO".to_string(),
                                source: "Controller".to_string(),
                                message: format!("→ {}: {}", target, cmd),
                            };

                            // Print to console
                            if console_output {
                                println!("[{}] CONTROLLER → {}: {}", msg.timestamp, target, cmd);
                            }

                            let mut state = shared_state.lock();
                            state.log_messages.push(msg);
                            if state.log_messages.len() > 8000 {
                                state.log_messages.remove(0);
                            }
                        }
                    }
                }
                // === Logs from all nodes ===
                else if input_id.contains("log") {
                    if let Some(log_text) = extract_string(&data) {
                        // Try to parse as JSON log
                        let (level, source, message) = parse_log_message(&log_text, input_id);

                        // Filter by log level
                        let level_value = match level.to_uppercase().as_str() {
                            "DEBUG" => 10,
                            "INFO" => 20,
                            "WARNING" | "WARN" => 30,
                            "ERROR" => 40,
                            _ => 20,
                        };

                        if level_value < log_level_threshold {
                            continue; // Skip logs below threshold
                        }

                        // Skip verbose debug messages (like debate_viewer does)
                        if level == "DEBUG" {
                            let skip_phrases = [
                                "Metadata:",
                                "Received LLM chunk",
                                "Received text delta",
                                "Received other event",
                            ];
                            if skip_phrases.iter().any(|p| message.contains(p)) {
                                continue;
                            }
                        }

                        let msg = LogMessage {
                            timestamp: get_timestamp(),
                            level: level.clone(),
                            source: source.clone(),
                            message: message.clone(),
                        };

                        // Print to console (like debate_viewer)
                        if console_output {
                            let level_prefix = match level.as_str() {
                                "ERROR" => "\x1b[91m[ERROR]\x1b[0m",
                                "WARNING" | "WARN" => "\x1b[93m[WARN]\x1b[0m",
                                "INFO" => "\x1b[96m[INFO]\x1b[0m",
                                "DEBUG" => "\x1b[92m[DEBUG]\x1b[0m",
                                _ => "[LOG]",
                            };
                            println!("[{}] {} {}: {}", msg.timestamp, level_prefix, source, message);
                        }

                        let mut state = shared_state.lock();
                        state.log_messages.push(msg);
                        if state.log_messages.len() > 8000 {
                            state.log_messages.remove(0);
                        }
                    }
                }
                // === Status updates ===
                else if input_id.contains("status") && !input_id.contains("buffer") {
                    if let Some(status) = extract_string(&data) {
                        let idx = extract_participant_index(input_id);
                        let mut state = shared_state.lock();
                        if let Some(p) = state.participants.get_mut(idx) {
                            p.status = status.clone();
                            p.is_speaking = status == "generating" || status == "streaming";
                        }
                    }
                }
            }

            Event::Stop { .. } => {
                log::info!("Received stop event");
                break;
            }

            Event::Error(err) => {
                // Timeout errors are expected when using recv_timeout
                if err.contains("Timeout") {
                    // Check for shutdown during timeout
                    if SHUTDOWN.load(Ordering::SeqCst) {
                        break;
                    }
                    // Normal timeout, continue to next iteration
                } else {
                    log::warn!("Event error: {}", err);
                }
            }

            _ => {}
        }
    }

    Ok(())
}

/// Parse a log message, attempting JSON format first
fn parse_log_message(log_text: &str, input_id: &str) -> (String, String, String) {
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(log_text) {
        let level = json.get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("INFO")
            .to_string();
        let node_name = json.get("node")
            .and_then(|v| v.as_str())
            .unwrap_or(input_id);
        let source = get_display_source(node_name, input_id);
        let message = json.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or(log_text)
            .to_string();
        (level, source, message)
    } else {
        // Plain text log
        let source = get_display_source(input_id, input_id);
        ("INFO".to_string(), source, log_text.to_string())
    }
}

/// Get display-friendly source name from input_id or node name
fn get_display_source(node_name: &str, input_id: &str) -> String {
    let lower = input_id.to_lowercase();

    // Bridge logs
    if lower.contains("bridge") {
        if lower.contains("bridge1") || lower.contains("bridge-to-tutor") || lower.contains("bridge-to-judge") {
            return "Bridge→Tutor".to_string();
        } else if lower.contains("bridge2") || lower.contains("bridge-to-student2") || lower.contains("bridge-to-llm2") {
            return "Bridge→Student2".to_string();
        } else if lower.contains("bridge3") || lower.contains("bridge-to-student1") || lower.contains("bridge-to-llm1") {
            return "Bridge→Student1".to_string();
        }
        return "Bridge".to_string();
    }

    // Participant logs
    if lower.contains("student1") || lower.contains("llm1") {
        return "Student1".to_string();
    }
    if lower.contains("student2") || lower.contains("llm2") {
        return "Student2".to_string();
    }
    if lower.contains("tutor") || lower.contains("judge") {
        return "Tutor".to_string();
    }

    // Controller
    if lower.contains("controller") {
        return "Controller".to_string();
    }

    // Audio pipeline
    if lower.contains("segmenter") {
        return "Segmenter".to_string();
    }
    if lower.contains("tts") || lower.contains("primespeech") {
        if lower.contains("1") {
            return "TTS-1".to_string();
        } else if lower.contains("2") {
            return "TTS-2".to_string();
        } else if lower.contains("3") || lower.contains("tutor") {
            return "TTS-3".to_string();
        }
        return "TTS".to_string();
    }
    if lower.contains("audio") {
        return "AudioPlayer".to_string();
    }
    if lower.contains("asr") {
        return "ASR".to_string();
    }

    // Default: clean up the name
    node_name.replace("_log", "").replace("/log", "").replace("-", " ")
}

fn extract_f64(data: &ArrowData) -> Option<f64> {
    let array = &data.0;
    if array.is_empty() {
        return None;
    }

    match array.data_type() {
        DataType::Float64 => {
            array.as_any().downcast_ref::<Float64Array>()
                .and_then(|arr| if arr.is_null(0) { None } else { Some(arr.value(0)) })
        }
        DataType::Float32 => {
            array.as_any().downcast_ref::<Float32Array>()
                .and_then(|arr| if arr.is_null(0) { None } else { Some(arr.value(0) as f64) })
        }
        DataType::Int64 => {
            array.as_any().downcast_ref::<Int64Array>()
                .and_then(|arr| if arr.is_null(0) { None } else { Some(arr.value(0) as f64) })
        }
        DataType::Int32 => {
            array.as_any().downcast_ref::<Int32Array>()
                .and_then(|arr| if arr.is_null(0) { None } else { Some(arr.value(0) as f64) })
        }
        _ => None,
    }
}

fn extract_string(data: &ArrowData) -> Option<String> {
    let array = &data.0;
    if array.is_empty() {
        return None;
    }

    match array.data_type() {
        DataType::Utf8 => {
            array.as_any().downcast_ref::<StringArray>()
                .and_then(|arr| if arr.is_null(0) { None } else { Some(arr.value(0).to_string()) })
        }
        DataType::LargeUtf8 => {
            array.as_any().downcast_ref::<LargeStringArray>()
                .and_then(|arr| if arr.is_null(0) { None } else { Some(arr.value(0).to_string()) })
        }
        DataType::UInt8 => {
            // Try to interpret as UTF-8 bytes
            array.as_any().downcast_ref::<UInt8Array>()
                .and_then(|arr| {
                    let bytes: Vec<u8> = arr.values().to_vec();
                    String::from_utf8(bytes).ok()
                })
        }
        _ => None,
    }
}

/// Extract f32 array for waveform data
fn extract_f32_array(data: &ArrowData) -> Option<Vec<f32>> {
    use dora_node_api::arrow::array::{Int16Array, ListArray, LargeListArray};

    let array = &data.0;
    if array.is_empty() {
        return None;
    }

    match array.data_type() {
        DataType::Float32 => {
            array.as_any().downcast_ref::<Float32Array>()
                .map(|arr| arr.values().to_vec())
        }
        DataType::Float64 => {
            array.as_any().downcast_ref::<Float64Array>()
                .map(|arr| arr.values().iter().map(|&v| v as f32).collect())
        }
        DataType::Int16 => {
            // Convert int16 PCM to float32 (-1.0 to 1.0)
            array.as_any().downcast_ref::<Int16Array>()
                .map(|arr| arr.values().iter().map(|&v| v as f32 / 32768.0).collect())
        }
        DataType::UInt8 => {
            // Raw bytes - interpret as float32
            array.as_any().downcast_ref::<UInt8Array>()
                .and_then(|arr| {
                    let bytes = arr.values();
                    if bytes.len() % 4 == 0 {
                        Some(bytes.chunks(4)
                            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                            .collect())
                    } else {
                        None
                    }
                })
        }
        // Handle ListArray<Float32> - primespeech sends pa.array([audio_array])
        DataType::List(field) | DataType::LargeList(field) => {
            log::debug!("Audio data is ListArray with field type: {:?}", field.data_type());

            // First try ListArray
            if let Some(list_arr) = array.as_any().downcast_ref::<ListArray>() {
                if list_arr.len() > 0 {
                    let first_value = list_arr.value(0);
                    // Try to extract float32 from the inner array
                    if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>() {
                        log::debug!("Extracted {} f32 samples from ListArray", float_arr.len());
                        return Some(float_arr.values().to_vec());
                    }
                    if let Some(float_arr) = first_value.as_any().downcast_ref::<Float64Array>() {
                        log::debug!("Extracted {} f64 samples from ListArray, converting to f32", float_arr.len());
                        return Some(float_arr.values().iter().map(|&v| v as f32).collect());
                    }
                    log::warn!("ListArray inner type not Float32/Float64: {:?}", first_value.data_type());
                }
            }

            // Then try LargeListArray
            if let Some(list_arr) = array.as_any().downcast_ref::<LargeListArray>() {
                if list_arr.len() > 0 {
                    let first_value = list_arr.value(0);
                    // Try to extract float32 from the inner array
                    if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>() {
                        log::debug!("Extracted {} f32 samples from LargeListArray", float_arr.len());
                        return Some(float_arr.values().to_vec());
                    }
                    if let Some(float_arr) = first_value.as_any().downcast_ref::<Float64Array>() {
                        log::debug!("Extracted {} f64 samples from LargeListArray, converting to f32", float_arr.len());
                        return Some(float_arr.values().iter().map(|&v| v as f32).collect());
                    }
                    log::warn!("LargeListArray inner type not Float32/Float64: {:?}", first_value.data_type());
                }
            }

            log::warn!("Failed to extract audio from ListArray");
            None
        }
        // Handle FixedSizeList (another possible list type)
        DataType::FixedSizeList(field, _size) => {
            use dora_node_api::arrow::array::FixedSizeListArray;
            log::debug!("Audio data is FixedSizeListArray with field type: {:?}", field.data_type());

            if let Some(list_arr) = array.as_any().downcast_ref::<FixedSizeListArray>() {
                if list_arr.len() > 0 {
                    let first_value = list_arr.value(0);
                    if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>() {
                        log::debug!("Extracted {} f32 samples from FixedSizeListArray", float_arr.len());
                        return Some(float_arr.values().to_vec());
                    }
                }
            }

            log::warn!("Failed to extract audio from FixedSizeListArray");
            None
        }
        dt => {
            log::warn!("Unsupported audio data type: {:?}", dt);
            None
        }
    }
}

fn get_metadata_string(metadata: &Metadata, key: &str) -> Option<String> {
    use dora_node_api::Parameter;
    metadata.parameters.get(key).map(|v| match v {
        Parameter::String(s) => s.clone(),
        Parameter::Integer(i) => i.to_string(),
        Parameter::Float(f) => f.to_string(),
        Parameter::Bool(b) => b.to_string(),
        Parameter::ListInt(l) => format!("{:?}", l),
        Parameter::ListFloat(l) => format!("{:?}", l),
        Parameter::ListString(l) => format!("{:?}", l),
    })
}

/// Extract participant index from input_id
/// Handles both naming conventions:
/// - Conference style: student1, student2, tutor
/// - Debate style: llm1, llm2, judge
/// - TTS style: tts1, tts2, tts3
fn extract_participant_index(id: &str) -> usize {
    let lower = id.to_lowercase();
    if lower.contains("student1") || lower.contains("llm1") || lower.contains("tts1") {
        0
    } else if lower.contains("student2") || lower.contains("llm2") || lower.contains("tts2") {
        1
    } else if lower.contains("tutor") || lower.contains("judge") || lower.contains("tts3") {
        2
    } else {
        0
    }
}

/// Extract participant name from input_id for audio metadata
fn extract_participant_from_audio_input(id: &str) -> &'static str {
    let lower = id.to_lowercase();
    if lower.contains("student1") || lower.contains("llm1") {
        "student1"
    } else if lower.contains("student2") || lower.contains("llm2") {
        "student2"
    } else if lower.contains("tutor") || lower.contains("judge") {
        "tutor"
    } else {
        "unknown"
    }
}

fn get_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() % 86400;
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// Truncate text to a maximum length, adding "..." if truncated
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Run in demo mode with simulated data
fn run_demo_mode(shared_state: SharedStateRef, _audio_player: Option<AudioPlayerRef>) {
    log::info!("Running in demo mode with simulated data");

    let mut time = 0.0f64;
    let mut segment_counts = [0u32; 3];

    while !SHUTDOWN.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
        time += 0.1;

        let mut state = shared_state.lock();

        // Process control commands from UI (log them in demo mode)
        let commands: Vec<ControlCommand> = std::mem::take(&mut state.control_commands);
        for cmd in commands {
            match &cmd {
                ControlCommand::Reset => {
                    // Clear logs and chat messages on reset
                    state.log_messages.clear();
                    state.chat_messages.clear();
                    for p in state.participants.iter_mut() {
                        p.conversation_history.clear();
                        p.last_text.clear();
                        p.segment_count = 0;
                    }
                    log::info!("[Demo] Reset command received");
                }
                _ => {
                    let msg = match &cmd {
                        ControlCommand::StartQuestion(qid) => format!("Start question {} received", qid),
                        ControlCommand::Pause => "Pause command received".to_string(),
                        ControlCommand::Resume => "Resume command received".to_string(),
                        ControlCommand::NextRound => "Next round command received".to_string(),
                        ControlCommand::SendPrompt(prompt) => format!("Send prompt received: {}", prompt),
                        ControlCommand::Reset => unreachable!(),
                    };
                    log::info!("[Demo] {}", msg);
                    state.log_messages.push(LogMessage {
                        timestamp: get_timestamp(),
                        level: "INFO".to_string(),
                        source: "UI".to_string(),
                        message: msg,
                    });
                    if state.log_messages.len() > 8000 {
                        state.log_messages.remove(0);
                    }
                }
            }
        }

        // Simulate buffer fill
        state.buffer_fill = 30.0 + 20.0 * (time * 0.5).sin();
        state.buffer_seconds = (state.buffer_fill / 100.0) * 360.0;

        // Simulate participant activity
        let active = ((time / 5.0) as usize) % 3;
        for (i, p) in state.participants.iter_mut().enumerate() {
            if i == active {
                p.is_speaking = true;
                p.status = "Speaking".to_string();
                segment_counts[i] += 1;
                p.segment_count = segment_counts[i];
                p.last_text = format!("Simulated text from {}...", p.name);
            } else {
                p.is_speaking = false;
                p.status = "Idle".to_string();
            }
        }

        // Simulate playback
        state.playback_status.is_playing = true;
        state.playback_status.current_speaker = state.participants[active].name.clone();

        // Simulate waveform
        state.waveform_data = (0..512)
            .map(|i| ((i as f64 * 0.1 + time * 10.0).sin() * 0.5) as f32)
            .collect();

        // Periodic log messages
        if (time * 10.0) as u32 % 50 == 0 {
            state.log_messages.push(LogMessage {
                timestamp: get_timestamp(),
                level: "INFO".to_string(),
                source: "demo".to_string(),
                message: format!("Demo message at t={:.1}s", time),
            });
            if state.log_messages.len() > 8000 {
                state.log_messages.remove(0);
            }
        }
    }
    log::info!("Demo mode shutting down");
}
