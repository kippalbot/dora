use std::collections::{BTreeMap, HashMap};

use dora_node_api::{
    DoraNode, Event, Parameter,
    arrow::array::{AsArray, StringArray},
    dora_core::config::DataId,
};
use eyre::{Context, Result};
use futures::StreamExt;
use openai_dive::v1::{
    api::Client,
    resources::response::{
        request::{
            ContentInput, InputMessage, ResponseInput, ResponseInputItem, ResponseParametersBuilder,
        },
        response::{OutputContent, ResponseObject, ResponseOutput, ResponseStreamEvent, Role},
    },
};
use openai_dive::v1::error::APIError;
use serde::Serialize;
use tokio::time::{Duration, timeout};

const NODE_NAME: &str = "openai-response-client";

mod config;
mod segmenter;

use config::Config;
use segmenter::StreamSegmenter;

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

struct SessionState {
    messages: Vec<ResponseInputItem>,
}

impl SessionState {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    fn add_user(&mut self, text: String) {
        self.messages.push(ResponseInputItem::Message(InputMessage {
            role: Role::User,
            content: ContentInput::Text(text),
        }));
    }

    fn add_assistant(&mut self, text: String) {
        self.messages.push(ResponseInputItem::Message(InputMessage {
            role: Role::Assistant,
            content: ContentInput::Text(text),
        }));
    }

    fn trim_history(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            let excess = self.messages.len() - max_messages;
            self.messages.drain(0..excess);
        }
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
        eprintln!("Failed to send log output: {:?}", err);
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

fn send_segment(
    node: &mut DoraNode,
    metadata: &BTreeMap<String, Parameter>,
    session_status: &str,
    segment_index: Option<u32>,
    text: &str,
) -> Result<()> {
    let mut meta = metadata.clone();
    meta.insert(
        "session_status".to_string(),
        Parameter::String(session_status.to_string()),
    );
    if let Some(index) = segment_index {
        meta.insert(
            "segment_index".to_string(),
            Parameter::String(index.to_string()),
        );
    }

    node.send_output(
        DataId::from("text".to_string()),
        meta,
        StringArray::from(vec![text]),
    )
    .context("Failed to send text output")?;
    Ok(())
}

#[derive(Serialize, Clone)]
struct ToolFunctionPayload {
    name: String,
    arguments: String,
}

#[derive(Serialize, Clone)]
struct ToolCallPayload {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ToolFunctionPayload,
}

#[derive(Default)]
struct ToolCallState {
    id: Option<String>,
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    arguments_buffer: String,
    emitted: bool,
}

fn send_tool_calls(node: &mut DoraNode, calls: &[ToolCallPayload]) -> Result<()> {
    if calls.is_empty() {
        return Ok(());
    }

    let payload = serde_json::to_string(calls).context("Failed to serialize tool calls")?;
    node.send_output(
        DataId::from("tool_calls".to_string()),
        Default::default(),
        StringArray::from(vec![payload.as_str()]),
    )
    .context("Failed to send tool_calls output")?;
    Ok(())
}

fn resolve_call_key(item_id: &str, item_to_call: &mut HashMap<String, String>) -> String {
    if let Some(existing) = item_to_call.get(item_id) {
        existing.clone()
    } else {
        item_to_call.insert(item_id.to_string(), item_id.to_string());
        item_id.to_string()
    }
}

fn collect_tool_calls_from_object(
    response: &ResponseObject,
    tool_states: &mut HashMap<String, ToolCallState>,
    item_to_call: &mut HashMap<String, String>,
) {
    for output in &response.output {
        if let ResponseOutput::FunctionToolCall(tool) = output {
            let call_key = tool.call_id.clone();
            let state = tool_states.entry(call_key.clone()).or_default();
            state.call_id = Some(call_key.clone());
            if !tool.id.is_empty() {
                state.id = Some(tool.id.clone());
                item_to_call.insert(tool.id.clone(), call_key.clone());
            }
            if !tool.name.is_empty() {
                state.name = Some(tool.name.clone());
            }
            if !tool.arguments.is_empty() {
                state.arguments_buffer = tool.arguments.clone();
                state.arguments = Some(tool.arguments.clone());
            }
        }
    }
}

fn collect_new_tool_calls(
    node: &mut DoraNode,
    tool_states: &mut HashMap<String, ToolCallState>,
    ready_calls: &mut Vec<ToolCallPayload>,
) -> Result<()> {
    let mut new_calls = Vec::new();
    for state in tool_states.values_mut() {
        if state.emitted {
            continue;
        }
        let name = match &state.name {
            Some(n) if !n.is_empty() => n.clone(),
            _ => continue,
        };
        let arguments = match &state.arguments {
            Some(args) => args.clone(),
            None => continue,
        };
        let id = state
            .id
            .clone()
            .or_else(|| state.call_id.clone())
            .unwrap_or_else(|| format!("tool_call_{}", ready_calls.len() + new_calls.len()));

        let payload = ToolCallPayload {
            id,
            call_type: "function".to_string(),
            function: ToolFunctionPayload { name, arguments },
        };
        state.emitted = true;
        new_calls.push(payload);
    }

    if !new_calls.is_empty() {
        ready_calls.extend(new_calls);
        send_tool_calls(node, ready_calls)?;
    }

    Ok(())
}

fn tool_calls_from_response(response: &ResponseObject) -> Vec<ToolCallPayload> {
    let mut calls = Vec::new();
    for output in &response.output {
        if let ResponseOutput::FunctionToolCall(tool) = output {
            let id = if !tool.id.is_empty() {
                tool.id.clone()
            } else {
                tool.call_id.clone()
            };
            calls.push(ToolCallPayload {
                id,
                call_type: "function".to_string(),
                function: ToolFunctionPayload {
                    name: tool.name.clone(),
                    arguments: tool.arguments.clone(),
                },
            });
        }
    }
    calls
}

fn extract_text_output(response: &ResponseObject) -> Option<String> {
    let mut buffer = String::new();

    for item in &response.output {
        if let ResponseOutput::Message(message) = item {
            if message.role != Role::Assistant {
                continue;
            }

            for content in &message.content {
                if let OutputContent::Text { text, .. } = content {
                    buffer.push_str(&text);
                }
            }
        }
    }

    if buffer.is_empty() {
        None
    } else {
        Some(buffer)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("[DEBUG] openai-response-client starting");
    let config = Config::load().context("Failed to load configuration")?;
    eprintln!("[DEBUG] Configuration loaded: default_model={}", config.default_model);

    let configured_log_level = LogLevel::parse(config.log_level()).unwrap_or(LogLevel::Info);

    let (provider_config, model_name) = config
        .resolve_model()
        .context("Failed to resolve default model route")?;
    let provider_id = provider_config.id.clone();

    let mut client = Client::new_from_env();

    let base_url = config
        .api_base
        .clone()
        .unwrap_or_else(|| provider_config.api_url.clone());
    if !base_url.is_empty() {
        client.set_base_url(&base_url);
    }

    let streaming_enabled = config.enable_streaming();
    let support_tools = config.enable_tools;

    let status_timeout = config
        .status_timeout_seconds
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(60));

    let mut sessions: HashMap<String, SessionState> = HashMap::new();

    eprintln!("[DEBUG] Initializing Dora node...");
    let (mut node, mut events) =
        DoraNode::init_from_env().context("Failed to initialize Dora node from environment")?;
    eprintln!("[DEBUG] Dora node initialized, node_name={}", node.id());
    let node_name = node.id().to_string();
    send_log(
        &mut node,
        LogLevel::Info,
        configured_log_level,
        "OpenAI response client initialized",
    );
    send_log(
        &mut node,
        LogLevel::Info,
        configured_log_level,
        &format!(
            "Default model: {} -> {} (provider: {})",
            config.default_model,
            model_name.as_str(),
            provider_id
        ),
    );
    send_log(
        &mut node,
        LogLevel::Info,
        configured_log_level,
        &format!(
            "Tools enabled: {}, local MCP: {}",
            support_tools, config.enable_local_mcp
        ),
    );

    while let Some(event) = events.next().await {
        match event {
            Event::Input { id, data, metadata } => {
                let metadata_parameters = metadata.parameters.clone();
                let session_id = metadata_parameters
                    .get("session_id")
                    .and_then(|value| match value {
                        Parameter::String(value) => Some(value.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "default".to_string());

                match id.as_str() {
                    "text" | "text_to_audio" => {
                        let text_array = data.as_string::<i32>();
                        let user_text = text_array
                            .iter()
                            .filter_map(|value| value.map(str::to_string))
                            .collect::<Vec<String>>()
                            .join(" ");

                        if user_text.trim().is_empty() {
                            send_log(
                                &mut node,
                                LogLevel::Warn,
                                configured_log_level,
                                "Received empty text input, skipping request",
                            );
                            continue;
                        }

                        let role_override =
                            metadata_parameters
                                .get("role")
                                .and_then(|value| match value {
                                    Parameter::String(value) => Some(value.clone()),
                                    _ => None,
                                });

                        let session = sessions
                            .entry(session_id.clone())
                            .or_insert_with(SessionState::new);

                        if matches!(role_override.as_deref(), Some("assistant")) {
                            send_log(
                                &mut node,
                                LogLevel::Debug,
                                configured_log_level,
                                &format!("Caching assistant context for session '{session_id}'"),
                            );
                            session.add_assistant(user_text.clone());
                            session.trim_history(config.max_history_exchanges);
                            continue;
                        }

                        session.add_user(user_text.clone());
                        session.trim_history(config.max_history_exchanges);

                        send_log(
                            &mut node,
                            LogLevel::Info,
                            configured_log_level,
                            &format!("Sending prompt to OpenAI: {}", user_text),
                        );

                        send_status(&mut node, "processing")?;

                        let mut builder = ResponseParametersBuilder::default();
                        builder.model(model_name.clone());
                        builder.input(ResponseInput::List(session.messages.clone()));
                        if let Some(prompt) = &config.system_prompt {
                            builder.instructions(prompt.clone());
                        }
                        if streaming_enabled {
                            builder.stream(true);
                        }

                        let parameters = match builder.build() {
                            Ok(parameters) => parameters,
                            Err(error) => {
                                send_log(
                                    &mut node,
                                    LogLevel::Error,
                                    configured_log_level,
                                    &format!("Failed to build response parameters: {error:?}"),
                                );
                                send_status(&mut node, "error")?;
                                continue;
                            }
                        };

                        if streaming_enabled {
                            send_log(
                                &mut node,
                                LogLevel::Debug,
                                configured_log_level,
                                "Using streaming mode",
                            );

                            match timeout(
                                status_timeout,
                                client.responses().create_stream(parameters),
                            )
                            .await
                            {
                                Ok(Ok(mut stream)) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Debug,
                                        configured_log_level,
                                        "Stream created successfully, starting event loop",
                                    );
                                    let mut segmenter = StreamSegmenter::new(10);
                                    let mut has_sent_segment = false;
                                    let mut segment_index: u32 = 0;
                                    let mut final_text = String::new();
                                    let mut chunk_count = 0u32;
                                    let mut segment_count = 0u32;
                                    let mut stream_error: Option<String> = None;
                                    let mut tool_states: HashMap<String, ToolCallState> =
                                        HashMap::new();
                                    let mut item_to_call: HashMap<String, String> = HashMap::new();
                                    let mut tool_calls_ready: Vec<ToolCallPayload> = Vec::new();

                                    while let Some(event_result) = stream.next().await {
                                        match event_result {
                                            Ok(ResponseStreamEvent::ResponseOutputTextDelta { delta, .. }) => {
                                                chunk_count += 1;
                                                final_text.push_str(&delta);
                                                send_log(
                                                    &mut node,
                                                    LogLevel::Debug,
                                                    configured_log_level,
                                                    &format!("Received text delta: {} chars", delta.len()),
                                                );

                                                if let Some(segment) = segmenter.add_chunk(&delta) {
                                                    let status_str = if has_sent_segment {
                                                        "ongoing"
                                                    } else {
                                                        "started"
                                                    };
                                                    send_segment(
                                                        &mut node,
                                                        &metadata_parameters,
                                                        status_str,
                                                        Some(segment_index),
                                                        &segment,
                                                    )?;
                                                    has_sent_segment = true;
                                                    segment_index += 1;
                                                    segment_count += 1;
                                                }
                                            }
                                            Ok(ResponseStreamEvent::ResponseOutputItemAdded { item, .. }) => {
                                                if support_tools {
                                                    if let ResponseOutput::FunctionToolCall(tool) = item {
                                                        let call_key = tool.call_id.clone();
                                                        let state = tool_states.entry(call_key.clone()).or_default();
                                                        state.call_id = Some(call_key.clone());
                                                        if !tool.id.is_empty() {
                                                            state.id = Some(tool.id.clone());
                                                            item_to_call.insert(tool.id.clone(), call_key.clone());
                                                        }
                                                        if !tool.name.is_empty() {
                                                            state.name = Some(tool.name.clone());
                                                        }
                                                        if !tool.arguments.is_empty() {
                                                            state.arguments_buffer = tool.arguments.clone();
                                                            state.arguments = Some(tool.arguments.clone());
                                                        }
                                                        collect_new_tool_calls(
                                                            &mut node,
                                                            &mut tool_states,
                                                            &mut tool_calls_ready,
                                                        )?;
                                                    }
                                                }
                                            }
                                            Ok(ResponseStreamEvent::ResponseOutputItemDone { item, .. }) => {
                                                if support_tools {
                                                    if let ResponseOutput::FunctionToolCall(tool) = item {
                                                        let call_key = tool
                                                            .call_id
                                                            .clone();
                                                        let state = tool_states.entry(call_key.clone()).or_default();
                                                        state.call_id = Some(call_key.clone());
                                                        if !tool.id.is_empty() {
                                                            state.id = Some(tool.id.clone());
                                                            item_to_call.insert(tool.id.clone(), call_key.clone());
                                                        }
                                                        if !tool.name.is_empty() {
                                                            state.name = Some(tool.name.clone());
                                                        }
                                                        if !tool.arguments.is_empty() {
                                                            state.arguments_buffer = tool.arguments.clone();
                                                            state.arguments = Some(tool.arguments.clone());
                                                        }
                                                        collect_new_tool_calls(
                                                            &mut node,
                                                            &mut tool_states,
                                                            &mut tool_calls_ready,
                                                        )?;
                                                    }
                                                }
                                            }
                                            Ok(ResponseStreamEvent::ResponseOutputTextDone { text, .. }) => {
                                                final_text = text.clone();
                                                if let Some(segment) = segmenter.flush() {
                                                    if !segment.trim().is_empty() {
                                                        let status_str = if has_sent_segment {
                                                            "ongoing"
                                                        } else {
                                                            "started"
                                                        };
                                                        send_segment(
                                                            &mut node,
                                                            &metadata_parameters,
                                                            status_str,
                                                            Some(segment_index),
                                                            &segment,
                                                        )?;
                                                        has_sent_segment = true;
                                                        segment_index += 1;
                                                        segment_count += 1;
                                                    }
                                                }
                                            }
                                            Ok(ResponseStreamEvent::ResponseFunctionCallArgumentsDelta { item_id, delta, .. }) => {
                                                if support_tools {
                                                    let call_key = resolve_call_key(&item_id, &mut item_to_call);
                                                    let state = tool_states.entry(call_key.clone()).or_default();
                                                    if state.id.is_none() {
                                                        state.id = Some(item_id.clone());
                                                    }
                                                    state.arguments_buffer.push_str(&delta);
                                                    state.arguments = Some(state.arguments_buffer.clone());
                                                }
                                            }
                                            Ok(ResponseStreamEvent::ResponseFunctionCallArgumentsDone { item_id, name, arguments, .. }) => {
                                                if support_tools {
                                                    let call_key = resolve_call_key(&item_id, &mut item_to_call);
                                                    let state = tool_states.entry(call_key.clone()).or_default();
                                                    if state.id.is_none() {
                                                        state.id = Some(item_id.clone());
                                                    }
                                                    state.name = Some(name);
                                                    state.arguments_buffer = arguments.clone();
                                                    state.arguments = Some(arguments);
                                                    collect_new_tool_calls(
                                                        &mut node,
                                                        &mut tool_states,
                                                        &mut tool_calls_ready,
                                                    )?;
                                                }
                                            }
                                            Ok(ResponseStreamEvent::ResponseCompleted { response, .. }) => {
                                                send_log(
                                                    &mut node,
                                                    LogLevel::Debug,
                                                    configured_log_level,
                                                    "Received ResponseCompleted event, ending stream loop",
                                                );
                                                if support_tools {
                                                    collect_tool_calls_from_object(
                                                        &response,
                                                        &mut tool_states,
                                                        &mut item_to_call,
                                                    );
                                                }
                                                if final_text.is_empty() {
                                                    if let Some(text) = extract_text_output(&response) {
                                                        final_text = text;
                                                    }
                                                }
                                                if support_tools {
                                                    collect_new_tool_calls(
                                                        &mut node,
                                                        &mut tool_states,
                                                        &mut tool_calls_ready,
                                                    )?;
                                                }
                                                // Break out of the loop - response is complete
                                                break;
                                            }
                                            Ok(ResponseStreamEvent::Error { code, message, .. }) => {
                                                stream_error = Some(format!("{code}: {message}"));
                                                send_log(
                                                    &mut node,
                                                    LogLevel::Error,
                                                    configured_log_level,
                                                    &format!("Received Error event: {code}: {message}"),
                                                );
                                                break;
                                            }
                                            Ok(other_event) => {
                                                send_log(
                                                    &mut node,
                                                    LogLevel::Debug,
                                                    configured_log_level,
                                                    &format!("Received other event: {:?}", std::mem::discriminant(&other_event)),
                                                );
                                            }
                                            Err(err) => {
                                                if let APIError::StreamError(message) = &err {
                                                    if message == "Stream ended" {
                                                        send_log(
                                                            &mut node,
                                                            LogLevel::Info,
                                                            configured_log_level,
                                                            "Received end-of-stream signal from OpenAI",
                                                        );
                                                        break;
                                                    }
                                                }

                                                send_log(
                                                    &mut node,
                                                    LogLevel::Error,
                                                    configured_log_level,
                                                    &format!("Stream error details: {:#?}", err),
                                                );
                                                stream_error = Some(err.to_string());
                                                break;
                                            }
                                        }
                                    }

                                    send_log(
                                        &mut node,
                                        LogLevel::Debug,
                                        configured_log_level,
                                        &format!(
                                            "Stream loop ended. Error: {}, Chunks: {}, Final text length: {}",
                                            stream_error.as_deref().unwrap_or("none"),
                                            chunk_count,
                                            final_text.len()
                                        ),
                                    );

                                    if let Some(err_msg) = stream_error {
                                        send_log(
                                            &mut node,
                                            LogLevel::Error,
                                            configured_log_level,
                                            &format!("Streaming error: {}", err_msg),
                                        );
                                        send_status(&mut node, &format!("error: {}", err_msg))?;
                                        send_segment(
                                            &mut node,
                                            &metadata_parameters,
                                            "ended",
                                            None,
                                            &format!("Error: {}", err_msg),
                                        )?;
                                        continue;
                                    }

                                    if support_tools {
                                        collect_new_tool_calls(
                                            &mut node,
                                            &mut tool_states,
                                            &mut tool_calls_ready,
                                        )?;
                                    }

                                    if let Some(remaining) = segmenter.flush() {
                                        if !remaining.trim().is_empty() {
                                            let status_str = if has_sent_segment {
                                                "ongoing"
                                            } else {
                                                "started"
                                            };
                                            send_segment(
                                                &mut node,
                                                &metadata_parameters,
                                                status_str,
                                                Some(segment_index),
                                                &remaining,
                                            )?;
                                            has_sent_segment = true;
                                            segment_index += 1;
                                            segment_count += 1;
                                        }
                                    }

                                    if has_sent_segment {
                                        send_segment(
                                            &mut node,
                                            &metadata_parameters,
                                            "ended",
                                            Some(segment_index),
                                            "",
                                        )?;
                                    } else if !final_text.trim().is_empty() {
                                        send_segment(
                                            &mut node,
                                            &metadata_parameters,
                                            "started",
                                            None,
                                            &final_text,
                                        )?;
                                        send_segment(
                                            &mut node,
                                            &metadata_parameters,
                                            "ended",
                                            None,
                                            "",
                                        )?;
                                    }

                                    if !final_text.trim().is_empty() {
                                        session.add_assistant(final_text.clone());
                                        session.trim_history(config.max_history_exchanges);
                                    }

                                    send_status(&mut node, "complete")?;
                                    send_log(
                                        &mut node,
                                        LogLevel::Info,
                                        configured_log_level,
                                        &format!(
                                            "Streaming complete: {} chars across {} segments ({} chunks)",
                                            final_text.len(),
                                            segment_count,
                                            chunk_count
                                        ),
                                    );
                                }
                                Ok(Err(error)) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Error,
                                        configured_log_level,
                                        &format!("OpenAI streaming error: {error}"),
                                    );
                                    send_status(&mut node, "error")?;
                                    send_segment(
                                        &mut node,
                                        &metadata_parameters,
                                        "ended",
                                        None,
                                        &format!("Error: {}", error),
                                    )?;
                                }
                                Err(_) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Error,
                                        configured_log_level,
                                        "OpenAI streaming request timed out",
                                    );
                                    send_status(&mut node, "timeout")?;
                                }
                            }
                        } else {
                            let responses = client.responses();
                            let api_call = responses.create(parameters);

                            match timeout(status_timeout, api_call).await {
                                Ok(Ok(response)) => {
                                    let tool_calls = tool_calls_from_response(&response);
                                    if !tool_calls.is_empty() {
                                        send_tool_calls(&mut node, &tool_calls)?;
                                    }

                                    if let Some(text) = extract_text_output(&response) {
                                        send_segment(
                                            &mut node,
                                            &metadata_parameters,
                                            "started",
                                            None,
                                            &text,
                                        )?;
                                        send_status(&mut node, "complete")?;
                                        send_segment(
                                            &mut node,
                                            &metadata_parameters,
                                            "ended",
                                            None,
                                            "",
                                        )?;
                                        session.add_assistant(text);
                                        session.trim_history(config.max_history_exchanges);
                                    } else {
                                        send_log(
                                            &mut node,
                                            LogLevel::Warn,
                                            configured_log_level,
                                            "OpenAI response did not contain assistant text output",
                                        );
                                        send_status(&mut node, "empty")?;
                                    }
                                }
                                Ok(Err(error)) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Error,
                                        configured_log_level,
                                        &format!("OpenAI API error: {error}"),
                                    );
                                    send_status(&mut node, "error")?;
                                }
                                Err(_) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Error,
                                        configured_log_level,
                                        "OpenAI API request timed out",
                                    );
                                    send_status(&mut node, "timeout")?;
                                }
                            }
                        }
                    }
                    "control" => {
                        let control_text = data
                            .as_string::<i32>()
                            .iter()
                            .filter_map(|value| value.map(str::to_string))
                            .collect::<Vec<String>>()
                            .join(" ");

                        // Try to parse as JSON first, fall back to plain text
                        let parsed = serde_json::from_str::<serde_json::Value>(&control_text)
                            .ok()
                            .and_then(|v| {
                                if v.is_object() {
                                    Some(v)
                                } else {
                                    None
                                }
                            });

                        let mut should_reset = false;
                        let mut prompt_text: Option<String> = None;

                        if let Some(json) = parsed {
                            // Handle JSON control input
                            if let Some(command) = json.get("command").and_then(|v| v.as_str()) {
                                if command.eq_ignore_ascii_case("reset") {
                                    should_reset = true;
                                }
                            }
                            if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
                                if !prompt.trim().is_empty() {
                                    prompt_text = Some(prompt.to_string());
                                }
                            }
                        } else if control_text.eq_ignore_ascii_case("reset") {
                            // Backward compatibility: plain text "reset"
                            should_reset = true;
                        }

                        // Handle reset command
                        if should_reset {
                            sessions.remove(&session_id);
                            send_log(
                                &mut node,
                                LogLevel::Info,
                                configured_log_level,
                                &format!("Reset session '{session_id}'"),
                            );
                            send_status(&mut node, "reset")?;
                        }

                        // Handle prompt field - send to API
                        if let Some(ref user_text) = prompt_text {
                            let session = sessions
                                .entry(session_id.clone())
                                .or_insert_with(SessionState::new);

                            session.add_user(user_text.clone());
                            session.trim_history(config.max_history_exchanges);

                            send_log(
                                &mut node,
                                LogLevel::Info,
                                configured_log_level,
                                &format!("Sending prompt from control to OpenAI: {}", user_text),
                            );

                            send_status(&mut node, "processing")?;

                            let mut builder = ResponseParametersBuilder::default();
                            builder.model(model_name.clone());
                            builder.input(ResponseInput::List(session.messages.clone()));
                            if let Some(prompt) = &config.system_prompt {
                                builder.instructions(prompt.clone());
                            }
                            if streaming_enabled {
                                builder.stream(true);
                            }

                            let parameters = match builder.build() {
                                Ok(parameters) => parameters,
                                Err(error) => {
                                    send_log(
                                        &mut node,
                                        LogLevel::Error,
                                        configured_log_level,
                                        &format!("Failed to build response parameters: {error:?}"),
                                    );
                                    send_status(&mut node, "error")?;
                                    continue;
                                }
                            };

                            if streaming_enabled {
                                send_log(
                                    &mut node,
                                    LogLevel::Debug,
                                    configured_log_level,
                                    "Using streaming mode for control prompt",
                                );

                                match timeout(
                                    status_timeout,
                                    client.responses().create_stream(parameters),
                                )
                                .await
                                {
                                    Ok(Ok(mut stream)) => {
                                        send_log(
                                            &mut node,
                                            LogLevel::Debug,
                                            configured_log_level,
                                            "Stream created successfully, starting event loop",
                                        );
                                        let mut segmenter = StreamSegmenter::new(10);
                                        let mut has_sent_segment = false;
                                        let mut segment_index: u32 = 0;
                                        let mut final_text = String::new();
                                        let mut chunk_count = 0u32;
                                        let mut segment_count = 0u32;
                                        let mut stream_error: Option<String> = None;
                                        let mut tool_states: HashMap<String, ToolCallState> =
                                            HashMap::new();
                                        let mut item_to_call: HashMap<String, String> = HashMap::new();
                                        let mut tool_calls_ready: Vec<ToolCallPayload> = Vec::new();

                                        while let Some(event_result) = stream.next().await {
                                            match event_result {
                                                Ok(ResponseStreamEvent::ResponseOutputTextDelta { delta, .. }) => {
                                                    chunk_count += 1;
                                                    if let Some(segment) = segmenter.add_chunk(&delta) {
                                                        if !segment.trim().is_empty() {
                                                            let status_str = if has_sent_segment {
                                                                "ongoing"
                                                            } else {
                                                                "started"
                                                            };
                                                            send_segment(
                                                                &mut node,
                                                                &metadata_parameters,
                                                                status_str,
                                                                Some(segment_index),
                                                                &segment,
                                                            )?;
                                                            has_sent_segment = true;
                                                            segment_index += 1;
                                                            segment_count += 1;
                                                        }
                                                    }
                                                }
                                                Ok(ResponseStreamEvent::ResponseOutputItemDone { item, .. }) => {
                                                    if support_tools {
                                                        if let ResponseOutput::FunctionToolCall(tool) = item {
                                                            let call_key = tool
                                                                .call_id
                                                                .clone();
                                                            let state = tool_states.entry(call_key.clone()).or_default();
                                                            state.call_id = Some(call_key.clone());
                                                            if !tool.id.is_empty() {
                                                                state.id = Some(tool.id.clone());
                                                                item_to_call.insert(tool.id.clone(), call_key.clone());
                                                            }
                                                            if !tool.name.is_empty() {
                                                                state.name = Some(tool.name.clone());
                                                            }
                                                            if !tool.arguments.is_empty() {
                                                                state.arguments_buffer = tool.arguments.clone();
                                                                state.arguments = Some(tool.arguments.clone());
                                                            }
                                                            collect_new_tool_calls(
                                                                &mut node,
                                                                &mut tool_states,
                                                                &mut tool_calls_ready,
                                                            )?;
                                                        }
                                                    }
                                                }
                                                Ok(ResponseStreamEvent::ResponseOutputTextDone { text, .. }) => {
                                                    final_text = text.clone();
                                                    if let Some(segment) = segmenter.flush() {
                                                        if !segment.trim().is_empty() {
                                                            let status_str = if has_sent_segment {
                                                                "ongoing"
                                                            } else {
                                                                "started"
                                                            };
                                                            send_segment(
                                                                &mut node,
                                                                &metadata_parameters,
                                                                status_str,
                                                                Some(segment_index),
                                                                &segment,
                                                            )?;
                                                            has_sent_segment = true;
                                                            segment_index += 1;
                                                            segment_count += 1;
                                                        }
                                                    }
                                                }
                                                Ok(ResponseStreamEvent::ResponseFunctionCallArgumentsDelta { item_id, delta, .. }) => {
                                                    if support_tools {
                                                        let call_key = resolve_call_key(&item_id, &mut item_to_call);
                                                        let state = tool_states.entry(call_key.clone()).or_default();
                                                        if state.id.is_none() {
                                                            state.id = Some(item_id.clone());
                                                        }
                                                        state.arguments_buffer.push_str(&delta);
                                                        state.arguments = Some(state.arguments_buffer.clone());
                                                    }
                                                }
                                                Ok(ResponseStreamEvent::ResponseFunctionCallArgumentsDone { item_id, name, arguments, .. }) => {
                                                    if support_tools {
                                                        let call_key = resolve_call_key(&item_id, &mut item_to_call);
                                                        let state = tool_states.entry(call_key.clone()).or_default();
                                                        if state.id.is_none() {
                                                            state.id = Some(item_id.clone());
                                                        }
                                                        state.name = Some(name);
                                                        state.arguments_buffer = arguments.clone();
                                                        state.arguments = Some(arguments);
                                                        collect_new_tool_calls(
                                                            &mut node,
                                                            &mut tool_states,
                                                            &mut tool_calls_ready,
                                                        )?;
                                                    }
                                                }
                                                Ok(ResponseStreamEvent::ResponseCompleted { response, .. }) => {
                                                    send_log(
                                                        &mut node,
                                                        LogLevel::Debug,
                                                        configured_log_level,
                                                        "Received ResponseCompleted event, ending stream loop",
                                                    );
                                                    if support_tools {
                                                        collect_tool_calls_from_object(
                                                            &response,
                                                            &mut tool_states,
                                                            &mut item_to_call,
                                                        );
                                                    }
                                                    if final_text.is_empty() {
                                                        if let Some(text) = extract_text_output(&response) {
                                                            final_text = text;
                                                        }
                                                    }
                                                    if support_tools {
                                                        collect_new_tool_calls(
                                                            &mut node,
                                                            &mut tool_states,
                                                            &mut tool_calls_ready,
                                                        )?;
                                                    }
                                                    break;
                                                }
                                                Ok(ResponseStreamEvent::Error { code, message, .. }) => {
                                                    stream_error = Some(format!("{code}: {message}"));
                                                    send_log(
                                                        &mut node,
                                                        LogLevel::Error,
                                                        configured_log_level,
                                                        &format!("Received Error event: {code}: {message}"),
                                                    );
                                                    break;
                                                }
                                                Ok(_) => {
                                                    // Other events
                                                }
                                                Err(err) => {
                                                    if let APIError::StreamError(message) = &err {
                                                        if message == "Stream ended" {
                                                            send_log(
                                                                &mut node,
                                                                LogLevel::Info,
                                                                configured_log_level,
                                                                "Received end-of-stream signal from OpenAI",
                                                            );
                                                            break;
                                                        }
                                                    }

                                                    send_log(
                                                        &mut node,
                                                        LogLevel::Error,
                                                        configured_log_level,
                                                        &format!("Stream error details: {:#?}", err),
                                                    );
                                                    stream_error = Some(err.to_string());
                                                    break;
                                                }
                                            }
                                        }

                                        send_log(
                                            &mut node,
                                            LogLevel::Debug,
                                            configured_log_level,
                                            &format!(
                                                "Stream loop ended. Error: {}, Chunks: {}, Final text length: {}",
                                                stream_error.as_deref().unwrap_or("none"),
                                                chunk_count,
                                                final_text.len()
                                            ),
                                        );

                                        if let Some(err) = stream_error {
                                            send_log(
                                                &mut node,
                                                LogLevel::Error,
                                                configured_log_level,
                                                &format!("Streaming error: {err}"),
                                            );
                                            send_status(&mut node, "error")?;
                                        } else {
                                            if has_sent_segment {
                                                send_segment(
                                                    &mut node,
                                                    &metadata_parameters,
                                                    "ended",
                                                    Some(segment_index),
                                                    "",
                                                )?;
                                            }

                                            if support_tools && !tool_calls_ready.is_empty() {
                                                send_tool_calls(&mut node, &tool_calls_ready)?;
                                            }

                                            send_log(
                                                &mut node,
                                                LogLevel::Debug,
                                                configured_log_level,
                                                &format!(
                                                    "Stream completed successfully. Segments: {}, Final text length: {}",
                                                    segment_count, final_text.len()
                                                ),
                                            );

                                            if !final_text.is_empty() {
                                                session.add_assistant(final_text);
                                                session.trim_history(config.max_history_exchanges);
                                            } else if !tool_calls_ready.is_empty() {
                                                // Tool calls present, no text
                                            } else {
                                                send_log(
                                                    &mut node,
                                                    LogLevel::Warn,
                                                    configured_log_level,
                                                    "OpenAI response did not contain assistant text output",
                                                );
                                                send_status(&mut node, "empty")?;
                                            }
                                        }
                                    }
                                    Ok(Err(error)) => {
                                        send_log(
                                            &mut node,
                                            LogLevel::Error,
                                            configured_log_level,
                                            &format!("OpenAI API error: {error}"),
                                        );
                                        send_status(&mut node, "error")?;
                                    }
                                    Err(_) => {
                                        send_log(
                                            &mut node,
                                            LogLevel::Error,
                                            configured_log_level,
                                            "OpenAI API request timed out",
                                        );
                                        send_status(&mut node, "timeout")?;
                                    }
                                }
                            } else {
                                // Non-streaming mode
                                match timeout(
                                    status_timeout,
                                    client.responses().create(parameters),
                                )
                                .await
                                {
                                    Ok(Ok(response)) => {
                                        if let Some(text) = extract_text_output(&response) {
                                            send_segment(&mut node, &metadata_parameters, "complete", None, &text)?;
                                            session.add_assistant(text);
                                            session.trim_history(config.max_history_exchanges);
                                        } else {
                                            send_log(
                                                &mut node,
                                                LogLevel::Warn,
                                                configured_log_level,
                                                "OpenAI response did not contain assistant text output",
                                            );
                                            send_status(&mut node, "empty")?;
                                        }
                                    }
                                    Ok(Err(error)) => {
                                        send_log(
                                            &mut node,
                                            LogLevel::Error,
                                            configured_log_level,
                                            &format!("OpenAI API error: {error}"),
                                        );
                                        send_status(&mut node, "error")?;
                                    }
                                    Err(_) => {
                                        send_log(
                                            &mut node,
                                            LogLevel::Error,
                                            configured_log_level,
                                            "OpenAI API request timed out",
                                        );
                                        send_status(&mut node, "timeout")?;
                                    }
                                }
                            }
                        }

                        // If neither reset nor prompt, log warning
                        if !should_reset && prompt_text.is_none() {
                            send_log(
                                &mut node,
                                LogLevel::Warn,
                                configured_log_level,
                                &format!(
                                    "Received control input without valid command or prompt: '{control_text}'",
                                ),
                            );
                        }
                    }
                    "tool_results" => {
                        send_log(
                            &mut node,
                            LogLevel::Warn,
                            configured_log_level,
                            "tool_results input not supported by openai-response-client",
                        );
                    }
                    other => {
                        send_log(
                            &mut node,
                            LogLevel::Warn,
                            configured_log_level,
                            &format!("Received unsupported input '{other}'"),
                        );
                    }
                }
            }
            Event::Stop(_) => {
                send_log(
                    &mut node,
                    LogLevel::Info,
                    configured_log_level,
                    "Received stop event, shutting down",
                );
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_dive::v1::resources::response::response::ResponseStreamEvent;

    fn collect_stream_text(events: &[ResponseStreamEvent]) -> HashMap<String, String> {
        let mut buffers: HashMap<String, String> = HashMap::new();

        for event in events {
            match event {
                ResponseStreamEvent::ResponseOutputTextDelta { item_id, delta, .. } => {
                    buffers.entry(item_id.clone()).or_default().push_str(delta);
                }
                ResponseStreamEvent::ResponseOutputTextDone { item_id, text, .. } => {
                    buffers.insert(item_id.clone(), text.clone());
                }
                _ => {}
            }
        }

        buffers
    }

    #[test]
    fn streaming_deltas_accumulate() {
        let events = vec![
            ResponseStreamEvent::ResponseOutputTextDelta {
                sequence_number: 1,
                item_id: "msg_1".into(),
                output_index: 0,
                content_index: 0,
                delta: "Hello".into(),
                logprobs: None,
            },
            ResponseStreamEvent::ResponseOutputTextDelta {
                sequence_number: 2,
                item_id: "msg_1".into(),
                output_index: 0,
                content_index: 0,
                delta: " world".into(),
                logprobs: None,
            },
            ResponseStreamEvent::ResponseOutputTextDone {
                sequence_number: 3,
                item_id: "msg_1".into(),
                output_index: 0,
                content_index: 0,
                text: "Hello world".into(),
                logprobs: None,
            },
        ];

        let buffers = collect_stream_text(&events);
        assert_eq!(buffers.get("msg_1"), Some(&"Hello world".to_string()));
    }

    #[test]
    fn streaming_multiple_items_separate() {
        let events = vec![
            ResponseStreamEvent::ResponseOutputTextDelta {
                sequence_number: 1,
                item_id: "msg_1".into(),
                output_index: 0,
                content_index: 0,
                delta: "Hello".into(),
                logprobs: None,
            },
            ResponseStreamEvent::ResponseOutputTextDelta {
                sequence_number: 2,
                item_id: "msg_2".into(),
                output_index: 1,
                content_index: 0,
                delta: "Bonjour".into(),
                logprobs: None,
            },
            ResponseStreamEvent::ResponseOutputTextDelta {
                sequence_number: 3,
                item_id: "msg_1".into(),
                output_index: 0,
                content_index: 0,
                delta: " there".into(),
                logprobs: None,
            },
            ResponseStreamEvent::ResponseOutputTextDone {
                sequence_number: 4,
                item_id: "msg_2".into(),
                output_index: 1,
                content_index: 0,
                text: "Bonjour".into(),
                logprobs: None,
            },
        ];

        let buffers = collect_stream_text(&events);
        assert_eq!(buffers.get("msg_1"), Some(&"Hello there".to_string()));
        assert_eq!(buffers.get("msg_2"), Some(&"Bonjour".to_string()));
    }
}
