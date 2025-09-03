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
use dora_node_api::IntoArrow;
use dora_node_api::MetadataParameters;
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};
use fastwebsockets::upgrade;
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::Payload;
use fastwebsockets::WebSocketError;
use std::process::Command;
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
use rand::random;
use serde;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use tokio::net::TcpListener;

// Helper function for hybrid logging - tries send_log first, falls back to println
#[allow(dead_code)]
fn log_message(node_opt: Option<&mut DoraNode>, level: &str, message: &str) {
    if let Some(node) = node_opt {
        // Try to send through dora logging system
        let log_data = serde_json::json!({
            "node": "websocket-server",
            "level": level,
            "message": message
        });
        
        if let Err(_) = node.send_output(
            DataId::from("log".to_string()),
            Default::default(),
            serde_json::to_string(&log_data).unwrap().into_arrow(),
        ) {
            // Fallback to println if node output fails
            println!("[{}] {}", level, message);
        }
    } else {
        // No node available, use println
        println!("[{}] {}", level, message);
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

/// Replaces a placeholder in a file and writes the result to an output file.
///
/// # Arguments
///
/// * `input_path` - Path to the input file with placeholder text.
/// * `placeholder` - The placeholder text to search for (e.g., "{{PLACEHOLDER}}").
/// * `replacement` - The text to replace the placeholder with.
/// * `output_path` - Path to write the modified content.
fn replace_placeholder_in_file(
    input_path: &str,
    replacement: &HashMap<String, String>,
    output_path: &str,
) -> io::Result<()> {
    // Read the file content into a string
    let mut content = fs::read_to_string(input_path)?;

    // Replace the placeholder
    for (placeholder, replacement) in replacement {
        // Ensure the placeholder is wrapped in curly braces
        // Replace the placeholder with the replacement text
        content = content.replace(placeholder, replacement);
    }

    // Write the modified content to the output file
    let mut file = fs::File::create(output_path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}


async fn handle_client(fut: upgrade::UpgradeFut) -> Result<(), WebSocketError> {
    println!("WebSocket client connected, waiting for upgrade completion");
    let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);
    println!("WebSocket connection established, waiting for first message");

    let frame = ws.read_frame().await?;
    println!("Received first frame, opcode: {:?}, payload size: {}", frame.opcode, frame.payload.len());
    
    if frame.opcode != OpCode::Text {
        println!("ERROR: Expected text frame, got {:?}", frame.opcode);
        return Err(WebSocketError::InvalidConnectionHeader);
    }
    
    println!("Parsing message as OpenAIRealtimeMessage");
    let data: OpenAIRealtimeMessage = match serde_json::from_slice(&frame.payload) {
        Ok(msg) => {
            println!("Successfully parsed message");
            msg
        },
        Err(e) => {
            println!("ERROR: Failed to parse message: {}", e);
            println!("Raw payload: {}", String::from_utf8_lossy(&frame.payload));
            return Err(WebSocketError::InvalidConnectionHeader);
        }
    };
    
    let OpenAIRealtimeMessage::SessionUpdate { session } = data else {
        println!("ERROR: Expected SessionUpdate, got different message type");
        return Err(WebSocketError::InvalidConnectionHeader);
    };
    println!("Received SessionUpdate from client");

    let input_audio_transcription = session
        .input_audio_transcription
        .as_ref()
        .map_or("whisper".to_string(), |t| {
            println!("Client requested transcription model: {}", t.model);
            t.model.clone()
        });
    let llm = session.model.clone();
    println!("Session config - Transcription: {}, LLM: {}", input_audio_transcription, llm);
    
    // Accept any model name from moly, but log what we're actually using
    println!("Client requested model: {}", llm);
    if llm.contains("Qwen") || llm.contains("GGUF") {
        println!("Note: Client requested a Qwen/GGUF model, will use whatever is configured in the template");
    }
    
    // Generate a unique ID for this connection
    // Each connection needs its own dataflow because the WebSocket server is a dynamic node
    let id = random::<u16>();
    let node_id = format!("server-{}", id);
    let dataflow = format!("{}-{}.yml", input_audio_transcription, id);
    let mut template = format!("{}-template-metal.yml", input_audio_transcription);
    // Prefer Linux/generic templates if present
    let candidates = [
        format!("{}-template-linux.yml", input_audio_transcription),
        format!("{}-template.yml", input_audio_transcription),
        template.clone(),
    ];
    for cand in &candidates {
        if std::path::Path::new(cand).exists() {
            template = cand.clone();
            break;
        }
    }
    
    println!("Looking for template file: {}", template);
    if !std::path::Path::new(&template).exists() {
        println!("WARNING: Template file not found: {}", template);
        // Try to find any available template as fallback
        let fallback_template = "whisper-template-metal.yml";
        if std::path::Path::new(fallback_template).exists() {
            println!("Using fallback template: {}", fallback_template);
            template = fallback_template.to_string();
        } else {
            println!("Available template files:");
            if let Ok(entries) = std::fs::read_dir(".") {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("yml") 
                            && path.to_string_lossy().contains("template") {
                            println!("  - {}", path.display());
                            // Use the first template we find as fallback
                            if template == format!("{input_audio_transcription}-template-metal.yml") {
                                template = path.to_string_lossy().to_string();
                                println!("Using fallback template: {}", template);
                            }
                        }
                    }
                }
            }
            // If still no template found, return error
            if !std::path::Path::new(&template).exists() {
                println!("ERROR: No template files found!");
                return Err(WebSocketError::InvalidConnectionHeader);
            }
        }
    }
    
    // Create the dataflow file from template
    println!("Creating dataflow '{}' from template '{}' with node_id '{}'", dataflow, template, node_id);
    let mut replacements = HashMap::new();
    replacements.insert("NODE_ID".to_string(), node_id.clone());
    replacements.insert("LLM_ID".to_string(), llm);
    replace_placeholder_in_file(&template, &replacements, &dataflow).unwrap();
    // Copy configuration file but replace the node ID with "server-id"
    // Read the configuration file and replace the node ID with "server-id"
    // Send session responses FIRST before starting dataflow
    // This allows moly to transition from "connecting" to "listening"
    let session_response = serde_json::json!({
        "id": format!("session_{}", node_id),
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
    
    let serialized_data = OpenAIRealtimeResponse::SessionCreated {
        session: session_response.clone(),
    };

    let payload =
        Payload::Bytes(Bytes::from(serde_json::to_string(&serialized_data).unwrap()).into());
    let frame = Frame::text(payload);
    println!("Sending session.created acknowledgment to client");
    ws.write_frame(frame).await?;
    
    // Also send session.updated to confirm the session update was processed
    let serialized_updated = OpenAIRealtimeResponse::SessionUpdated {
        session: session_response,
    };
    let payload_updated =
        Payload::Bytes(Bytes::from(serde_json::to_string(&serialized_updated).unwrap()).into());
    let frame_updated = Frame::text(payload_updated);
    println!("Sending session.updated acknowledgment to client");
    ws.write_frame(frame_updated).await?;
    
    // Build the dataflow first to avoid race with coordinator
    println!("Building dataflow {} before start", dataflow);
    let build_output = std::process::Command::new("dora")
        .arg("build")
        .arg(&dataflow)
        .output()
        .expect("Failed to execute dora build command");
    if !build_output.status.success() {
        eprintln!("Failed to build dataflow: {}", String::from_utf8_lossy(&build_output.stderr));
        ws.write_frame(Frame::close(1011, b"Failed to build dataflow")).await?;
        return Err(WebSocketError::InvalidConnectionHeader);
    }
    // Start the dataflow using dora CLI
    println!("Starting dataflow {} with node_id {}", dataflow, node_id);
    let output = std::process::Command::new("dora")
        .arg("start")
        .arg(&dataflow)
        .arg("--name")
        .arg(&node_id)
        .arg("--detach")
        .output()
        .expect("Failed to execute dora start command");
    
    if !output.status.success() {
        eprintln!("Failed to start dataflow: {}", String::from_utf8_lossy(&output.stderr));
        ws.write_frame(Frame::close(1011, b"Failed to start dataflow")).await?;
        return Err(WebSocketError::InvalidConnectionHeader);
    }
    
    println!("✅ Dataflow started successfully with node_id: {}", node_id);
    
    // Poll for dataflow to be fully initialized instead of fixed wait
    println!("Waiting for dataflow to initialize...");
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 30;  // 30 * 200ms = 6 seconds max wait
    const POLL_INTERVAL_MS: u64 = 200;
    
    loop {
        // Check if dataflow is running using dora list
        let list_output = Command::new("dora")
            .arg("list")
            .output()
            .expect("Failed to execute dora list command");
        
        if list_output.status.success() {
            let output_str = String::from_utf8_lossy(&list_output.stdout);
            // Check if our node_id appears in the list and is Running
            if output_str.contains(&node_id) && output_str.contains("Running") {
                println!("✅ Dataflow {} is confirmed running", node_id);
                // Add a small additional delay to ensure all nodes are initialized
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                break;
            }
        }
        
        retry_count += 1;
        if retry_count >= MAX_RETRIES {
            eprintln!("❌ Timeout waiting for dataflow {} to be ready after {} seconds", 
                     node_id, (MAX_RETRIES as u64 * POLL_INTERVAL_MS) / 1000);
            ws.write_frame(Frame::close(1011, b"Dataflow initialization timeout")).await?;
            return Err(WebSocketError::InvalidConnectionHeader);
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    
    // Try to initialize the Dora node with retries
    println!("Attempting to initialize Dora node with ID: {}", node_id);
    
    let mut node_init_retries = 0;
    const MAX_NODE_INIT_RETRIES: u32 = 3;
    const NODE_INIT_TIMEOUT_SECS: u64 = 10;  // Longer timeout per attempt
    
    let (mut node, mut events) = loop {
        node_init_retries += 1;
        println!("Dynamic node connection attempt {} of {}", node_init_retries, MAX_NODE_INIT_RETRIES);
        
        let node_id_clone = node_id.clone();
        let node_init_handle = tokio::task::spawn_blocking(move || {
            DoraNode::init_from_node_id(NodeId::from(node_id_clone))
        });
        
        match tokio::time::timeout(
            std::time::Duration::from_secs(NODE_INIT_TIMEOUT_SECS),
            node_init_handle
        ).await {
            Ok(Ok(Ok((n, e)))) => {
                println!("✅ Dora node initialized successfully as '{}' on attempt {}", node_id, node_init_retries);
                break (n, e);
            },
            Ok(Ok(Err(e))) if node_init_retries < MAX_NODE_INIT_RETRIES => {
                println!("⚠️  Failed to initialize Dora node '{}' on attempt {}: {:?}", node_id, node_init_retries, e);
                println!("Waiting 1 second before retry...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            },
            Ok(Err(_)) | Err(_) if node_init_retries < MAX_NODE_INIT_RETRIES => {
                println!("⚠️  Dora node initialization timed out for '{}' on attempt {}", node_id, node_init_retries);
                println!("Waiting 1 second before retry...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            },
            _ => {
                println!("❌ Failed to initialize Dora node '{}' after {} attempts", node_id, MAX_NODE_INIT_RETRIES);
                println!("Continuing without Dora node connection - audio forwarding will not work");
                // Just maintain the websocket connection
                loop {
                    match ws.read_frame().await {
                        Ok(frame) if frame.opcode == OpCode::Close => break,
                        Ok(_) => continue,
                        Err(_) => break,
                    }
                }
                return Ok(());
            }
        }
    };
    
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
    
    println!("🚀 Starting main event loop with fixed-size resampler");
    println!("📊 Pipeline: WebSocket → ASR → MaaS → Text-Segmenter → TTS → Audio → WebSocket");
    println!("👂 Listening for audio input and text output from dataflow nodes...");
    println!("" );
    
    let mut audio_chunks_received = 0;
    let mut text_chunks_sent = 0;
    let mut last_activity = std::time::Instant::now();
    loop {
        let event_fut = events.recv_async().map(Either::Left);
        let frame_fut = ws.read_frame().map(Either::Right);
        let event_stream = (event_fut, frame_fut).race();
        let mut finished = false;
        let frame = match event_stream.await {
            future::Either::Left(Some(ev)) => {
                let frame = match ev {
                    dora_node_api::Event::Input {
                        id,
                        metadata: _,
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
                            if id.contains("transcription") {
                                println!("┌─ [{}ms] 🎙️  ASR OUTPUT", time_since_last);
                                println!("│  Node: {} → WebSocket", id);
                                println!("│  Text: '{}'", str.chars().take(100).collect::<String>());
                                println!("│  Stats: Chunk #{}, {} chars", text_chunks_sent, str.len());
                                println!("└─ 📤 Forwarding to client as transcription delta");
                            } else if id.contains("text") && (id.contains("maas") || id.contains("qwen")) {
                                println!("┌─ [{}ms] 🤖 LLM OUTPUT", time_since_last);
                                println!("│  Node: {} → WebSocket", id);
                                println!("│  Text: '{}'", str.chars().take(100).collect::<String>());
                                println!("│  Stats: Chunk #{}, {} chars", text_chunks_sent, str.len());
                                println!("└─ 📤 Forwarding to client as text delta");
                            } else {
                                println!("┌─ [{}ms] 📝 TEXT OUTPUT", time_since_last);
                                println!("│  Node: {} → WebSocket", id);
                                println!("│  Text: '{}'", str.chars().take(100).collect::<String>());
                                println!("│  Stats: Chunk #{}, {} chars", text_chunks_sent, str.len());
                                println!("└─ 📤 Forwarding to client");
                            }
                            
                            let serialized_data =
                                OpenAIRealtimeResponse::ResponseAudioTranscriptDelta {
                                    response_id: "123".to_string(),
                                    item_id: "123".to_string(),
                                    output_index: 123,
                                    content_index: 123,
                                    delta: str.to_string(),
                                };

                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&serialized_data).unwrap())
                                    .into(),
                            ));
                            frame
                        } else if id.contains("audio") {
                            audio_chunks_received += 1;
                            
                            // Log audio processing with enhanced detail
                            if id.contains("primespeech") {
                                println!("┌─ [{}ms] 🔊 TTS OUTPUT", time_since_last);
                                println!("│  Node: {} → WebSocket", id);
                                println!("│  Audio: {} samples, {} bytes", data.len(), data.get_array_memory_size());
                                println!("│  Stats: Chunk #{}", audio_chunks_received);
                                println!("└─ 📤 Forwarding to client as audio data");
                            } else if id.contains("audio-player") {
                                println!("┌─ [{}ms] 🎵 AUDIO PLAYBACK", time_since_last);
                                println!("│  Node: {} → WebSocket", id);
                                println!("│  Audio: {} samples", data.len());
                                println!("│  Stats: Chunk #{}", audio_chunks_received);
                                println!("└─ 📤 Forwarding to client");
                            } else {
                                println!("┌─ [{}ms] 🎵 AUDIO OUTPUT", time_since_last);
                                println!("│  Node: {} → WebSocket", id);
                                println!("│  Audio: {} samples", data.len());
                                println!("│  Stats: Chunk #{}", audio_chunks_received);
                                println!("└─ 📤 Forwarding to client");
                            }
                            
                            // Handle audio data - it might be a list/array
                            let audio_data = if let Ok(vec_data) = into_vec::<f32>(&data) {
                                println!("   ✓ Converted {} audio samples using into_vec", vec_data.len());
                                vec_data
                            } else {
                                // Try different array types
                                if let Some(array) = data.as_any().downcast_ref::<dora_node_api::arrow::array::Float32Array>() {
                                    println!("   ✓ Converting from Float32Array ({} samples)", array.len());
                                    let mut vec_data = Vec::with_capacity(array.len());
                                    for i in 0..array.len() {
                                        if array.is_valid(i) {
                                            vec_data.push(array.value(i));
                                        }
                                    }
                                    vec_data
                                } else if let Some(list_array) = data.as_any().downcast_ref::<dora_node_api::arrow::array::ListArray>() {
                                    // println!("Converting from ListArray with {} elements", list_array.len());
                                    // For PrimeSpeech: pa.array([audio_array]) creates a list with one element
                                    if list_array.len() > 0 {
                                        // Get the first (and usually only) element
                                        let values = list_array.value(0);
                                        // println!("ListArray element type: {:?}", values.data_type());
                                        
                                        if let Some(float_array) = values.as_any().downcast_ref::<dora_node_api::arrow::array::Float32Array>() {
                                            // println!("Extracting {} float32 samples from ListArray", float_array.len());
                                            let mut vec_data = Vec::with_capacity(float_array.len());
                                            for i in 0..float_array.len() {
                                                if float_array.is_valid(i) {
                                                    vec_data.push(float_array.value(i));
                                                }
                                            }
                                            vec_data
                                        } else {
                                            println!("ERROR: ListArray element is not Float32Array, it's: {:?}", values.data_type());
                                            continue;
                                        }
                                    } else {
                                        println!("ERROR: Empty ListArray");
                                        continue;
                                    }
                                } else {
                                    println!("ERROR: Unknown array type, cannot downcast");
                                    continue;
                                }
                            };
                            
                            // For TTS, process immediately without buffering for low latency
                            // Create a resampler for this chunk
                            let params = SincInterpolationParameters {
                                sinc_len: 64,  // Lower for faster processing
                                f_cutoff: 0.95,
                                interpolation: SincInterpolationType::Linear,
                                oversampling_factor: 128,
                                window: WindowFunction::Blackman,
                            };
                            
                            let mut resampler = SincFixedIn::<f32>::new(
                                24000.0 / 32000.0,  // Resample ratio (3/4)
                                2.0,
                                params,
                                audio_data.len(),   // Exact input size
                                1,
                            ).expect("Failed to create TTS resampler");
                            
                            let input = vec![audio_data];
                            let output = resampler.process(&input, None).expect("TTS resampling failed");
                            let resampled = output[0].clone();
                            
                            let data = convert_f32_to_pcm16(&resampled);
                            let serialized_data = OpenAIRealtimeResponse::ResponseAudioDelta {
                                response_id: "123".to_string(),
                                item_id: "123".to_string(),
                                output_index: 123,
                                content_index: 123,
                                delta: general_purpose::STANDARD.encode(data),
                            };
                            finished = true;

                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&serialized_data).unwrap())
                                    .into(),
                            ));
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
                        } else if id.contains("speech_stopped") || id.contains("speech_ended") {
                            let serialized_data =
                                OpenAIRealtimeResponse::InputAudioBufferSpeechStopped {
                                    audio_end_ms: 123,
                                    item_id: "123".to_string(),
                                };

                            let frame = Frame::text(Payload::Bytes(
                                Bytes::from(serde_json::to_string(&serialized_data).unwrap())
                                    .into(),
                            ));
                            frame
                        } else {
                            // Ignore other inputs (e.g., question_ended, is_speaking, speech_probability, log)
                            continue;
                        }
                    }
                    dora_node_api::Event::Error(_) => {
                        // println!("Error in input: {}", s);
                        continue;
                    }
                    _ => break,
                };
                Some(frame)
            }
            future::Either::Left(None) => break,
            future::Either::Right(Ok(frame)) => {
                // println!("Received WebSocket frame, opcode: {:?}, payload size: {}", frame.opcode, frame.payload.len());
                match frame.opcode {
                    OpCode::Close => break,
                    OpCode::Text | OpCode::Binary => {
                        let data: OpenAIRealtimeMessage =
                            serde_json::from_slice(&frame.payload).unwrap();
                        // println!("Parsed WebSocket message type: {:?}", std::mem::discriminant(&data));

                        match data {
                            OpenAIRealtimeMessage::InputAudioBufferAppend { audio } => {
                                // println!("Received audio buffer from client, length: {}", audio.len());
                                let f32_data = audio;
                                // Decode base64 encoded audio data
                                let f32_data = f32_data.trim();
                                if f32_data.is_empty() {
                                    continue;
                                }

                                match general_purpose::STANDARD.decode(f32_data) {
                                    Ok(decoded_data) => {
                                        // println!("Decoded {} bytes of PCM16 audio", decoded_data.len());
                                        let f32_data = convert_pcm16_to_f32(&decoded_data);
                                        // println!("Converted to {} f32 samples", f32_data.len());
                                        
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
                                            // println!("Sending {} audio samples to speech-monitor", f32_data.len());
                                            match node.send_output(
                                                DataId::from("audio".to_string()),
                                                parameter,
                                                f32_data.to_vec().into_arrow(),  // Create a fresh vector copy
                                            ) {
                                                Ok(_) => {}, // println!("Successfully sent audio to speech-monitor"),
                                                Err(e) => println!("ERROR sending audio: {:?}", e),
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("ERROR: Failed to decode base64 audio: {:?}", e);
                                        println!("Base64 string length: {}, first 100 chars: {}", 
                                                f32_data.len(), 
                                                &f32_data[..f32_data.len().min(100)]);
                                    }
                                }
                            }
                            OpenAIRealtimeMessage::InputAudioBufferCommit => {
                                // Don't break - just continue processing
                                // This allows continuous audio streaming
                                continue;
                            }
                            OpenAIRealtimeMessage::ResponseCreate { response } => {
                                if let Some(text) = response.instructions {
                                    node.send_output(
                                        DataId::from("text".to_string()),
                                        Default::default(),
                                        text.into_arrow(),
                                    )
                                    .unwrap();
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => break,
                }
                None
            }
            future::Either::Right(Err(_)) => break,
        };
        if let Some(frame) = frame {
            ws.write_frame(frame).await?;
        }
        if finished {
            let serialized_data = OpenAIRealtimeResponse::ResponseDone {
                response: serde_json::Value::Null,
            };

            let payload = Payload::Bytes(
                Bytes::from(serde_json::to_string(&serialized_data).unwrap()).into(),
            );
            println!("Sending response done: {:?}", serialized_data);
            let frame = Frame::text(payload);
            ws.write_frame(frame).await?;
        };
    }
    
    // Connection closed
    println!("🔌 WebSocket client disconnected");
    println!("   Dataflow '{}' with node_id '{}' will be stopped", dataflow, node_id);

    Ok(())
}
async fn server_upgrade(
    mut req: Request<Incoming>,
) -> Result<Response<Empty<Bytes>>, WebSocketError> {
    println!("WebSocket upgrade request received");
    println!("  Method: {:?}", req.method());
    println!("  URI: {:?}", req.uri());
    println!("  Headers:");
    for (name, value) in req.headers() {
        println!("    {}: {:?}", name, value);
    }
    
    let (response, fut) = upgrade::upgrade(&mut req)?;
    println!("WebSocket upgrade successful");

    tokio::task::spawn(async move {
        if let Err(e) = tokio::task::unconstrained(handle_client(fut)).await {
            eprintln!("Error in websocket connection: {}", e);
        }
    });

    Ok(response)
}

fn main() -> Result<(), WebSocketError> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async move {
        let port = std::env::var("PORT").unwrap_or_else(|_| "8123".to_string());
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;
        println!("Server started, listening on {}", addr);
        
        loop {
            let (stream, _) = listener.accept().await?;
            println!("Client connected");
            
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let conn_fut = http1::Builder::new()
                    .serve_connection(io, service_fn(server_upgrade))
                    .with_upgrades();
                if let Err(e) = conn_fut.await {
                    println!("An error occurred: {:?}", e);
                }
            });
        }
    })
}
