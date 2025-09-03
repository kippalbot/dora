use base64::Engine;
use base64::engine::general_purpose;
use dora_node_api::DoraNode;
use dora_node_api::IntoArrow;
use dora_node_api::MetadataParameters;
use dora_node_api::arrow::array::{Array, AsArray};
use dora_node_api::arrow::datatypes::DataType;
use dora_node_api::dora_core::config::DataId;
use dora_node_api::dora_core::config::NodeId;
use dora_node_api::into_vec;
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::Payload;
use fastwebsockets::WebSocketError;
use fastwebsockets::upgrade;
use futures_concurrency::future::Race;
use futures_util::FutureExt;
use futures_util::future;
use futures_util::future::Either;
use http_body_util::Empty;
use hyper::Request;
use hyper::Response;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use rand::random;
use serde;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use tokio::net::TcpListener;

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

fn default_model() -> String {
    "Qwen/Qwen2.5-0.5B-Instruct-GGUF".to_string()
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SessionConfig {
    #[serde(default)]
    pub modalities: Vec<String>,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub input_audio_format: String,
    #[serde(default)]
    pub output_audio_format: String,
    #[serde(default)]
    pub input_audio_transcription: Option<TranscriptionConfig>,
    #[serde(default)]
    pub turn_detection: Option<TurnDetectionConfig>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub tool_choice: String,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default)]
    pub max_response_output_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TranscriptionConfig {
    pub model: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TurnDetectionConfig {
    #[serde(rename = "type")]
    pub detection_type: String,
    pub threshold: f32,
    #[serde(default)]
    pub prefix_padding_ms: u32,
    #[serde(default)]
    pub silence_duration_ms: u32,
    #[serde(default)]
    pub interrupt_response: bool,
    #[serde(default)]
    pub create_response: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseConfig {
    pub modalities: Vec<String>,
    pub instructions: Option<String>,
    pub voice: Option<String>,
    pub output_audio_format: Option<String>,
    pub tools: Option<serde_json::Value>,
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
    println!("Starting WebSocket upgrade...");
    let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);
    println!("WebSocket connection established");

    println!("Waiting for initial frame...");
    let frame = ws.read_frame().await?;
    println!("Received frame with opcode: {:?}, payload size: {}", frame.opcode, frame.payload.len());
    
    if frame.opcode != OpCode::Text {
        eprintln!("ERROR: Expected Text frame, got {:?}", frame.opcode);
        ws.write_frame(Frame::close(1002, b"Protocol error")).await?;
        return Err(WebSocketError::InvalidConnectionHeader);
    }
    
    let data: OpenAIRealtimeMessage = match serde_json::from_slice(&frame.payload) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("ERROR: Failed to parse initial message: {}", e);
            eprintln!("Payload: {:?}", String::from_utf8_lossy(&frame.payload));
            ws.write_frame(Frame::close(1003, b"Unsupported data")).await?;
            return Err(WebSocketError::InvalidConnectionHeader);
        }
    };
    
    let OpenAIRealtimeMessage::SessionUpdate { session } = data else {
        eprintln!("ERROR: Expected SessionUpdate, got different message type");
        ws.write_frame(Frame::close(1003, b"Unsupported data")).await?;
        return Err(WebSocketError::InvalidConnectionHeader);
    };
    println!("Session update received successfully");

    let _tools = serde_json::to_string(&session.tools).unwrap();
    let input_audio_transcription = session
        .input_audio_transcription
        .map_or("moyoyo-whisper".to_string(), |t| t.model);
    let llm = session.model.clone();
    let id = random::<u16>();
    let node_id = format!("server-{id}");
    let dataflow = format!("{input_audio_transcription}-{}.yml", id);
    // Pick a template suited for the platform
    let candidates = [
        format!("{input_audio_transcription}-template-linux.yml"),
        format!("{input_audio_transcription}-template.yml"),
        format!("{input_audio_transcription}-template-metal.yml"),
    ];
    let template = candidates
        .iter()
        .find(|p| std::path::Path::new(p.as_str()).exists())
        .cloned()
        .unwrap_or_else(|| format!("{input_audio_transcription}-template-metal.yml"));
    let mut replacements = HashMap::new();
    replacements.insert("NODE_ID".to_string(), node_id.clone());
    replacements.insert("LLM_ID".to_string(), llm);
    println!("Filling template: {}", template);
    replace_placeholder_in_file(&template, &replacements, &dataflow).unwrap();
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
    // Start the dataflow using dora CLI with the node_id as the name
    println!("Starting dataflow {} with name {}", dataflow, node_id);
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
    
    println!("Dataflow started successfully");
    
    // Wait for dataflow to be fully initialized
    println!("Waiting for dataflow to initialize...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    let (mut node, mut events) = match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
        Ok((n, e)) => {
            println!("Successfully connected as dynamic node: {}", node_id);
            (n, e)
        }
        Err(e) => {
            eprintln!("Failed to connect as dynamic node {}: {:?}", node_id, e);
            ws.write_frame(Frame::close(1011, b"Server error - failed to connect to dataflow")).await?;
            return Err(WebSocketError::InvalidConnectionHeader);
        }
    };
    let serialized_data = OpenAIRealtimeResponse::SessionCreated {
        session: serde_json::Value::Null,
    };

    let payload =
        Payload::Bytes(Bytes::from(serde_json::to_string(&serialized_data).unwrap()).into());
    let frame = Frame::text(payload);
    ws.write_frame(frame).await?;
    
    // Setup resampling for audio
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
                        if data.data_type() == &DataType::Utf8 {
                            let data = data.as_string::<i32>();
                            let str = data.value(0);
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
                            // Handle audio data - it might be a list/array from PrimeSpeech
                            let audio_data = if let Ok(vec_data) = into_vec::<f32>(&data) {
                                vec_data
                            } else {
                                // Try different array types
                                if let Some(array) = data.as_any().downcast_ref::<dora_node_api::arrow::array::Float32Array>() {
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
                                            eprintln!("ERROR: ListArray element is not Float32Array, it's: {:?}", values.data_type());
                                            continue;
                                        }
                                    } else {
                                        eprintln!("ERROR: Empty ListArray");
                                        continue;
                                    }
                                } else {
                                    eprintln!("ERROR: Unknown array type, cannot downcast");
                                    continue;
                                }
                            };
                            
                            // For TTS, resample from 32kHz to 24kHz immediately without buffering for low latency
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
                        } else if id.contains("speech_stopped") {
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
                match frame.opcode {
                    OpCode::Close => break,
                    OpCode::Text | OpCode::Binary => {
                        let data: OpenAIRealtimeMessage = match serde_json::from_slice(&frame.payload) {
                            Ok(d) => d,
                            Err(e) => {
                                eprintln!("Failed to parse WebSocket message: {}", e);
                                eprintln!("Raw payload: {:?}", String::from_utf8_lossy(&frame.payload));
                                continue;
                            }
                        };

                        match data {
                            OpenAIRealtimeMessage::InputAudioBufferAppend { audio } => {
                                let f32_data = audio;
                                // Decode base64 encoded audio data
                                let f32_data = f32_data.trim();
                                if f32_data.is_empty() {
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
                                            
                                            // Resample the chunk from 24kHz to 16kHz
                                            let input = vec![chunk];
                                            let output = downsampler.process(&input, None).expect("Downsampling failed");
                                            let resampled = output[0].clone();
                                            
                                            // Send resampled audio to Dora
                                            let mut parameter = MetadataParameters::default();
                                            parameter.insert(
                                                "sample_rate".to_string(),
                                                dora_node_api::Parameter::Integer(16000),
                                            );
                                            node.send_output(
                                                DataId::from("audio".to_string()),
                                                parameter,
                                                resampled.into_arrow(),
                                            )
                                            .unwrap();
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to decode base64 audio: {}", e);
                                    }
                                }
                            }
                            OpenAIRealtimeMessage::InputAudioBufferCommit => {
                                // Just acknowledge the commit, don't break the connection
                                println!("Audio buffer committed");
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
                    OpCode::Ping => {
                        // Respond to ping with pong
                        let pong = Frame::pong(frame.payload);
                        ws.write_frame(pong).await?;
                        println!("Responded to ping with pong");
                    }
                    OpCode::Pong => {
                        // Pong received, no action needed
                        println!("Received pong");
                    }
                    _ => {
                        // Ignore other opcodes
                        println!("Received unknown opcode: {:?}", frame.opcode);
                    }
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

    // Send proper close frame before exiting
    ws.write_frame(Frame::close(1000, b"Normal closure")).await?;
    
    Ok(())
}

async fn server_upgrade(
    mut req: Request<Incoming>,
) -> Result<Response<Empty<Bytes>>, WebSocketError> {
    let (response, fut) = upgrade::upgrade(&mut req)?;

    tokio::task::spawn(async move {
        if let Err(e) = tokio::task::unconstrained(handle_client(fut)).await {
            eprintln!("Error in websocket connection: {}", e);
        }
    });

    Ok(response)
}

pub fn lib_main() -> Result<(), WebSocketError> {
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

// Python bindings would go here if needed
// Currently not implemented as this is a standalone Rust node
