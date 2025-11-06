//! Dora MaaS Client - Model-as-a-Service client for Dora dataflows
//!
//! This node provides cloud AI integration for Dora applications, serving as a
//! drop-in replacement for local LLM nodes. It features:
//!
//! - Multi-provider support (OpenAI, Gemini, etc.)
//! - Real-time streaming with SSE
//! - Intelligent text segmentation for TTS
//! - Session-based conversation management
//! - Event-driven architecture without threading
//!
//! # Architecture
//!
//! The client operates as a Dora node, processing events in a single async loop:
//! 1. Receives text input events from ASR or other nodes
//! 2. Routes requests to configured cloud providers
//! 3. Streams responses through the segmenter
//! 4. Emits segmented text for TTS processing

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dora_node_api::{
    DoraNode, Event, Parameter,
    arrow::array::{AsArray, StringArray},
    dora_core::config::DataId,
};
use eyre::{Context, Result};
use outfox_openai::spec::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent, ChatCompletionTool,
    ChatCompletionToolType, CreateChatCompletionRequest, FunctionObject, PartibleTextContent,
};
use serde_json::json;

mod client;
mod config;
mod segmenter;
mod streaming;
mod tool;

use config::Config;
use segmenter::StreamSegmenter;
use tool::ToolSet;

// Helper function to send log messages
fn send_log(node: &mut DoraNode, level: &str, message: &str) -> Result<()> {
    let log_data = json!({
        "node": "maas-client",
        "level": level,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp()
    });
    node.send_output(
        DataId::from("log".to_string()),
        Default::default(),
        StringArray::from(vec![log_data.to_string().as_str()]),
    )
    .context("Failed to send log output")?;
    Ok(())
}

struct ChatSession {
    messages: Vec<ChatCompletionRequestMessage>,
    total_tokens: usize,
    tool_set: Option<Arc<Mutex<ToolSet>>>,
}

impl ChatSession {
    fn new(system_prompt: String) -> Self {
        let system_message =
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: PartibleTextContent::Text(system_prompt),
                name: None,
            });

        Self {
            messages: vec![system_message],
            total_tokens: 0,
            tool_set: None, // Will be set separately
        }
    }

    fn set_tool_set(&mut self, tool_set: Arc<Mutex<ToolSet>>) {
        self.tool_set = Some(tool_set);
    }

    fn has_tools(&self) -> bool {
        self.tool_set
            .as_ref()
            .and_then(|ts| ts.lock().ok())
            .map(|ts| ts.has_tools())
            .unwrap_or(false)
    }

    fn get_tool_definitions(&self) -> Option<Vec<ChatCompletionTool>> {
        self.tool_set
            .as_ref()
            .and_then(|ts| ts.lock().ok())
            .map(|ts| {
                ts.tools()
                    .iter()
                    .map(|tool| ChatCompletionTool {
                        kind: ChatCompletionToolType::Function,
                        function: FunctionObject {
                            name: tool.name(),
                            description: Some(tool.description()),
                            parameters: Some(tool.parameters()),
                            strict: None,
                        },
                    })
                    .collect()
            })
    }

    fn add_user_message(&mut self, content: String) {
        let message = ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(content),
            name: None,
        });
        self.messages.push(message);
    }

    fn add_assistant_message(&mut self, content: String) {
        let message =
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(content)),
                name: None,
                tool_calls: None,
                audio: None,
                refusal: None,
            });
        self.messages.push(message);
    }

    fn add_assistant_message_with_tools(
        &mut self,
        content: String,
        tool_calls: Vec<ChatCompletionMessageToolCall>,
    ) {
        let message =
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: if content.is_empty() {
                    None
                } else {
                    Some(ChatCompletionRequestAssistantMessageContent::Text(content))
                },
                name: None,
                tool_calls: Some(tool_calls),
                audio: None,
                refusal: None,
            });
        self.messages.push(message);
    }

    fn add_tool_message(&mut self, tool_call_id: String, content: String) {
        let message = ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
            content: PartibleTextContent::Text(content),
            tool_call_id,
        });
        self.messages.push(message);
    }

    fn manage_history(&mut self, max_exchanges: usize) {
        // Keep system message + last N exchanges (N*2 messages)
        let max_messages = 1 + (max_exchanges * 2);
        if self.messages.len() > max_messages {
            let excess = self.messages.len() - max_messages;
            // Remove old messages but keep system prompt
            self.messages.drain(1..=excess);
        }
    }

    fn reset(&mut self) {
        // Keep only system message
        self.messages.truncate(1);
        self.total_tokens = 0;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check if running as dynamic node with --name argument
    let args: Vec<String> = std::env::args().collect();
    let node_id = if args.len() > 2 && args[1] == "--name" {
        Some(args[2].clone())
    } else {
        None
    };

    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Log level is available for future use
    let _log_level = &config.log_level;

    // Initialize MCP tools if enabled
    let tool_set = if config.enable_tools {
        match config.init_tool_set().await {
            Ok(Some(ts)) => {
                let tool_count = ts.tools().len();
                eprintln!("Initialized {} MCP tools", tool_count);
                Some(Arc::new(Mutex::new(ts)))
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!("Warning: Failed to initialize MCP tools: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create provider clients
    let clients = config.create_clients();

    // Initialize Dora node - use node_id if provided (dynamic node), otherwise from env
    let (mut node, events) = if let Some(id) = node_id {
        eprintln!("Initializing as dynamic node with ID: {}", id);
        match DoraNode::init_from_node_id(dora_node_api::dora_core::config::NodeId::from(
            id.clone(),
        )) {
            Ok((n, e)) => {
                eprintln!("✅ Successfully initialized dynamic node '{}'", id);
                (n, e)
            }
            Err(e) => {
                eprintln!("❌ Failed to initialize dynamic node '{}': {:?}", id, e);
                return Err(e.into());
            }
        }
    } else {
        eprintln!("Initializing from environment variables...");
        match DoraNode::init_from_env() {
            Ok((n, e)) => {
                eprintln!("✅ Successfully initialized node from environment");
                (n, e)
            }
            Err(e) => {
                eprintln!("❌ Failed to initialize node from environment: {:?}", e);
                return Err(e.into());
            }
        }
    };

    // Send initialization logs
    send_log(&mut node, "INFO", "MaaS Client initialized")?;
    send_log(
        &mut node,
        "INFO",
        &format!("Model: {}", config.default_model),
    )?;
    send_log(
        &mut node,
        "INFO",
        &format!("Providers: {}", config.providers.len()),
    )?;

    // Session storage
    let mut sessions: HashMap<String, ChatSession> = HashMap::new();

    // Process events
    eprintln!("🔵 [MAAS-CLIENT] Starting event loop...");
    let events = futures::executor::block_on_stream(events);
    eprintln!("🔵 [MAAS-CLIENT] Event stream created, waiting for events...");

    let mut event_count = 0;
    for event in events {
        event_count += 1;
        eprintln!(
            "🔵 [MAAS-CLIENT] Received event #{}: {:?}",
            event_count, event
        );
        match event {
            Event::Input { id, data, metadata } => {
                // Extract session ID from metadata
                let session_id = metadata
                    .parameters
                    .get("session_id")
                    .and_then(|p| match p {
                        Parameter::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "default".to_string());

                // debug!("Received input '{}' for session '{}'", id, session_id);

                match id.as_str() {
                    "text" | "text_to_audio" => {
                        // Extract text from input
                        let text_array = data.as_string::<i32>();
                        let user_text = text_array
                            .iter()
                            .filter_map(|s| s)
                            .collect::<Vec<_>>()
                            .join(" ");

                        if user_text.is_empty() {
                            send_log(&mut node, "WARNING", "Received empty text input")?;
                            continue;
                        }

                        // Get or create session
                        let session = sessions.entry(session_id.clone()).or_insert_with(|| {
                            let mut session = ChatSession::new(config.system_prompt.clone());
                            if let Some(ref ts) = tool_set {
                                session.set_tool_set(ts.clone());
                            }
                            session
                        });

                        let role = metadata
                            .parameters
                            .get("role")
                            .and_then(|p| match p {
                                Parameter::String(s) => Some(s.as_str()),
                                _ => None,
                            });

                        if matches!(role, Some("assistant")) {
                            send_log(
                                &mut node,
                                "DEBUG",
                                &format!("Caching assistant context: {}", user_text),
                            )?;
                            session.add_assistant_message(user_text.clone());
                            session.manage_history(config.max_history_exchanges);
                            continue;
                        }

                        send_log(&mut node, "INFO", &format!("Processing: {}", user_text))?;

                        // Add user message
                        session.add_user_message(user_text.clone());

                        // Manage history
                        session.manage_history(config.max_history_exchanges);

                        // Process the conversation with automatic tool handling
                        // FIX: Added loop to handle tool calls without waiting for user input
                        // When the LLM calls a tool, we execute it and immediately send the results
                        // back to get the final response, instead of waiting for the next user message
                        let mut continue_conversation = true;
                        while continue_conversation {
                            continue_conversation = false; // Default to not continuing unless we have tool calls

                            // Route to appropriate provider
                            let (provider_id, model_name) =
                                config.route_model(&config.default_model).ok_or_else(|| {
                                    eyre::eyre!(
                                        "No route found for model: {}",
                                        config.default_model
                                    )
                                })?;

                            // Create chat completion request with the mapped model name
                            let mut request = CreateChatCompletionRequest::new(
                                model_name.clone(), // Use the mapped model name, not the config model ID
                                session.messages.clone(),
                            );

                            // Add tool definitions if available
                            if config.enable_tools {
                                if config.enable_local_mcp && session.has_tools() {
                                    // Use local MCP tools
                                    request.tools = session.get_tool_definitions();
                                    send_log(
                                        &mut node,
                                        "DEBUG",
                                        &format!(
                                            "Added {} local MCP tool definitions",
                                            request.tools.as_ref().map(|t| t.len()).unwrap_or(0)
                                        ),
                                    )?;
                                } else if !config.enable_local_mcp {
                                    // Pass through tools from client metadata
                                    if let Some(tools_param) = metadata.parameters.get("tools") {
                                        // Parse tools from metadata (expecting JSON array)
                                        if let Parameter::String(tools_json) = tools_param {
                                            if let Ok(tools) =
                                                serde_json::from_str::<Vec<ChatCompletionTool>>(
                                                    tools_json,
                                                )
                                            {
                                                request.tools = Some(tools.clone());
                                                send_log(
                                                    &mut node,
                                                    "DEBUG",
                                                    &format!(
                                                        "Added {} client-provided tool definitions",
                                                        tools.len()
                                                    ),
                                                )?;
                                            }
                                        }
                                    }
                                }
                            }
                            request.stream = config.enable_streaming;
                            request.temperature = Some(0.7);

                            let client = clients.get(&provider_id).ok_or_else(|| {
                                eyre::eyre!("No client found for provider: {}", provider_id)
                            })?;

                            send_log(
                                &mut node,
                                "DEBUG",
                                &format!(
                                    "Routing to provider '{}' with model '{}'",
                                    provider_id, model_name
                                ),
                            )?;

                            // Send "processing" status when starting API call
                            node.send_output(
                                DataId::from("status".to_string()),
                                Default::default(),
                                StringArray::from(vec!["processing"]),
                            )
                            .context("Failed to send status output")?;

                            // Make API call - use streaming if enabled
                            if config.enable_streaming.unwrap_or(false) {
                                // Streaming mode
                                send_log(&mut node, "DEBUG", "Using streaming mode")?;

                                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                let mut has_sent_segment = false;
                                let mut segment_index: u32 = 0;

                                // Start streaming in background
                                let client_clone = client.clone();
                                let request_clone = request.clone();
                                let stream_handle = tokio::spawn(async move {
                                    client_clone.complete_streaming(request_clone, tx).await
                                });

                                // Use segmenter to buffer chunks into meaningful segments
                                let mut segmenter = StreamSegmenter::new(10); // Max 10 words without punctuation
                                let mut accumulated = String::new();
                                let mut chunk_count = 0;
                                let mut segment_count = 0;

                                while let Some(chunk) = rx.recv().await {
                                    chunk_count += 1;

                                    // Add chunk to segmenter and check if we have a segment ready
                                    if let Some(segment) = segmenter.add_chunk(&chunk) {
                                        accumulated.push_str(&segment);
                                        segment_count += 1;

                                        // Send the meaningful segment with metadata passthrough
                                        let mut segment_metadata = metadata.parameters.clone();
                                        segment_metadata.insert(
                                            "session_status".to_string(),
                                            Parameter::String(
                                                if !has_sent_segment {
                                                    "started".to_string()
                                                } else {
                                                    "ongoing".to_string()
                                                },
                                            ),
                                        );
                                        segment_metadata.insert(
                                            "segment_index".to_string(),
                                            Parameter::String(segment_index.to_string()),
                                        );

                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            segment_metadata,
                                            StringArray::from(vec![segment.as_str()]),
                                        )
                                        .context("Failed to send text segment")?;
                                        has_sent_segment = true;
                                        segment_index += 1;
                                    }
                                }

                                // Flush any remaining buffered content
                                if let Some(final_segment) = segmenter.flush() {
                                    accumulated.push_str(&final_segment);
                                    segment_count += 1;

                                    let mut segment_metadata = metadata.parameters.clone();
                                    segment_metadata.insert(
                                        "session_status".to_string(),
                                        Parameter::String(
                                            if !has_sent_segment {
                                                "started".to_string()
                                            } else {
                                                "ongoing".to_string()
                                            },
                                        ),
                                    );
                                    segment_metadata.insert(
                                        "segment_index".to_string(),
                                        Parameter::String(segment_index.to_string()),
                                    );

                                    node.send_output(
                                        DataId::from("text".to_string()),
                                        segment_metadata,
                                        StringArray::from(vec![final_segment.as_str()]),
                                    )
                                    .context("Failed to send final segment")?;
                                    has_sent_segment = true;
                                    segment_index += 1;

                                    send_log(
                                        &mut node,
                                        "DEBUG",
                                        &format!(
                                            "Sent final segment {} ({} chars)",
                                            segment_count,
                                            final_segment.len()
                                        ),
                                    )?;
                                }

                                // Wait for streaming to complete
                                match stream_handle.await {
                                    Ok(Ok((final_text, tool_calls))) => {
                                        send_log(
                                            &mut node,
                                            "INFO",
                                            &format!(
                                                "Streaming complete: {} chars in {} segments (from {} chunks)",
                                                final_text.len(),
                                                segment_count,
                                                chunk_count
                                            ),
                                        )?;

                                        // Send "complete" status
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec!["complete"]),
                                    )
                                    .context("Failed to send status output")?;

                                    // FIX: Handle tool calls from streaming response
                                    // When the LLM returns tool calls, we either execute them locally (enable_local_mcp=true)
                                    // or pass them back to the client (enable_local_mcp=false)
                                    if let Some(tool_calls) = tool_calls {
                                            send_log(
                                                &mut node,
                                                "INFO",
                                                &format!(
                                                    "Received {} tool calls",
                                                    tool_calls.len()
                                                ),
                                            )?;

                                            // Add assistant message with tool calls first
                                            session.add_assistant_message_with_tools(
                                                final_text.clone(),
                                                tool_calls.clone(),
                                            );

                                            if config.enable_local_mcp {
                                                // Execute tool calls locally
                                                send_log(
                                                    &mut node,
                                                    "INFO",
                                                    "Executing tool calls locally",
                                                )?;

                                                // Execute tool calls and collect results
                                                let mut tool_results = Vec::new();
                                                if let Some(ref tool_set) = session.tool_set {
                                                    for tool_call in &tool_calls {
                                                        send_log(
                                                            &mut node,
                                                            "DEBUG",
                                                            &format!(
                                                                "Calling tool: {} with args: {}",
                                                                tool_call.function.name,
                                                                tool_call.function.arguments
                                                            ),
                                                        )?;

                                                        // Get the tool from the tool set
                                                        let tool = {
                                                            let tool_set_guard =
                                                                tool_set.lock().unwrap();
                                                            tool_set_guard
                                                                .get_tool(&tool_call.function.name)
                                                        };

                                                        let result = if let Some(tool) = tool {
                                                            // Parse arguments
                                                            let args: serde_json::Value =
                                                                serde_json::from_str(
                                                                    &tool_call.function.arguments,
                                                                )
                                                                .unwrap_or(serde_json::Value::Null);

                                                            // Execute the tool
                                                            match tool.call(args).await {
                                                                Ok(result) => {
                                                                    let content = if let Some(
                                                                        contents,
                                                                    ) =
                                                                        result.content
                                                                    {
                                                                        contents
                                                                            .iter()
                                                                            .filter_map(|c| {
                                                                                c.as_text()
                                                                            })
                                                                            .map(|t| t.text.clone())
                                                                            .collect::<Vec<_>>()
                                                                            .join("\n")
                                                                    } else {
                                                                        "Tool executed successfully"
                                                                            .to_string()
                                                                    };
                                                                    send_log(
                                                                        &mut node,
                                                                        "DEBUG",
                                                                        &format!(
                                                                            "Tool result: {}",
                                                                            content
                                                                        ),
                                                                    )?;
                                                                    content
                                                                }
                                                                Err(e) => {
                                                                    send_log(
                                                                        &mut node,
                                                                        "ERROR",
                                                                        &format!(
                                                                            "Tool execution failed: {}",
                                                                            e
                                                                        ),
                                                                    )?;
                                                                    format!("Error: {}", e)
                                                                }
                                                            }
                                                        } else {
                                                            let msg = format!(
                                                                "Tool '{}' not found",
                                                                tool_call.function.name
                                                            );
                                                            send_log(&mut node, "ERROR", &msg)?;
                                                            msg
                                                        };

                                                        // Collect the result to add later
                                                        tool_results
                                                            .push((tool_call.id.clone(), result));
                                                    }
                                                }

                                                // Add all tool results to session
                                                for (tool_call_id, result) in tool_results {
                                                    session.add_tool_message(tool_call_id, result);
                                                }

                                                // After tool execution, immediately make another request to get the final response
                                                // Don't wait for user input - we need to send the tool results back to the LLM
                                                send_log(
                                                    &mut node,
                                                    "DEBUG",
                                                    "Sending tool results back to LLM for final response",
                                                )?;

                                                // Set flag to continue the conversation with tool results
                                                continue_conversation = true;
                                            } else {
                                                // Pass tool calls back to client
                                                send_log(
                                                    &mut node,
                                                    "INFO",
                                                    "Passing tool calls to client",
                                                )?;

                                                // Serialize tool calls and send to client
                                                let tool_calls_json =
                                                    serde_json::to_string(&tool_calls)?;
                                                node.send_output(
                                                    DataId::from("tool_calls".to_string()),
                                                    Default::default(),
                                                    StringArray::from(vec![
                                                        tool_calls_json.as_str(),
                                                    ]),
                                                )
                                                .context("Failed to send tool calls")?;

                                                // Don't continue conversation - wait for tool results from client
                                                continue_conversation = false;
                                            }
                                        } else {
                                            // No tool calls, just add the text message
                                            session.add_assistant_message(final_text.clone());
                                        }
                                    }
                                    Ok(Err(e)) => {
                                        send_log(
                                            &mut node,
                                            "ERROR",
                                            &format!("Streaming error: {}", e),
                                        )?;

                                        // Send "error" status
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec![format!("error: {}", e)]),
                                        )
                                        .context("Failed to send status output")?;

                                        let mut error_metadata = metadata.parameters.clone();
                                        error_metadata.insert(
                                            "session_status".to_string(),
                                            Parameter::String("ended".to_string()),
                                        );

                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            error_metadata,
                                            StringArray::from(vec![
                                                format!("Error: {}", e).as_str(),
                                            ]),
                                        )
                                        .context("Failed to send error")?;
                                    }
                                    Err(e) => {
                                        send_log(
                                            &mut node,
                                            "ERROR",
                                            &format!("Task error: {}", e),
                                        )?;

                                        // Send "error" status
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec![format!("error: {}", e)]),
                                        )
                                        .context("Failed to send status output")?;

                                        let mut error_metadata = metadata.parameters.clone();
                                        error_metadata.insert(
                                            "session_status".to_string(),
                                            Parameter::String("ended".to_string()),
                                        );
                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            error_metadata,
                                            StringArray::from(vec![format!("Error: {}", e).as_str()]),
                                        )
                                        .context("Failed to send error output")?;
                                    }
                                }

                                if has_sent_segment {
                                    let mut end_metadata = metadata.parameters.clone();
                                    end_metadata.insert(
                                        "session_status".to_string(),
                                        Parameter::String("ended".to_string()),
                                    );
                                    end_metadata.insert(
                                        "segment_index".to_string(),
                                        Parameter::String(segment_index.to_string()),
                                    );
                                    node.send_output(
                                        DataId::from("text".to_string()),
                                        end_metadata,
                                        StringArray::from(vec![""]),
                                    )
                                    .context("Failed to send session end marker")?;
                                }
                            } else {
                                // Non-streaming mode
                                match client.complete(request).await {
                                    Ok(response) => {
                                        if let Some(choice) = response.choices.first() {
                                            let content = match &choice.message {
                                        outfox_openai::spec::ChatCompletionResponseMessage { content, .. } => {
                                            content.clone().unwrap_or_default()
                                        }
                                    };

                                            send_log(
                                                &mut node,
                                                "INFO",
                                                &format!(
                                                    "Generated response ({} chars)",
                                                    content.len()
                                                ),
                                            )?;

                                            // Add assistant message to session
                                            session.add_assistant_message(content.clone());

                                            // Send response with metadata passthrough
                                            let mut reply_metadata = metadata.parameters.clone();
                                            reply_metadata.insert(
                                                "session_status".to_string(),
                                                Parameter::String("started".to_string()),
                                            );
                                            node.send_output(
                                                DataId::from("text".to_string()),
                                                reply_metadata,
                                                StringArray::from(vec![content.as_str()]),
                                            )
                                            .context("Failed to send text output")?;

                                            // Send "complete" status
                                            node.send_output(
                                                DataId::from("status".to_string()),
                                                Default::default(),
                                                StringArray::from(vec!["complete"]),
                                            )
                                            .context("Failed to send status output")?;

                                            let mut end_metadata = metadata.parameters.clone();
                                            end_metadata.insert(
                                                "session_status".to_string(),
                                                Parameter::String("ended".to_string()),
                                            );
                                            node.send_output(
                                                DataId::from("text".to_string()),
                                                end_metadata,
                                                StringArray::from(vec![""]),
                                            )
                                            .context("Failed to send session end marker")?;
                                        }
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error: {}", e);
                                        eprintln!("[MAIN] API call error: {}", error_msg);
                                        send_log(&mut node, "ERROR", &error_msg)?;

                                        // Send "error" status
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec![format!("error: {}", e)]),
                                        )
                                        .context("Failed to send status output")?;

                                        // Send error response with metadata passthrough
                                        let mut error_metadata = metadata.parameters.clone();
                                        error_metadata.insert(
                                            "session_status".to_string(),
                                            Parameter::String("ended".to_string()),
                                        );
                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            error_metadata,
                                            StringArray::from(vec![error_msg.as_str()]),
                                        )
                                        .context("Failed to send error output")?;
                                    }
                                }
                            } // Close the else block for non-streaming
                        } // Close the while continue_conversation loop
                    }
                    "tool_results" => {
                        // Handle tool results from client (when enable_local_mcp=false)
                        let results_array = data.as_string::<i32>();
                        if let Some(results_json) = results_array.iter().next().flatten() {
                            send_log(&mut node, "INFO", "Received tool results from client")?;

                            // Parse tool results
                            if let Ok(tool_results) =
                                serde_json::from_str::<Vec<(String, String)>>(results_json)
                            {
                                // Get session
                                if let Some(session) = sessions.get_mut(&session_id) {
                                    // Add tool results to session
                                    for (tool_call_id, result) in tool_results {
                                        session.add_tool_message(tool_call_id.clone(), result);
                                    }

                                    send_log(
                                        &mut node,
                                        "DEBUG",
                                        "Added tool results to session, making API call for final response",
                                    )?;

                                    // Route to appropriate provider
                                    let (provider_id, model_name) = config
                                        .route_model(&config.default_model)
                                        .ok_or_else(|| {
                                            eyre::eyre!(
                                                "No route found for model: {}",
                                                config.default_model
                                            )
                                        })?;

                                    // Create chat completion request with tool results
                                    let mut request = CreateChatCompletionRequest::new(
                                        model_name.clone(),
                                        session.messages.clone(),
                                    );

                                    // Tool definitions should still be included for context
                                    if config.enable_tools && !config.enable_local_mcp {
                                        if let Some(tools_param) = metadata.parameters.get("tools")
                                        {
                                            if let Parameter::String(tools_json) = tools_param {
                                                if let Ok(tools) =
                                                    serde_json::from_str::<Vec<ChatCompletionTool>>(
                                                        tools_json,
                                                    )
                                                {
                                                    request.tools = Some(tools);
                                                }
                                            }
                                        }
                                    }

                                    request.stream = config.enable_streaming;
                                    request.temperature = Some(0.7);

                                    let client = clients.get(&provider_id).ok_or_else(|| {
                                        eyre::eyre!("No client found for provider: {}", provider_id)
                                    })?;

                                    // Send "processing" status
                                    node.send_output(
                                        DataId::from("status".to_string()),
                                        Default::default(),
                                        StringArray::from(vec!["processing"]),
                                    )
                                    .context("Failed to send status output")?;

                                    // Make API call to get final response after tool execution
                                    match client.complete(request).await {
                                        Ok(response) => {
                                            if let Some(choice) = response.choices.first() {
                                                let content = match &choice.message {
                                                    outfox_openai::spec::ChatCompletionResponseMessage { content, .. } => {
                                                        content.clone().unwrap_or_default()
                                                    }
                                                };

                                                send_log(
                                                    &mut node,
                                                    "INFO",
                                                    &format!(
                                                        "Generated response after tool execution ({} chars)",
                                                        content.len()
                                                    ),
                                                )?;

                                                // Add assistant message to session
                                                session.add_assistant_message(content.clone());

                                                // Send response with metadata passthrough
                                                node.send_output(
                                                    DataId::from("text".to_string()),
                                                    metadata.parameters.clone(),
                                                    StringArray::from(vec![content.as_str()]),
                                                )
                                                .context("Failed to send text output")?;

                                                // Send "complete" status
                                                node.send_output(
                                                    DataId::from("status".to_string()),
                                                    Default::default(),
                                                    StringArray::from(vec!["complete"]),
                                                )
                                                .context("Failed to send status output")?;
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg =
                                                format!("Error processing tool results: {}", e);
                                            send_log(&mut node, "ERROR", &error_msg)?;

                                            // Send "error" status
                                            node.send_output(
                                                DataId::from("status".to_string()),
                                                Default::default(),
                                                StringArray::from(vec![format!("error: {}", e)]),
                                            )
                                            .context("Failed to send status output")?;
                                        }
                                    }
                                }
                            } else {
                                send_log(&mut node, "ERROR", "Failed to parse tool results")?;
                            }
                        }
                    }
                    "control" => {
                        // Handle control commands
                        let control_text = data
                            .as_string::<i32>()
                            .iter()
                            .filter_map(|value| value.map(str::to_string))
                            .collect::<Vec<String>>()
                            .join(" ");

                        send_log(&mut node, "INFO", &format!("Received control input: {}", control_text))?;

                        // Try to parse as JSON first, fall back to plain text
                        let parsed = serde_json::from_str::<serde_json::Value>(&control_text)
                            .ok()
                            .and_then(|v| if v.is_object() { Some(v) } else { None });

                        let mut should_reset = false;
                        let mut prompt_text: Option<String> = None;

                        if let Some(json) = parsed {
                            // Handle JSON control input
                            send_log(&mut node, "DEBUG", &format!("Parsed JSON control: {:?}", json))?;

                            if let Some(command) = json.get("command").and_then(|v| v.as_str()) {
                                send_log(&mut node, "DEBUG", &format!("Found command: {}", command))?;
                                if command.eq_ignore_ascii_case("reset") {
                                    should_reset = true;
                                } else if command.eq_ignore_ascii_case("ready") {
                                    // Send ready status
                                    node.send_output(
                                        DataId::from("status".to_string()),
                                        Default::default(),
                                        StringArray::from(vec!["ready"]),
                                    )
                                    .context("Failed to send status output")?;
                                } else if command.eq_ignore_ascii_case("exit") {
                                    sessions.remove(&session_id);
                                    send_log(&mut node, "INFO", &format!("Removed session: {}", session_id))?;
                                }
                            }
                            if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
                                send_log(&mut node, "DEBUG", &format!("Found prompt field: {}", prompt))?;
                                if !prompt.trim().is_empty() {
                                    prompt_text = Some(prompt.to_string());
                                    send_log(&mut node, "INFO", &format!("Extracted prompt from control: {}", prompt))?;
                                }
                            }
                        } else {
                            // Plain text command (backward compatibility)
                            if control_text.eq_ignore_ascii_case("reset") {
                                should_reset = true;
                            } else if control_text.eq_ignore_ascii_case("ready") {
                                node.send_output(
                                    DataId::from("status".to_string()),
                                    Default::default(),
                                    StringArray::from(vec!["ready"]),
                                )
                                .context("Failed to send status output")?;
                            } else if control_text.eq_ignore_ascii_case("exit") {
                                sessions.remove(&session_id);
                                send_log(&mut node, "INFO", &format!("Removed session: {}", session_id))?;
                            }
                        }

                        // Handle reset command
                        if should_reset {
                            if let Some(session) = sessions.get_mut(&session_id) {
                                session.reset();
                                send_log(&mut node, "INFO", &format!("Reset session: {}", session_id))?;
                            }
                            node.send_output(
                                DataId::from("status".to_string()),
                                Default::default(),
                                StringArray::from(vec!["reset"]),
                            )
                            .context("Failed to send status output")?;
                        }

                        // Handle prompt field - process through LLM like text input
                        if let Some(user_text) = prompt_text {
                            send_log(&mut node, "INFO", &format!("Sending prompt from control to API: {}", user_text))?;

                            // Get or create session
                            let session = sessions.entry(session_id.clone()).or_insert_with(|| {
                                let mut session = ChatSession::new(config.system_prompt.clone());
                                if let Some(ref ts) = tool_set {
                                    session.set_tool_set(ts.clone());
                                }
                                session
                            });

                            // Add user message
                            session.add_user_message(user_text.clone());

                            // Manage history
                            session.manage_history(config.max_history_exchanges);

                            // Route to appropriate provider
                            let (provider_id, model_name) =
                                config.route_model(&config.default_model).ok_or_else(|| {
                                    eyre::eyre!(
                                        "No route found for model: {}",
                                        config.default_model
                                    )
                                })?;

                            // Create chat completion request
                            let mut request = CreateChatCompletionRequest::new(
                                model_name.clone(),
                                session.messages.clone(),
                            );

                            request.stream = config.enable_streaming;
                            request.temperature = Some(0.7);

                            let client = clients.get(&provider_id).ok_or_else(|| {
                                eyre::eyre!("No client found for provider: {}", provider_id)
                            })?;

                            // Send "processing" status
                            node.send_output(
                                DataId::from("status".to_string()),
                                Default::default(),
                                StringArray::from(vec!["processing"]),
                            )
                            .context("Failed to send status output")?;

                            // Make API call - use streaming if enabled
                            if config.enable_streaming.unwrap_or(false) {
                                // Streaming mode
                                send_log(&mut node, "DEBUG", "Using streaming mode for control prompt")?;

                                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                let mut has_sent_segment = false;
                                let mut segment_index: u32 = 0;

                                // Start streaming in background
                                let client_clone = client.clone();
                                let request_clone = request.clone();
                                let stream_handle = tokio::spawn(async move {
                                    client_clone.complete_streaming(request_clone, tx).await
                                });

                                send_log(&mut node, "DEBUG", "Stream created successfully, starting event loop")?;

                                // Use segmenter to buffer chunks
                                let mut segmenter = StreamSegmenter::new(10);
                                let mut accumulated = String::new();
                                let mut chunk_count = 0;
                                let mut segment_count = 0;

                                while let Some(chunk) = rx.recv().await {
                                    chunk_count += 1;

                                    if let Some(segment) = segmenter.add_chunk(&chunk) {
                                        accumulated.push_str(&segment);
                                        segment_count += 1;

                                        // Send segment with metadata
                                        let mut segment_metadata = metadata.parameters.clone();
                                        segment_metadata.insert(
                                            "session_status".to_string(),
                                            Parameter::String(
                                                if !has_sent_segment {
                                                    "started".to_string()
                                                } else {
                                                    "ongoing".to_string()
                                                },
                                            ),
                                        );
                                        segment_metadata.insert(
                                            "segment_index".to_string(),
                                            Parameter::String(segment_index.to_string()),
                                        );

                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            segment_metadata,
                                            StringArray::from(vec![segment.as_str()]),
                                        )
                                        .context("Failed to send text segment")?;
                                        has_sent_segment = true;
                                        segment_index += 1;
                                    }
                                }

                                // Flush remaining content
                                if let Some(final_segment) = segmenter.flush() {
                                    accumulated.push_str(&final_segment);
                                    segment_count += 1;

                                    let mut segment_metadata = metadata.parameters.clone();
                                    segment_metadata.insert(
                                        "session_status".to_string(),
                                        Parameter::String(
                                            if !has_sent_segment {
                                                "started".to_string()
                                            } else {
                                                "ongoing".to_string()
                                            },
                                        ),
                                    );
                                    segment_metadata.insert(
                                        "segment_index".to_string(),
                                        Parameter::String(segment_index.to_string()),
                                    );

                                    node.send_output(
                                        DataId::from("text".to_string()),
                                        segment_metadata,
                                        StringArray::from(vec![final_segment.as_str()]),
                                    )
                                    .context("Failed to send final segment")?;
                                }

                                // Wait for stream to complete
                                match stream_handle.await {
                                    Ok(Ok(_)) => {
                                        // Send complete log matching openai-response-client
                                        send_log(
                                            &mut node,
                                            "INFO",
                                            &format!(
                                                "Streaming complete: {} chars across {} segments ({} chunks)",
                                                accumulated.len(),
                                                segment_count,
                                                chunk_count
                                            ),
                                        )?;
                                    }
                                    Ok(Err(e)) => {
                                        send_log(&mut node, "ERROR", &format!("Streaming error: {}", e))?;
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec!["error"]),
                                        )
                                        .context("Failed to send error status")?;
                                        continue;
                                    }
                                    Err(e) => {
                                        send_log(&mut node, "ERROR", &format!("Stream task failed: {}", e))?;
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec!["error"]),
                                        )
                                        .context("Failed to send error status")?;
                                        continue;
                                    }
                                }

                                // Add assistant message to session
                                if !accumulated.is_empty() {
                                    session.add_assistant_message(accumulated.clone());
                                    session.manage_history(config.max_history_exchanges);
                                }

                                // Send final empty text message with session_status="ended" for bridge
                                let mut final_metadata = metadata.parameters.clone();
                                final_metadata.insert(
                                    "session_status".to_string(),
                                    Parameter::String("ended".to_string()),
                                );
                                node.send_output(
                                    DataId::from("text".to_string()),
                                    final_metadata.clone(),
                                    StringArray::from(vec![""]),
                                )
                                .context("Failed to send final text message")?;

                                // Send completion status
                                node.send_output(
                                    DataId::from("status".to_string()),
                                    Default::default(),
                                    StringArray::from(vec!["complete"]),
                                )
                                .context("Failed to send complete status")?;

                            } else {
                                // Non-streaming mode
                                send_log(&mut node, "DEBUG", "Using non-streaming mode for control prompt")?;

                                match client.complete(request.clone()).await {
                                    Ok(response) => {
                                        let assistant_message = response.choices.first()
                                            .and_then(|choice| choice.message.content.clone())
                                            .unwrap_or_default();

                                        // Send response
                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            Default::default(),
                                            StringArray::from(vec![assistant_message.as_str()]),
                                        )
                                        .context("Failed to send text output")?;

                                        // Add to session
                                        session.add_assistant_message(assistant_message);
                                        session.manage_history(config.max_history_exchanges);

                                        // Send final empty text message with session_status="ended" for bridge
                                        let mut final_metadata = metadata.parameters.clone();
                                        final_metadata.insert(
                                            "session_status".to_string(),
                                            Parameter::String("ended".to_string()),
                                        );
                                        node.send_output(
                                            DataId::from("text".to_string()),
                                            final_metadata,
                                            StringArray::from(vec![""]),
                                        )
                                        .context("Failed to send final text message")?;

                                        // Send complete status
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec!["complete"]),
                                        )
                                        .context("Failed to send status output")?;
                                    }
                                    Err(e) => {
                                        send_log(&mut node, "ERROR", &format!("API error: {}", e))?;
                                        node.send_output(
                                            DataId::from("status".to_string()),
                                            Default::default(),
                                            StringArray::from(vec!["error"]),
                                        )
                                        .context("Failed to send error status")?;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        send_log(&mut node, "WARNING", &format!("Unknown input ID: {}", id))?;
                    }
                }
            }
            Event::Stop(_) => {
                send_log(&mut node, "INFO", "Received stop event, shutting down")?;
                break;
            }
            _ => {}
        }
    }

    send_log(&mut node, "INFO", "MaaS Client stopped")?;
    Ok(())
}
