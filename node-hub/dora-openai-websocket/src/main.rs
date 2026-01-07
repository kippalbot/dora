// Copyright 2023 Divy Srivastava <dj.srivastava23@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use base64::engine::general_purpose;
use base64::Engine;
use dora_node_api::arrow::array::{Array, AsArray};
use dora_node_api::arrow::datatypes::DataType;
use dora_node_api::dora_core::config::DataId;
use dora_node_api::dora_core::config::NodeId;
use dora_node_api::into_vec;
use dora_node_api::DoraNode;
use dora_node_api::EventStream;
use dora_node_api::IntoArrow;
use dora_node_api::MetadataParameters;
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};
use fastwebsockets::upgrade;
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::Payload;
use tokio::process::Command;
use fastwebsockets::WebSocketError;
use futures_concurrency::future::Race;
use futures_util::future;
use futures_util::future::Either;
use futures_util::FutureExt;
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use serde;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};

// Global state for sharing the Dora node connection across WebSocket clients
// Using RwLock<Option<...>> allows resetting the connection on reconnect
static DORA_NODE: OnceCell<RwLock<Option<Arc<Mutex<DoraNode>>>>> = OnceCell::new();
static DORA_EVENTS: OnceCell<RwLock<Option<Arc<Mutex<EventStream>>>>> = OnceCell::new();
static MAAS_PID: OnceCell<Arc<Mutex<Option<u32>>>> = OnceCell::new();
static DORA_NODE_NAME: OnceCell<String> = OnceCell::new();
static CONNECTION_ACTIVE: AtomicBool = AtomicBool::new(false);

// Log level threshold (0=DEBUG, 1=INFO, 2=WARNING, 3=ERROR)
static LOG_LEVEL_THRESHOLD: OnceCell<u8> = OnceCell::new();

fn get_log_level_value(level: &str) -> u8 {
    match level.to_uppercase().as_str() {
        "DEBUG" => 0,
        "INFO" => 1,
        "WARNING" | "WARN" => 2,
        "ERROR" => 3,
        _ => 1, // Default to INFO
    }
}

fn should_log(level: &str) -> bool {
    let threshold = LOG_LEVEL_THRESHOLD.get().copied().unwrap_or(1); // Default to INFO
    get_log_level_value(level) >= threshold
}

// Helper function to send log messages through dora (sync version for use within event loop)
fn send_log(node: &mut DoraNode, level: &str, message: &str) {
    // Filter by log level threshold
    if !should_log(level) {
        return;
    }

    // Print to console
    println!("[websocket-server] [{}] {}", level, message);

    let log_data = serde_json::json!({
        "node": "websocket-server",
        "level": level,
        "message": message
    });

    let _ = node.send_output(
        DataId::from("log".to_string()),
        Default::default(),
        serde_json::to_string(&log_data).unwrap().into_arrow(),
    );
}

// Async version using global node (for use before node is locked in handle_client)
async fn send_log_async(level: &str, message: &str) {
    // Filter by log level threshold
    if !should_log(level) {
        return;
    }

    // Print to console
    println!("[websocket-server] [{}] {}", level, message);

    if let Some(node_lock) = DORA_NODE.get() {
        let read_guard = node_lock.read().await;
        if let Some(node_arc) = &*read_guard {
            let mut node = node_arc.lock().await;
            let log_data = serde_json::json!({
                "node": "websocket-server",
                "level": level,
                "message": message
            });

            let _ = node.send_output(
                DataId::from("log".to_string()),
                Default::default(),
                serde_json::to_string(&log_data).unwrap().into_arrow(),
            );
        }
    }
}

// Send reset signal to downstream nodes (ASR, text-segmenter, TTS)
async fn send_reset_async() {
    send_log_async("INFO", "Sending reset signal to downstream nodes...").await;

    if let Some(node_lock) = DORA_NODE.get() {
        let read_guard = node_lock.read().await;
        if let Some(node_arc) = &*read_guard {
            let mut node = node_arc.lock().await;

            // Send "reset" command to the reset output
            // This will be connected to text-segmenter/reset, primespeech/control, and asr/control
            let result = node.send_output(
                DataId::from("reset".to_string()),
                Default::default(),
                "reset".to_string().into_arrow(),
            );

            match result {
                Ok(_) => {
                    println!("[websocket-server] [INFO] Reset signal sent successfully");
                }
                Err(e) => {
                    println!("[websocket-server] [WARNING] Failed to send reset signal: {:?}", e);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorDetails {
    pub code: Option<String>,
    pub message: String,
    pub param: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum OpenAIRealtimeMessage {
    #[serde(rename = "session.update")]
    SessionUpdate { session: SessionConfig },
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend {
        audio: String, // base64 encoded audio
    },
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit,
    #[serde(rename = "response.create")]
    ResponseCreate { response: ResponseConfig },
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate { item: ConversationItem },
    #[serde(rename = "conversation.item.truncate")]
    ConversationItemTruncate {
        item_id: String,
        content_index: u32,
        audio_end_ms: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
    },
    // Gracefully ignore unknown/unsupported client events
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionConfig {
    pub modalities: Vec<String>,
    pub instructions: String,
    pub voice: String,
    pub model: String,
    pub input_audio_format: String,
    pub output_audio_format: String,
    pub input_audio_transcription: Option<TranscriptionConfig>,
    pub turn_detection: Option<TurnDetectionConfig>,
    pub tools: Vec<serde_json::Value>,
    pub tool_choice: String,
    pub temperature: f32,
    pub max_response_output_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptionConfig {
    pub model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TurnDetectionConfig {
    #[serde(rename = "type")]
    pub detection_type: String,
    pub threshold: f32,
    pub prefix_padding_ms: u32,
    pub silence_duration_ms: u32,
    pub interrupt_response: bool,
    pub create_response: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseConfig {
    pub modalities: Vec<String>,
    pub instructions: Option<String>,
    pub voice: Option<String>,
    pub output_audio_format: Option<String>,
    pub tools: Option<Vec<serde_json::Value>>,
    pub tool_choice: Option<String>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConversationItem {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    pub status: Option<String>,
    pub role: String,
    pub content: Vec<ContentPart>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_audio")]
    InputAudio {
        audio: String,
        transcript: Option<String>,
    },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "audio")]
    Audio {
        audio: String,
        transcript: Option<String>,
    },
}

// Incoming message types from OpenAI
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum OpenAIRealtimeResponse {
    #[serde(rename = "error")]
    Error { error: ErrorDetails },
    #[serde(rename = "response.created")]
    ResponseCreated { response: serde_json::Value },
    #[serde(rename = "session.created")]
    SessionCreated { session: serde_json::Value },
    #[serde(rename = "session.updated")]
    SessionUpdated { session: serde_json::Value },
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated { item: serde_json::Value },
    #[serde(rename = "conversation.item.truncated")]
    ConversationItemTruncated { item: serde_json::Value },
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta {
        response_id: String,
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String, // base64 encoded audio
    },
    #[serde(rename = "response.audio.done")]
    ResponseAudioDone {
        response_id: String,
        item_id: String,
        output_index: u32,
        content_index: u32,
    },
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta {
        response_id: String,
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String,
    },
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta {
        response_id: String,
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String,
    },
    #[serde(rename = "response.done")]
    ResponseDone { response: serde_json::Value },
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted {
        audio_start_ms: u32,
        item_id: String,
    },
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped { audio_end_ms: u32, item_id: String },
    #[serde(other)]
    Other,
}

fn convert_pcm16_to_f32(bytes: &[u8]) -> Vec<f32> {
    let mut samples = Vec::with_capacity(bytes.len() / 2);

    for chunk in bytes.chunks_exact(2) {
        let pcm16_sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        let f32_sample = pcm16_sample as f32 / 32767.0;
        samples.push(f32_sample);
    }

    samples
}

fn convert_f32_to_pcm16(samples: &[f32]) -> Vec<u8> {
    let mut pcm16_bytes = Vec::with_capacity(samples.len() * 2);

    for &sample in samples {
        // Clamp to [-1.0, 1.0] and convert to i16
        let clamped = sample.max(-1.0).min(1.0);
        let pcm16_sample = (clamped * 32767.0) as i16;
        pcm16_bytes.extend_from_slice(&pcm16_sample.to_le_bytes());
    }

    pcm16_bytes
}


async fn handle_client(fut: upgrade::UpgradeFut) -> Result<(), WebSocketError> {
    send_log_async("INFO", "WebSocket client connected, waiting for upgrade completion").await;
    let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);
    send_log_async("INFO", "WebSocket connection established, waiting for first message").await;

    let frame = ws.read_frame().await?;
    send_log_async("DEBUG", &format!("Received first frame, opcode: {:?}, payload size: {}", frame.opcode, frame.payload.len())).await;

    if frame.opcode != OpCode::Text {
        send_log_async("ERROR", &format!("Expected text frame, got {:?}", frame.opcode)).await;
        // Send proper close for protocol error
        ws.write_frame(Frame::close(1002, b"Protocol error")).await?;
        return Err(WebSocketError::InvalidConnectionHeader);
    }
    
    send_log_async("DEBUG", "Parsing message as OpenAIRealtimeMessage").await;
    let data: OpenAIRealtimeMessage = match serde_json::from_slice(&frame.payload) {
        Ok(msg) => {
            send_log_async("DEBUG", "Successfully parsed message").await;
            msg
        },
        Err(e) => {
            send_log_async("ERROR", &format!("Failed to parse message: {}", e)).await;
            send_log_async("DEBUG", &format!("Raw payload: {}", String::from_utf8_lossy(&frame.payload))).await;
            // Unsupported/invalid initial data
            ws.write_frame(Frame::close(1003, b"Unsupported data")).await?;
            return Err(WebSocketError::InvalidConnectionHeader);
        }
    };
    
    let OpenAIRealtimeMessage::SessionUpdate { session } = data else {
        send_log_async("ERROR", "Expected SessionUpdate, got different message type").await;
        ws.write_frame(Frame::close(1003, b"Unsupported data")).await?;
        return Err(WebSocketError::InvalidConnectionHeader);
    };
    send_log_async("INFO", "Received SessionUpdate from client").await;

    let input_audio_transcription = session
        .input_audio_transcription
        .as_ref()
        .map_or("whisper".to_string(), |t| {
            // Can't await in closure, log after
            t.model.clone()
        });
    if let Some(t) = &session.input_audio_transcription {
        send_log_async("INFO", &format!("Client requested transcription model: {}", t.model)).await;
    }
    let llm = session.model.clone();
    send_log_async("INFO", &format!("Session config - Transcription: {}, LLM: {}", input_audio_transcription, llm)).await;

    // Accept any model name from moly, but log what we're actually using
    send_log_async("INFO", &format!("Client requested model: {}", llm)).await;
    if llm.contains("Qwen") || llm.contains("GGUF") {
        send_log_async("DEBUG", "Note: Client requested a Qwen/GGUF model, will use whatever is configured in the template").await;
    }
    
    // Prepare session response but DON'T send yet - wait until maas-client is ready
    let session_response = serde_json::json!({
        "id": format!("session_wserver"),
        "object": "realtime.session",
        "model": session.model.clone(),
        "modalities": session.modalities.clone(),
        "instructions": session.instructions.clone(),
        "voice": session.voice.clone(),
        "input_audio_format": session.input_audio_format.clone(),
        "output_audio_format": session.output_audio_format.clone(),
        "input_audio_transcription": session.input_audio_transcription.clone(),
        "turn_detection": session.turn_detection.clone(),
        "tools": session.tools.clone(),
        "tool_choice": session.tool_choice.clone(),
        "temperature": session.temperature,
        "max_response_output_tokens": session.max_response_output_tokens,
    });
    
    send_log_async("DEBUG", "Session response prepared, but will wait to send until maas-client is ready...").await;

    // NOTE: Dynamic nodes are now connected at server startup, not per-client
    // The wserver and maas-client nodes are initialized in main() when --name is provided
    // On reconnect, we reinitialize the dora connection if needed

    // Check if another client is already connected
    // Use compare_exchange to atomically check and set
    let was_active = CONNECTION_ACTIVE.swap(true, Ordering::SeqCst);
    if was_active {
        send_log_async("WARNING", "Another client session was active, taking over...").await;
        // Don't reject - just log and continue. The previous session may have crashed.
        // The locks will serialize access anyway.
    }

    // Create a guard to ensure CONNECTION_ACTIVE is reset on any exit
    struct ConnectionGuard;
    impl Drop for ConnectionGuard {
        fn drop(&mut self) {
            CONNECTION_ACTIVE.store(false, Ordering::SeqCst);
            println!("[websocket-server] [INFO] Connection guard dropped, ready for new client");
        }
    }
    let _connection_guard = ConnectionGuard;

    send_log_async("INFO", "Getting Dora node connection for client session...").await;

    // Get the RwLock containers
    let node_lock = match DORA_NODE.get() {
        Some(n) => n,
        None => {
            CONNECTION_ACTIVE.store(false, Ordering::SeqCst);
            send_log_async("ERROR", "Dora node not initialized. Make sure to run with --name argument").await;
            let _ = ws.write_frame(Frame::text(Payload::Borrowed(r#"{
                "type": "error",
                "error": {
                    "message": "Server not connected to dataflow. Please restart the server with --name argument.",
                    "type": "server_error",
                    "code": "dataflow_not_connected"
                }
            }"#.as_bytes()))).await;
            return Ok(());
        }
    };

    let events_lock = match DORA_EVENTS.get() {
        Some(e) => e,
        None => {
            CONNECTION_ACTIVE.store(false, Ordering::SeqCst);
            send_log_async("ERROR", "Dora events not initialized").await;
            return Ok(());
        }
    };

    // Get the existing dora connection (kept alive across client sessions)
    let node_arc = {
        let read_guard = node_lock.read().await;
        match &*read_guard {
            Some(arc) => arc.clone(),
            None => {
                CONNECTION_ACTIVE.store(false, Ordering::SeqCst);
                send_log_async("ERROR", "Dora node not available - server may need restart").await;
                let _ = ws.write_frame(Frame::text(Payload::Borrowed(r#"{
                    "type": "error",
                    "error": {
                        "message": "Dora connection lost. Please restart the server.",
                        "type": "server_error",
                        "code": "connection_lost"
                    }
                }"#.as_bytes()))).await;
                return Ok(());
            }
        }
    };

    let events_arc = {
        let read_guard = events_lock.read().await;
        match &*read_guard {
            Some(arc) => arc.clone(),
            None => {
                CONNECTION_ACTIVE.store(false, Ordering::SeqCst);
                send_log_async("ERROR", "Dora events not available").await;
                return Ok(());
            }
        }
    };

    send_log_async("INFO", "Reusing existing Dora connection for new client session").await;
    
    // Optionally spawn a dynamic maas-client. When using a static maas-client in the dataflow,
    // set SPAWN_MAAS=1 (or "true") to spawn dynamically. Default is false (use static node).
    let spawn_maas = std::env::var("SPAWN_MAAS")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);

    if spawn_maas {
        // Kill existing maas-client if any and spawn a new one with updated config
        if let Some(pid_arc) = MAAS_PID.get() {
        let mut pid_guard = pid_arc.lock().await;
        if let Some(pid) = *pid_guard {
            send_log_async("INFO", &format!("Killing existing maas-client with PID: {}", pid)).await;
            // Try to kill the process
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .output();
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Spawn new maas-client with potentially updated config
        send_log_async("INFO", "Spawning new maas-client for this session...").await;
        // Use relative path or get from environment variable
        let config_path = std::env::var("MAAS_CONFIG_PATH")
            .unwrap_or_else(|_| "maas_mcp_browser_config.toml".to_string());
        
        match tokio::process::Command::new("dora-maas-client")
            .arg("--name")
            .arg("maas-client")
            .env("MAAS_CONFIG_PATH", &config_path)
            .spawn() {
            Ok(mut child) => {
                if let Some(pid) = child.id() {
                    send_log_async("INFO", &format!("New maas-client spawned with PID: {}", pid)).await;
                    send_log_async("INFO", &format!("Using config: {}", config_path)).await;
                    *pid_guard = Some(pid);

                    // Monitor the process in the background
                    tokio::spawn(async move {
                        match child.wait().await {
                            Ok(status) => {
                                if !status.success() {
                                    send_log_async("WARNING", &format!("maas-client exited with status: {:?}", status)).await;
                                }
                            }
                            Err(e) => {
                                send_log_async("WARNING", &format!("Error waiting for maas-client: {}", e)).await;
                            }
                        }
                    });
                }
            }
            Err(e) => {
                send_log_async("WARNING", &format!("Failed to spawn maas-client from PATH: {}", e)).await;
                send_log_async("INFO", "Falling back to 'cargo run -p dora-maas-client' (dev flow)").await;
                match tokio::process::Command::new("cargo")
                    .arg("run")
                    .arg("-p")
                    .arg("dora-maas-client")
                    .arg("--")
                    .arg("--name")
                    .arg("maas-client")
                    .env("MAAS_CONFIG_PATH", &config_path)
                    .spawn() {
                    Ok(mut child) => {
                        if let Some(pid) = child.id() {
                            send_log_async("INFO", &format!("Fallback cargo run: maas-client PID: {}", pid)).await;
                            send_log_async("INFO", &format!("Using config: {}", config_path)).await;
                            *pid_guard = Some(pid);
                            tokio::spawn(async move {
                                match child.wait().await {
                                    Ok(status) => {
                                        if !status.success() {
                                            send_log_async("WARNING", &format!("maas-client (cargo run) exited with status: {:?}", status)).await;
                                        }
                                    }
                                    Err(e) => {
                                        send_log_async("WARNING", &format!("Error waiting for maas-client (cargo run): {}", e)).await;
                                    }
                                }
                            });
                        }
                    }
                    Err(e2) => {
                        send_log_async("ERROR", &format!("Failed to spawn maas-client via cargo run as well: {}", e2)).await;
                    }
                }
            }
        }

            // Wait for maas-client to be ready before sending session acknowledgments
            send_log_async("INFO", "Waiting 10 seconds for maas-client to connect to dataflow...").await;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            send_log_async("INFO", "maas-client should now be ready").await;
        }
    } else {
        send_log_async("INFO", "SPAWN_MAAS disabled; using existing static maas-client in dataflow").await;
    }

    // NOW send session acknowledgments after maas-client is ready
    send_log_async("INFO", "Sending session acknowledgments to client now that maas-client is ready...").await;
    
    let serialized_data = OpenAIRealtimeResponse::SessionCreated {
        session: session_response.clone(),
    };
    let payload =
        Payload::Bytes(Bytes::from(serde_json::to_string(&serialized_data).unwrap()).into());
    let frame = Frame::text(payload);
    send_log_async("INFO", "Sending session.created acknowledgment to client").await;
    ws.write_frame(frame).await?;

    // Also send session.updated to confirm the session update was processed
    let serialized_updated = OpenAIRealtimeResponse::SessionUpdated {
        session: session_response,
    };
    let payload_updated =
        Payload::Bytes(Bytes::from(serde_json::to_string(&serialized_updated).unwrap()).into());
    let frame_updated = Frame::text(payload_updated);
    send_log_async("INFO", "Sending session.updated acknowledgment to client").await;
    ws.write_frame(frame_updated).await?;
    
    let mut node = node_arc.lock().await;
    let mut events = events_arc.lock().await;
    
    // Create resampler with fixed buffer size for microphone input
    const DOWNSAMPLE_CHUNK_SIZE: usize = 4800; // 200ms at 24kHz
    
    let resampler_params = SincInterpolationParameters {
        sinc_len: 256,          // High quality
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    
    // Downsampler for microphone input (24kHz -> 16kHz)
    let mut downsampler = SincFixedIn::<f32>::new(
        16000.0 / 24000.0,  // Resample ratio (2/3)
        2.0,                // Max delay
        resampler_params,
        DOWNSAMPLE_CHUNK_SIZE,  // Fixed input size
        1,                  // 1 channel (mono)
    ).expect("Failed to create downsampler");
    
    let mut audio_buffer = Vec::new(); // Buffer for microphone audio
    // Segment counting is now handled via metadata from primespeech

    send_log(&mut node, "INFO", "Starting main event loop with fixed-size resampler");
    send_log(&mut node, "INFO", "Pipeline: WebSocket -> ASR -> MaaS -> Text-Segmenter -> TTS -> Audio -> WebSocket");
    send_log(&mut node, "INFO", "Listening for audio input and text output from dataflow nodes...");

    // Wait for client to send greeting via ResponseCreate message
    send_log(&mut node, "INFO", "Waiting for client to send greeting via response.create message...");
    
    let mut audio_chunks_received = 0;
    let mut text_chunks_sent = 0;
    let mut last_activity = std::time::Instant::now();
    let mut should_send_completion = false; // Track if we need to send completion events after audio
    let mut response_created_sent = false; // Ensure response.created is sent once per turn
    let mut response_active = false; // Only emit deltas while a response is active
    // Track if we've replied to a Close frame
    let mut close_replied = false;
    // Deduplicate greeting instructions to avoid repeated forwards
    let mut last_greeting: Option<String> = None;
    let mut last_greet_time: Option<std::time::Instant> = None;
    loop {
        let event_fut = events.recv_async().map(Either::Left);
        let frame_fut = ws.read_frame().map(Either::Right);
        let event_stream = (event_fut, frame_fut).race();
        let frame = match event_stream.await {
            future::Either::Left(Some(ev)) => {
                let frame = match ev {
                    dora_node_api::Event::Input {
                        id,
                        metadata,
                        data,
                    } => {
                        let now = std::time::Instant::now();
                        let time_since_last = now.duration_since(last_activity).as_millis();
                        last_activity = now;
                        
                        if data.data_type() == &DataType::Utf8 {
                            let data = data.as_string::<i32>();
                            let str = data.value(0);
                            text_chunks_sent += 1;
                            
                            // Determine the source node and log appropriately with full pipeline context
                            if id.contains("segment_complete") {
                                // PrimeSpeech completion or error
                                if str == "error" {
                                    // Try to surface error details from metadata
                                    let mut err = String::from("unknown");
                                    let mut stage = String::from("unknown");
                                    let mut segment_idx = String::new();
                                    if let Some(param) = metadata.parameters.get("error") {
                                        if let dora_node_api::Parameter::String(s) = param { err = s.clone(); }
                                    }
                                    if let Some(param) = metadata.parameters.get("error_stage") {
                                        if let dora_node_api::Parameter::String(s) = param { stage = s.clone(); }
                                    }
                                    if let Some(param) = metadata.parameters.get("segment_index") {
                                        if let dora_node_api::Parameter::Integer(i) = param { segment_idx = format!(", segment: {}", i); }
                                    }
                                    send_log(&mut node, "ERROR", &format!("[{}ms] TTS ERROR: {} -> WebSocket, stage: {}, error: {}{}", time_since_last, id, stage, err, segment_idx));
                                } else {
                                    let mut session_status = "unknown".to_string();
                                    if let Some(param) = metadata.parameters.get("session_status") {
                                        if let dora_node_api::Parameter::String(s) = param { session_status = s.clone(); }
                                    }
                                    send_log(&mut node, "DEBUG", &format!("[{}ms] TTS SEGMENT COMPLETE: {} -> WebSocket, status: '{}', session_status: {}", time_since_last, id, str, session_status));
                                }
                            } else if id.contains("log") {
                                // Log channel (e.g., from PrimeSpeech) - skip forwarding to client
                                send_log(&mut node, "DEBUG", &format!("[{}ms] NODE LOG from {}: {}", time_since_last, id, str));
                                continue;
                            } else if id.contains("transcription") {
                                send_log(&mut node, "INFO", &format!("[{}ms] ASR OUTPUT: {} -> WebSocket, text: '{}', chunk #{}, {} chars", time_since_last, id, str.chars().take(100).collect::<String>(), text_chunks_sent, str.len()));
                            } else if id.contains("text") && (id.contains("maas") || id.contains("qwen")) {
                                send_log(&mut node, "INFO", &format!("[{}ms] LLM OUTPUT: {} -> WebSocket, text: '{}', chunk #{}, {} chars", time_since_last, id, str.chars().take(100).collect::<String>(), text_chunks_sent, str.len()));
                            } else {
                                send_log(&mut node, "INFO", &format!("[{}ms] TEXT OUTPUT: {} -> WebSocket, text: '{}', chunk #{}, {} chars", time_since_last, id, str.chars().take(100).collect::<String>(), text_chunks_sent, str.len()));
                            }

                            // Ensure a response is active before sending any deltas
                            if !response_active {
                                let created = OpenAIRealtimeResponse::ResponseCreated {
                                    response: serde_json::json!({
                                        "id": "123",
                                        "status": "in_progress",
                                        "output": []
                                    }),
                                };
                                let created_frame = Frame::text(Payload::Bytes(
                                    Bytes::from(serde_json::to_string(&created).unwrap()).into(),
                                ));
                                ws.write_frame(created_frame).await?;
                                send_log(&mut node, "DEBUG", "Sent response.created (id=123) before transcript/text delta");
                                response_created_sent = true;
                                response_active = true;
                            }

                            // Route: ASR transcription -> transcript delta; LLM text -> text delta
                            let serialized_data = if id.contains("transcription") {
                                OpenAIRealtimeResponse::ResponseAudioTranscriptDelta {
                                    response_id: "123".to_string(),
                                    item_id: "123".to_string(),
                                    output_index: 123,
                                    content_index: 123,
                                    delta: str.to_string(),
                                }
                            } else {
                                OpenAIRealtimeResponse::ResponseTextDelta {
                                    response_id: "123".to_string(),
                                    item_id: "123".to_string(),
                                    output_index: 123,
                                    content_index: 123,
                                    delta: str.to_string(),
                                }
                            };

                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&serialized_data).unwrap()).into(),
                            ));
                            frame
                        } else if id.contains("audio") {
                            audio_chunks_received += 1;

                            // Extract session_status from metadata to detect end of response
                            let session_status = if let Some(param) = metadata.parameters.get("session_status") {
                                match param {
                                    dora_node_api::Parameter::String(s) => s.clone(),
                                    _ => "unknown".to_string()
                                }
                            } else {
                                "unknown".to_string()
                            };

                            // Check if this is a session end signal (empty audio with special marker)
                            // Handle both boolean and string representations (Python True -> String or Bool)
                            let is_session_end_signal = if let Some(param) = metadata.parameters.get("is_session_end_signal") {
                                match param {
                                    dora_node_api::Parameter::Bool(b) => *b,
                                    dora_node_api::Parameter::String(s) => s.to_lowercase() == "true",
                                    _ => false
                                }
                            } else {
                                false
                            };

                            // Log audio processing with enhanced detail
                            if id.contains("primespeech") {
                                send_log(&mut node, "DEBUG", &format!("[{}ms] TTS OUTPUT: {} -> WebSocket, {} samples, {} bytes, chunk #{}, session_status: {}, is_session_end_signal: {}", time_since_last, id, data.len(), data.get_array_memory_size(), audio_chunks_received, session_status, is_session_end_signal));
                            } else if id.contains("audio-player") {
                                send_log(&mut node, "DEBUG", &format!("[{}ms] AUDIO PLAYBACK: {} -> WebSocket, {} samples, chunk #{}, session_status: {}", time_since_last, id, data.len(), audio_chunks_received, session_status));
                            } else {
                                send_log(&mut node, "DEBUG", &format!("[{}ms] AUDIO OUTPUT: {} -> WebSocket, {} samples, chunk #{}, session_status: {}", time_since_last, id, data.len(), audio_chunks_received, session_status));
                            }

                            // Handle session end signal: send completion events without processing audio
                            if is_session_end_signal && session_status == "ended" {
                                send_log(&mut node, "INFO", "Received session end signal (empty audio with session_status=ended)");

                                // Send completion events immediately
                                // Send response.audio.done
                                let audio_done = OpenAIRealtimeResponse::ResponseAudioDone {
                                    response_id: "123".to_string(),
                                    item_id: "123".to_string(),
                                    output_index: 123,
                                    content_index: 123,
                                };
                                let audio_done_frame = Frame::text(Payload::Bytes(
                                    Bytes::from(serde_json::to_string(&audio_done).unwrap()).into(),
                                ));
                                ws.write_frame(audio_done_frame).await?;
                                send_log(&mut node, "DEBUG", "Sent response.audio.done for session end");

                                // Send response.done
                                let response_done = OpenAIRealtimeResponse::ResponseDone {
                                    response: serde_json::json!({
                                        "id": "123",
                                        "status": "completed",
                                        "status_details": null,
                                        "output": [],
                                        "usage": {
                                            "total_tokens": 0,
                                            "input_tokens": 0,
                                            "output_tokens": 0,
                                            "input_token_details": {
                                                "cached_tokens": 0,
                                                "text_tokens": 0,
                                                "audio_tokens": 0
                                            },
                                            "output_token_details": {
                                                "cached_tokens": 0,
                                                "text_tokens": 0,
                                                "audio_tokens": 0
                                            }
                                        }
                                    }),
                                };
                                let response_done_frame = Frame::text(Payload::Bytes(
                                    Bytes::from(serde_json::to_string(&response_done).unwrap()).into(),
                                ));
                                ws.write_frame(response_done_frame).await?;
                                send_log(&mut node, "INFO", "Sent response.done for session end - conversation complete");

                                // Reset response state
                                response_created_sent = false;
                                response_active = false;

                                // Skip normal audio processing
                                continue;
                            }

                            // Handle audio data - it might be a list/array
                            let audio_data = if let Ok(vec_data) = into_vec::<f32>(&data) {
                                send_log(&mut node, "DEBUG", &format!("Extracted {} audio samples using into_vec", vec_data.len()));
                                vec_data
                            } else {
                                // Try different array types
                                if let Some(array) = data.as_any().downcast_ref::<dora_node_api::arrow::array::Float32Array>() {
                                    send_log(&mut node, "DEBUG", &format!("Converting from Float32Array ({} samples)", array.len()));
                                    let mut vec_data = Vec::with_capacity(array.len());
                                    for i in 0..array.len() {
                                        if array.is_valid(i) {
                                            vec_data.push(array.value(i));
                                        }
                                    }
                                    vec_data
                                } else if let Some(list_array) = data.as_any().downcast_ref::<dora_node_api::arrow::array::ListArray>() {
                                    // For PrimeSpeech: pa.array([audio_array]) creates a list with one element
                                    if list_array.len() > 0 {
                                        // Get the first (and usually only) element
                                        let values = list_array.value(0);

                                        if let Some(float_array) = values.as_any().downcast_ref::<dora_node_api::arrow::array::Float32Array>() {
                                            let mut vec_data = Vec::with_capacity(float_array.len());
                                            for i in 0..float_array.len() {
                                                if float_array.is_valid(i) {
                                                    vec_data.push(float_array.value(i));
                                                }
                                            }
                                            vec_data
                                        } else {
                                            send_log(&mut node, "ERROR", &format!("ListArray element is not Float32Array, it's: {:?}", values.data_type()));
                                            continue;
                                        }
                                    } else {
                                        send_log(&mut node, "ERROR", "Empty ListArray");
                                        continue;
                                    }
                                } else {
                                    send_log(&mut node, "ERROR", "Unknown array type, cannot downcast");
                                    continue;
                                }
                            };
                            
                            // For TTS, process immediately without buffering for low latency
                            // Create a resampler for this chunk (use source sample_rate from metadata if present)
                            let params = SincInterpolationParameters {
                                sinc_len: 64,  // Lower for faster processing
                                f_cutoff: 0.95,
                                interpolation: SincInterpolationType::Linear,
                                oversampling_factor: 128,
                                window: WindowFunction::Blackman,
                            };
                            // Determine source sample rate (fallback to 32000) and resample to 24000
                            let src_rate: f64 = if let Some(param) = metadata.parameters.get("sample_rate") {
                                match param { dora_node_api::Parameter::Integer(i) => (*i as f64).max(1.0), _ => 32000.0 }
                            } else { 32000.0 };
                            let dst_rate: f64 = 24000.0;
                            let ratio = (dst_rate / src_rate).max(1e-6);
                            send_log(&mut node, "DEBUG", &format!("Resampling {} -> {} (ratio {:.5})", src_rate as i64, dst_rate as i64, ratio));

                            let mut resampler = SincFixedIn::<f32>::new(
                                ratio,
                                2.0,
                                params,
                                audio_data.len().max(1),   // Avoid zero-sized buffer
                                1,
                            ).expect("Failed to create TTS resampler");

                            let input = vec![audio_data];
                            let output = resampler.process(&input, None).expect("TTS resampling failed");
                            let resampled = output[0].clone();
                            send_log(&mut node, "DEBUG", &format!("Resampled {} -> {} samples", input[0].len(), resampled.len()));

                            let data = convert_f32_to_pcm16(&resampled);
                            send_log(&mut node, "DEBUG", &format!("Encoded PCM16 bytes: {}", data.len()));

                            // Ensure we notify the client a response was created before first delta
                            if !response_created_sent {
                                let created = OpenAIRealtimeResponse::ResponseCreated {
                                    response: serde_json::json!({
                                        "id": "123",
                                        "status": "in_progress",
                                        "output": []
                                    }),
                                };
                                let created_frame = Frame::text(Payload::Bytes(
                                    Bytes::from(serde_json::to_string(&created).unwrap()).into(),
                                ));
                                ws.write_frame(created_frame).await?;
                                send_log(&mut node, "DEBUG", "Sent response.created (id=123)");
                                response_created_sent = true;
                                response_active = true;
                            }
                            let serialized_data = OpenAIRealtimeResponse::ResponseAudioDelta {
                                response_id: "123".to_string(),
                                item_id: "123".to_string(),
                                output_index: 123,
                                content_index: 123,
                                delta: general_purpose::STANDARD.encode(data),
                            };

                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&serialized_data).unwrap())
                                    .into(),
                            ));
                            
                            // Check if this is the last segment using session_status
                            if session_status == "ended" {
                                send_log(&mut node, "INFO", "Last segment detected (session_status=ended), will send completion events AFTER audio");
                                should_send_completion = true;
                            }

                            // Return the audio frame to be sent first
                            frame
                        } else if id.contains("speech_started") {
                            let serialized_data =
                                OpenAIRealtimeResponse::InputAudioBufferSpeechStarted {
                                    audio_start_ms: 123,
                                    item_id: "123".to_string(),
                                };

                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&serialized_data).unwrap())
                                    .into(),
                            ));
                            frame
                        } else if id.contains("question_ended") {
                            send_log(&mut node, "INFO", "Question ended detected - complete sentence, triggering LLM response");
                            
                            // Send speech stopped event to indicate a complete question
                            let speech_stopped = OpenAIRealtimeResponse::InputAudioBufferSpeechStopped {
                                audio_end_ms: 123,
                                item_id: "123".to_string(),
                            };
                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&speech_stopped).unwrap()).into(),
                            ));
                            
                            // Queue additional events to send after this frame
                            // When using server_vad, we need to:
                            // 1. Commit the audio buffer 
                            // 2. Create a response
                            
                            // Store the frame to return, but also queue commit and response
                            let commit_msg = serde_json::json!({
                                "type": "input_audio_buffer.committed",
                                "item_id": "123",
                                "audio": ""  // Empty audio since we already sent it
                            });
                            let commit_frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&commit_msg).unwrap()).into(),
                            ));
                            
                            // Create response to trigger LLM
                            let create_response = serde_json::json!({
                                "type": "response.create",
                                "response": {
                                    "modalities": ["text", "audio"],
                                    "instructions": null,
                                    "voice": null,
                                    "output_audio_format": "pcm16",
                                    "tools": [],
                                    "tool_choice": "none",
                                    "temperature": 0.8,
                                    "max_output_tokens": 4096
                                }
                            });
                            let response_frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&create_response).unwrap()).into(),
                            ));
                            
                            // Send all three events
                            // Note: We can only return one frame here, so we'll need to handle this differently
                            // For now, just send the speech_stopped event
                            frame
                        } else {
                            // Ignore other inputs (e.g., question_ended, is_speaking, speech_probability, log)
                            continue;
                        }
                    }
                    dora_node_api::Event::Error(_) => {
                        // Keep the WebSocket open on upstream errors; just skip this event
                        continue;
                    }
                    dora_node_api::Event::InputClosed { id } => {
                        // Do NOT close the WebSocket when a single input closes (e.g., text).
                        // This event only indicates no more items for that input; the session continues.
                        send_log(&mut node, "DEBUG", &format!("Dora input closed: {:?}", id));
                        continue;
                    }
                    _ => {
                        // Ignore other event types to keep the connection alive
                        continue;
                    },
                };
                Some(frame)
            }
            future::Either::Left(None) => break,
            future::Either::Right(Ok(frame)) => {
                // println!("Received WebSocket frame, opcode: {:?}, payload size: {}", frame.opcode, frame.payload.len());
                match frame.opcode {
                    OpCode::Close => {
                        // Echo close and break the loop
                        if !close_replied {
                            ws.write_frame(Frame::close(1000, b"Normal closure")).await?;
                            close_replied = true;
                        }
                        break
                    },
                    OpCode::Ping => {
                        // Respond to ping with pong to keep the connection alive
                        let pong = Frame::pong(frame.payload);
                        ws.write_frame(pong).await?;
                        continue;
                    }
                    OpCode::Pong => {
                        // Ignore
                        continue;
                    }
                    OpCode::Text | OpCode::Binary => {
                        // Parse client JSON safely; ignore unknown or malformed messages
                        let parsed: Result<OpenAIRealtimeMessage, _> =
                            serde_json::from_slice(&frame.payload);
                        let data = match parsed {
                            Ok(d) => d,
                            Err(e) => {
                                send_log(&mut node, "WARNING", &format!("Ignoring malformed client message ({} bytes): {}", frame.payload.len(), e));
                                // Keep the connection open; move to next iteration
                                continue;
                            }
                        };
                        // println!("Parsed WebSocket message type: {:?}", std::mem::discriminant(&data));

                        match data {
                            OpenAIRealtimeMessage::InputAudioBufferAppend { audio } => {
                                // Log audio input periodically (every 50 chunks to avoid spam)
                                static AUDIO_INPUT_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                                let count = AUDIO_INPUT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if count % 50 == 0 {
                                    send_log(&mut node, "DEBUG", &format!("📡 Audio input chunk #{}, base64 len: {}", count, audio.len()));
                                }
                                let f32_data = audio;
                                // Decode base64 encoded audio data
                                let f32_data = f32_data.trim();
                                if f32_data.is_empty() {
                                    send_log(&mut node, "WARNING", "Empty audio buffer received, skipping");
                                    continue;
                                }

                                match general_purpose::STANDARD.decode(f32_data) {
                                    Ok(decoded_data) => {
                                        let f32_data = convert_pcm16_to_f32(&decoded_data);

                                        // Add to buffer
                                        audio_buffer.extend_from_slice(&f32_data);

                                        // Process buffer in fixed-size chunks
                                        while audio_buffer.len() >= DOWNSAMPLE_CHUNK_SIZE {
                                            // Take exactly DOWNSAMPLE_CHUNK_SIZE samples
                                            let chunk: Vec<f32> = audio_buffer.drain(..DOWNSAMPLE_CHUNK_SIZE).collect();

                                            // Resample using the pre-created downsampler
                                            let input = vec![chunk];
                                            let output = downsampler.process(&input, None).expect("Resampling failed");
                                            let f32_data = output[0].clone();

                                            let mut parameter = MetadataParameters::default();
                                            parameter.insert(
                                                "sample_rate".to_string(),
                                                dora_node_api::Parameter::Integer(16000),
                                            );
                                            match node.send_output(
                                                DataId::from("audio".to_string()),
                                                parameter,
                                                f32_data.to_vec().into_arrow(),  // Create a fresh vector copy
                                            ) {
                                                Ok(_) => {
                                                    // Log forwarded audio periodically
                                                    static AUDIO_FWD_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                                                    let fwd_count = AUDIO_FWD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                                    if fwd_count % 50 == 0 {
                                                        send_log(&mut node, "DEBUG", &format!("🎤 Audio forwarded to dataflow #{}, {} samples @ 16kHz", fwd_count, f32_data.len()));
                                                    }
                                                },
                                                Err(e) => send_log(&mut node, "ERROR", &format!("Failed to send audio: {:?}", e)),
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        send_log(&mut node, "ERROR", &format!("Failed to decode base64 audio: {:?}, length: {}, first 100 chars: {}", e, f32_data.len(), &f32_data[..f32_data.len().min(100)]));
                                    }
                                }
                            }
                            OpenAIRealtimeMessage::InputAudioBufferCommit => {
                                send_log(&mut node, "DEBUG", "Received InputAudioBufferCommit from client");
                                // Don't break - just continue processing
                                // This allows continuous audio streaming
                                continue;
                            }
                            OpenAIRealtimeMessage::ResponseCreate { response } => {
                                send_log(&mut node, "INFO", "Received ResponseCreate from client with instructions");
                                if let Some(text) = response.instructions {
                                    // Throttle duplicate greetings: ignore if same as the last within 3s
                                    let now = std::time::Instant::now();
                                    let is_dup = last_greeting.as_ref().map(|g| g == &text).unwrap_or(false)
                                        && last_greet_time.map(|t| now.duration_since(t).as_millis() < 3000).unwrap_or(false);
                                    if is_dup {
                                        send_log(&mut node, "DEBUG", "Ignoring duplicate greeting within 3s window");
                                    } else {
                                        send_log(&mut node, "INFO", &format!("Forwarding greeting instructions to maas-client: {}", text));
                                        match node.send_output(
                                            DataId::from("text".to_string()),
                                            Default::default(),
                                            text.clone().into_arrow(),
                                        ) {
                                            Ok(_) => {
                                                last_greeting = Some(text);
                                                last_greet_time = Some(now);
                                                send_log(&mut node, "INFO", "Successfully sent greeting to maas-client, waiting for LLM response and TTS audio...");
                                            }
                                            Err(e) => {
                                                send_log(&mut node, "WARNING", &format!("Failed to send greeting to maas-client: {:?} (maas-client might not be fully connected yet)", e));
                                            }
                                        }
                                    }
                                }
                            }
                            OpenAIRealtimeMessage::Other => {
                                // Unknown/unsupported client message type; skip
                                continue;
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        // Ignore other client message variants; keep the connection open
                        continue;
                    },
                }
                None
            }
            future::Either::Right(Err(_)) => break,
        };
        if let Some(frame) = frame {
            // Check if this is a question_ended event that needs additional frames
            let is_question_ended = if let Frame { payload: Payload::Bytes(ref data), .. } = frame {
                let text = String::from_utf8_lossy(data);
                text.contains("input_audio_buffer.speech_stopped")
            } else {
                false
            };
            
            ws.write_frame(frame).await?;
            
            // If question ended, do NOT send client-origin events back to the client.
            // We already forward ASR text to the MaaS client, which triggers the LLM.
            // Sending `input_audio_buffer.committed` or `response.create` from server→client
            // is invalid for the OpenAI Realtime protocol and can cause disconnects.
            if is_question_ended {
                send_log(&mut node, "DEBUG", "Question ended detected; relying on server-side LLM trigger (no client-origin events sent)");
            }

            // Send completion events immediately after audio frame if this was the last segment
            if should_send_completion {
                send_log(&mut node, "INFO", "Sending completion events after last audio segment");

                // Send response.audio.done
                let audio_done = OpenAIRealtimeResponse::ResponseAudioDone {
                    response_id: "123".to_string(),
                    item_id: "123".to_string(),
                    output_index: 123,
                    content_index: 123,
                };
                let audio_done_frame = Frame::text(Payload::Bytes(
                    Bytes::from(serde_json::to_string(&audio_done).unwrap()).into(),
                ));
                ws.write_frame(audio_done_frame).await?;
                send_log(&mut node, "DEBUG", "Sent response.audio.done");

                // Send response.done
                let response_done = OpenAIRealtimeResponse::ResponseDone {
                    response: serde_json::json!({
                        "id": "123",
                        "status": "completed",
                        "status_details": null,
                        "output": [],
                        "usage": {
                            "total_tokens": 0,
                            "input_tokens": 0,
                            "output_tokens": 0,
                            "input_token_details": {
                                "cached_tokens": 0,
                                "text_tokens": 0,
                                "audio_tokens": 0
                            },
                            "output_token_details": {
                                "cached_tokens": 0,
                                "text_tokens": 0,
                                "audio_tokens": 0
                            }
                        }
                    }),
                };
                let response_done_frame = Frame::text(Payload::Bytes(
                    Bytes::from(serde_json::to_string(&response_done).unwrap()).into(),
                ));
                ws.write_frame(response_done_frame).await?;
                send_log(&mut node, "INFO", "Sent response.done - conversation complete");

                // Reset flags; mark response as closed to suppress late deltas
                should_send_completion = false;
                response_created_sent = false;
                response_active = false;
            }
        }
    }

    // Drop the node and events locks BEFORE cleanup to avoid deadlock with send_log_async
    drop(node);
    drop(events);

    // Connection closed - send a proper close if we haven't yet
    // Use a timeout to avoid hanging on dead connections
    if !close_replied {
        let close_result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            ws.write_frame(Frame::close(1000, b"Normal closure"))
        ).await;
        if close_result.is_err() {
            send_log_async("DEBUG", "Timeout sending close frame (client may have disconnected abruptly)").await;
        }
    }
    send_log_async("INFO", "WebSocket client disconnected").await;

    // Send reset signal to clear state in downstream nodes (ASR, text-segmenter, TTS)
    send_reset_async().await;

    // Keep the dora node connection alive - don't clear it
    // The dataflow connection persists across WebSocket client sessions
    // CONNECTION_ACTIVE is reset by the ConnectionGuard when this function exits
    send_log_async("INFO", "Session ended, dora connection kept alive for next client").await;

    Ok(())
}
async fn server_upgrade(
    mut req: Request<Incoming>,
) -> Result<Response<Empty<Bytes>>, WebSocketError> {
    send_log_async("DEBUG", &format!("WebSocket upgrade request received: {:?} {:?}", req.method(), req.uri())).await;

    let (response, fut) = upgrade::upgrade(&mut req)?;
    send_log_async("INFO", "WebSocket upgrade successful").await;

    tokio::task::spawn(async move {
        if let Err(e) = tokio::task::unconstrained(handle_client(fut)).await {
            send_log_async("ERROR", &format!("Error in websocket connection: {}", e)).await;
        }
    });

    Ok(response)
}

fn main() -> Result<(), WebSocketError> {
    // Initialize log level from environment variable (default: INFO)
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
    let log_level_value = get_log_level_value(&log_level);
    let _ = LOG_LEVEL_THRESHOLD.set(log_level_value);
    println!("[websocket-server] [INFO] Log level set to: {} (threshold: {})", log_level.to_uppercase(), log_level_value);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async move {
        send_log_async("INFO", "WebSocket server starting...").await;

        // Parse command line arguments
        let args: Vec<String> = std::env::args().collect();
        let mut node_name: Option<String> = None;
        let mut dataflow_path: Option<String> = None;

        // Look for --name and --dataflow arguments
        let mut i = 1; // Skip program name
        while i < args.len() {
            match args[i].as_str() {
                "--name" | "-n" => {
                    if i + 1 < args.len() {
                        node_name = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("[websocket-server] [ERROR] --name requires a value");
                        i += 1;
                    }
                }
                "--dataflow" | "-d" => {
                    if i + 1 < args.len() {
                        dataflow_path = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("[websocket-server] [ERROR] --dataflow requires a value");
                        i += 1;
                    }
                }
                "--help" | "-h" => {
                    println!("dora-openai-websocket - WebSocket server for OpenAI Realtime API");
                    println!();
                    println!("USAGE:");
                    println!("    dora-openai-websocket [OPTIONS]");
                    println!();
                    println!("OPTIONS:");
                    println!("    -n, --name <NODE_NAME>      Dynamic node name to connect as (required)");
                    println!("    -d, --dataflow <PATH>       Path to dataflow YAML file to start (required)");
                    println!("    -h, --help                  Print help information");
                    println!();
                    println!("ENVIRONMENT VARIABLES:");
                    println!("    DORA_NODE_NAME, DORA_NODE_ID    Node name (if --name not provided)");
                    println!("    DATAFLOW_PATH                   Dataflow file path (if --dataflow not provided)");
                    println!("    PORT                            WebSocket server port (default: 8123)");
                    println!("    HOST                            WebSocket server host (default: 0.0.0.0)");
                    return Ok(());
                }
                _ => {
                    // Check if it looks like a dataflow file (positional argument)
                    if args[i].ends_with(".yml") || args[i].ends_with(".yaml") {
                        dataflow_path = Some(args[i].clone());
                    }
                    i += 1;
                }
            }
        }

        // Fallback to environment variables if not provided via args
        // Priority: --name arg > DORA_NODE_NAME > DORA_NODE_ID > default
        if node_name.is_none() {
            if let Ok(name) = std::env::var("DORA_NODE_NAME") {
                println!("[websocket-server] [DEBUG] Using node name from DORA_NODE_NAME env: {}", name);
                node_name = Some(name);
            } else if let Ok(name) = std::env::var("DORA_NODE_ID") {
                println!("[websocket-server] [DEBUG] Using node name from DORA_NODE_ID env: {}", name);
                node_name = Some(name);
            }
        } else {
            println!("[websocket-server] [DEBUG] Using node name from --name argument: {}", node_name.as_ref().unwrap());
        }

        // Priority: --dataflow arg > DATAFLOW_PATH env > search current directory
        let dataflow_path = if let Some(path) = dataflow_path {
            println!("[websocket-server] [DEBUG] Using dataflow from --dataflow argument: {}", path);
            path
        } else if let Ok(path) = std::env::var("DATAFLOW_PATH") {
            println!("[websocket-server] [DEBUG] Using dataflow from DATAFLOW_PATH env: {}", path);
            path
        } else {
            // Search for any .yml file in current directory
            let mut found_path = None;
            if let Ok(entries) = std::fs::read_dir(".") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "yml" || ext == "yaml" {
                            if let Some(name) = path.file_name() {
                                let name_str = name.to_string_lossy();
                                // Prefer files that look like dataflow configs
                                if name_str.contains("dataflow") || name_str.contains("chatbot") || name_str.contains("whisper") {
                                    found_path = Some(path.to_string_lossy().to_string());
                                    break;
                                } else if found_path.is_none() {
                                    found_path = Some(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }

            if let Some(path) = found_path {
                println!("[websocket-server] [DEBUG] Auto-detected dataflow file: {}", path);
                path
            } else {
                eprintln!("[websocket-server] [ERROR] No dataflow file specified and none found in current directory");
                eprintln!("[websocket-server] [INFO] Usage: dora-openai-websocket --dataflow <path.yml> [--name <node_name>]");
                return Err(WebSocketError::InvalidConnectionHeader);
            }
        };

        println!("[websocket-server] [INFO] Using dataflow: {}", dataflow_path);

        // First ensure dora daemon is running
        let list_output = Command::new("dora")
            .arg("list")
            .output()
            .await;

        let daemon_running = match &list_output {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // If stderr contains "Could not connect" or similar, daemon isn't running
                !stderr.contains("Could not connect") && !stderr.contains("Connection refused")
            }
            Err(_) => false,
        };

        if !daemon_running {
            println!("[websocket-server] [INFO] Dora daemon not running, starting with 'dora up'...");
            let up_output = Command::new("dora")
                .arg("up")
                .output()
                .await
                .expect("Failed to execute dora up command");

            if !up_output.status.success() {
                let stderr = String::from_utf8_lossy(&up_output.stderr);
                // Ignore "already running" errors
                if !stderr.contains("already") {
                    eprintln!("[websocket-server] [ERROR] Failed to start dora daemon: {}", stderr);
                    return Err(WebSocketError::InvalidConnectionHeader);
                }
            }
            println!("[websocket-server] [INFO] ✅ Dora daemon started");
            // Give daemon time to initialize
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Check if any dataflow is already running
        let list_output = Command::new("dora")
            .arg("list")
            .output()
            .await
            .expect("Failed to execute dora list command");

        let list_str = String::from_utf8_lossy(&list_output.stdout);
        // Check if output contains a dataflow with "Running" status
        // The format is: UUID  Name  Status
        let has_running_dataflow = list_str.lines()
            .skip(1) // Skip header
            .any(|line| !line.trim().is_empty() && line.contains("Running"));

        println!("[websocket-server] [DEBUG] dora list output:\n{}", list_str);

        if !has_running_dataflow {
            println!("[websocket-server] [INFO] No dataflow running, starting dataflow...");

            let output = Command::new("dora")
                .arg("start")
                .arg(&dataflow_path)
                .arg("--detach")
                .output()
                .await
                .expect("Failed to execute dora start command");

            if !output.status.success() {
                eprintln!("[websocket-server] [ERROR] Failed to start dataflow: {}", String::from_utf8_lossy(&output.stderr));
                return Err(WebSocketError::InvalidConnectionHeader);
            }

            println!("[websocket-server] [INFO] ✅ Dataflow started successfully");

            // Wait for nodes to initialize (Python nodes like ASR/TTS need more time)
            println!("[websocket-server] [INFO] Waiting for nodes to initialize (5s)...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        } else {
            println!("[websocket-server] [INFO] ✅ Dataflow already running");
        }

        // Require node name to be specified
        if node_name.is_none() {
            eprintln!("[websocket-server] [ERROR] Node name not specified");
            eprintln!("[websocket-server] [INFO] Usage: dora-openai-websocket --dataflow <path.yml> --name <node_name>");
            eprintln!("[websocket-server] [INFO] The node name should match the dynamic node ID in your dataflow YAML");
            return Err(WebSocketError::InvalidConnectionHeader);
        }

        // Connect to dataflow as dynamic node with retry logic
        // Priority: --name argument > DORA_NODE_NAME/DORA_NODE_ID env > init_from_env()
        let max_retries = 5;
        let mut retry_count = 0;
        let mut init_result = None;

        while retry_count < max_retries {
            let result = if let Some(name) = &node_name {
                if retry_count == 0 {
                    send_log_async("INFO", &format!("Connecting to dataflow as dynamic node: {}", name)).await;
                }
                DoraNode::init_from_node_id(NodeId::from(name.clone()))
            } else {
                send_log_async("INFO", "No node name provided, trying init_from_env()...").await;
                DoraNode::init_from_env()
            };

            match result {
                Ok(res) => {
                    init_result = Some(Ok(res));
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count < max_retries {
                        println!("[websocket-server] [WARNING] Connection attempt {} failed, retrying in 2s... ({})", retry_count, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    } else {
                        init_result = Some(Err(e));
                    }
                }
            }
        }

        match init_result.unwrap() {
            Ok((node, events)) => {
                let node_id = node.id().to_string();
                send_log_async("INFO", &format!("Successfully connected to dataflow as '{}'", node_id)).await;

                // Store the node name for potential reconnection
                if let Some(name) = &node_name {
                    let _ = DORA_NODE_NAME.set(name.clone());
                }

                // Initialize RwLock containers (only once)
                let _ = DORA_NODE.set(RwLock::new(Some(Arc::new(Mutex::new(node)))));
                let _ = DORA_EVENTS.set(RwLock::new(Some(Arc::new(Mutex::new(events)))));

                // Initialize MAAS_PID storage (will be set when client connects)
                MAAS_PID.set(Arc::new(Mutex::new(None))).unwrap_or_else(|_| panic!("Failed to set MAAS_PID"));

                send_log_async("INFO", "Dora node and events stored globally").await;
                send_log_async("INFO", "Waiting for WebSocket client to connect before spawning maas-client...").await;
            }
            Err(e) => {
                eprintln!("[websocket-server] [ERROR] Failed to connect to dataflow: {:?}", e);
                eprintln!("[websocket-server] [ERROR] The WebSocket server requires a dataflow connection to function.");
                eprintln!("[websocket-server] [INFO] Usage:");
                eprintln!("[websocket-server] [INFO]   dora-openai-websocket --dataflow <path.yml> --name <node_name>");
                eprintln!("[websocket-server] [INFO] Or set environment variables: DATAFLOW_PATH, DORA_NODE_NAME");
                return Err(WebSocketError::InvalidConnectionHeader);
            }
        }

        let port = std::env::var("PORT").unwrap_or_else(|_| "8123".to_string());
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;
        send_log_async("INFO", &format!("WebSocket server ready, listening on {}", addr)).await;
        send_log_async("INFO", "Press Ctrl+C to stop server and dataflow").await;

        // Set up Ctrl+C handler
        let shutdown_signal = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            println!("\n[websocket-server] [INFO] Ctrl+C received, shutting down...");
        };

        // Run the server with graceful shutdown
        tokio::select! {
            _ = async {
                loop {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            send_log_async("INFO", "Client connected").await;

                            tokio::spawn(async move {
                                let io = hyper_util::rt::TokioIo::new(stream);
                                let conn_fut = http1::Builder::new()
                                    .serve_connection(io, service_fn(server_upgrade))
                                    .with_upgrades();
                                if let Err(e) = conn_fut.await {
                                    send_log_async("ERROR", &format!("An error occurred: {:?}", e)).await;
                                }
                            });
                        }
                        Err(e) => {
                            send_log_async("ERROR", &format!("Accept error: {}", e)).await;
                        }
                    }
                }
            } => {}
            _ = shutdown_signal => {
                println!("[websocket-server] [INFO] Stopping running dataflows...");

                // First get the list of running dataflows
                let list_output = Command::new("dora")
                    .arg("list")
                    .output()
                    .await;

                if let Ok(output) = list_output {
                    let list_str = String::from_utf8_lossy(&output.stdout);

                    // Find all running dataflows and stop them
                    for line in list_str.lines().skip(1) {
                        if line.contains("Running") {
                            // Extract UUID (first column)
                            if let Some(uuid) = line.split_whitespace().next() {
                                println!("[websocket-server] [INFO] Stopping dataflow: {}", uuid);

                                let stop_output = Command::new("dora")
                                    .arg("stop")
                                    .arg(uuid)
                                    .output()
                                    .await;

                                match stop_output {
                                    Ok(output) => {
                                        if output.status.success() {
                                            println!("[websocket-server] [INFO] ✅ Dataflow {} stopped", uuid);
                                        } else {
                                            let stdout = String::from_utf8_lossy(&output.stdout);
                                            let stderr = String::from_utf8_lossy(&output.stderr);
                                            let error_msg = if !stderr.is_empty() {
                                                stderr.to_string()
                                            } else if !stdout.is_empty() {
                                                stdout.to_string()
                                            } else {
                                                format!("exit code: {:?}", output.status.code())
                                            };
                                            // Check if it's just "not found" which means already stopped
                                            if error_msg.contains("not found") || error_msg.contains("No running") {
                                                println!("[websocket-server] [INFO] Dataflow {} already stopped", uuid);
                                            } else {
                                                println!("[websocket-server] [WARNING] Failed to stop {}: {}", uuid, error_msg.trim());
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("[websocket-server] [WARNING] Failed to execute dora stop: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                println!("[websocket-server] [INFO] Server shutdown complete");
            }
        }

        Ok(())
    })
}
