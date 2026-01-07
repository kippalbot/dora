//! dora-local-llm: Multi-tenant LLM HTTP server with MLX inference
//!
//! Provides OpenAI-compatible `/v1/chat/completions` API backed by local MLX inference
//! with prefix-based KV caching for efficient multi-tenant serving.

use std::sync::Arc;

use dora_node_api::{
    merged::{MergeExternalSend, MergedEvent},
    DoraNode, Event,
};
use eyre::Context;
use outfox_openai::spec::{
    ChatChoice, ChatCompletionResponseMessage, CompletionUsage, CreateChatCompletionRequest,
    CreateChatCompletionResponse, FinishReason, Role,
};
use salvo::cors::*;
use salvo::prelude::*;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;

mod inference;
mod prefix_cache;

use inference::Inference;
use prefix_cache::PrefixCache;

/// Application state shared across HTTP handlers
#[derive(Clone)]
struct AppState {
    inference: Arc<Inference>,
    cache: Arc<Mutex<PrefixCache>>,
}

/// Configuration from environment
struct Config {
    port: u16,
    model_path: String,
    max_cache_entries: usize,
}

impl Config {
    fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8000),
            model_path: std::env::var("MLX_MODEL")
                .unwrap_or_else(|_| "mlx-community/Qwen2.5-7B-Instruct-4bit".to_string()),
            max_cache_entries: std::env::var("MAX_CACHE_ENTRIES")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(20),
        }
    }
}

/// Server events for communicating with Dora event loop
enum ServerEvent {
    Shutdown,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    tracing::info!("Starting dora-local-llm server on port {}", config.port);
    tracing::info!("Model: {}", config.model_path);
    tracing::info!("Max cache entries: {}", config.max_cache_entries);

    // Load model once at startup
    tracing::info!("Loading MLX model...");
    let inference = Arc::new(
        Inference::new(&config.model_path).context("Failed to load MLX model")?,
    );
    tracing::info!("Model loaded successfully");

    // Create prefix cache
    let cache = Arc::new(Mutex::new(PrefixCache::new(config.max_cache_entries)));

    let state = AppState { inference, cache };

    // Build Salvo router
    let router = Router::new()
        .hoop(affix_state::inject(state))
        .hoop(
            Cors::new()
                .allow_origin(AllowOrigin::any())
                .allow_methods(AllowMethods::any())
                .allow_headers(AllowHeaders::any())
                .into_handler(),
        )
        .push(Router::with_path("v1/chat/completions").post(chat_completions))
        .push(Router::with_path("v1/models").get(list_models))
        .push(Router::with_path("health").get(health))
        .push(Router::with_path("v1/cache/stats").get(cache_stats));

    let listen_addr = format!("0.0.0.0:{}", config.port);
    let acceptor = TcpListener::new(&listen_addr).bind().await;

    // Create channel for server events
    let (server_tx, server_rx) = mpsc::channel::<ServerEvent>(1);
    let server_events = ReceiverStream::new(server_rx);

    // Spawn HTTP server
    tokio::spawn(async move {
        Server::new(acceptor).serve(router).await;
        let _ = server_tx.send(ServerEvent::Shutdown).await;
    });

    tracing::info!("HTTP server listening on {}", listen_addr);

    // Initialize Dora node
    let (_node, events) = DoraNode::init_from_env()?;
    tracing::info!("Dora node initialized");

    // Merge Dora events with server events
    let merged = events.merge_external_send(server_events);
    let events = futures::executor::block_on_stream(merged);

    // Event loop
    for event in events {
        match event {
            MergedEvent::External(ServerEvent::Shutdown) => {
                tracing::info!("Server shutdown, exiting");
                break;
            }
            MergedEvent::Dora(Event::Stop(_)) => {
                tracing::info!("Received stop event, shutting down");
                break;
            }
            MergedEvent::Dora(Event::Input { id, .. }) => {
                tracing::debug!("Received input on {} (ignored - inference via HTTP)", id);
            }
            _ => {}
        }
    }

    tracing::info!("dora-local-llm shutdown complete");
    Ok(())
}

/// POST /v1/chat/completions - OpenAI-compatible chat completions
#[handler]
async fn chat_completions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let state = depot
        .obtain::<AppState>()
        .map_err(|_| StatusError::internal_server_error())?;

    let request: CreateChatCompletionRequest = req.parse_json().await.map_err(|e| {
        tracing::error!("Failed to parse request: {}", e);
        StatusError::bad_request()
    })?;

    // Check if streaming is requested
    let stream = request.stream.unwrap_or(false);
    if stream {
        // TODO: Implement streaming support with proper Salvo SSE API
        tracing::warn!("Streaming not yet implemented, falling back to non-streaming");
    }

    // Non-streaming response
    let response = generate_response(state.inference.clone(), state.cache.clone(), request)
        .await
        .map_err(|e| {
            tracing::error!("Generation error: {}", e);
            StatusError::internal_server_error()
        })?;

    res.render(Json(response));
    Ok(())
}

/// Generate a complete (non-streaming) response
async fn generate_response(
    inference: Arc<Inference>,
    cache: Arc<Mutex<PrefixCache>>,
    request: CreateChatCompletionRequest,
) -> eyre::Result<CreateChatCompletionResponse> {
    let model = request.model.clone();
    let temperature = request.temperature.unwrap_or(0.7);
    // max_completion_tokens is the field name in outfox-openai
    let max_tokens = request.max_completion_tokens.unwrap_or(2048) as usize;

    // Generate text
    let (content, prompt_tokens, completion_tokens) = {
        let mut cache_guard = cache.lock().await;
        inference.generate(&request.messages, &mut cache_guard, temperature, max_tokens)?
    };

    Ok(CreateChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp() as u32,
        model,
        usage: Some(CompletionUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatCompletionResponseMessage {
                role: Role::Assistant,
                content: Some(content),
                tool_calls: None,
                audio: None,
                refusal: None,
            },
            finish_reason: Some(FinishReason::Stop),
            logprobs: None,
        }],
        service_tier: None,
        system_fingerprint: None,
    })
}

/// GET /v1/models - List available models
#[handler]
async fn list_models(depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let state = depot
        .obtain::<AppState>()
        .map_err(|_| StatusError::internal_server_error())?;

    let model_id = state.inference.model_id();

    let response = serde_json::json!({
        "object": "list",
        "data": [{
            "id": model_id,
            "object": "model",
            "created": chrono::Utc::now().timestamp(),
            "owned_by": "local"
        }]
    });

    res.render(Json(response));
    Ok(())
}

/// GET /health - Health check
#[handler]
async fn health(res: &mut Response) {
    res.render(Json(serde_json::json!({
        "status": "healthy",
        "service": "dora-local-llm"
    })));
}

/// GET /v1/cache/stats - Cache statistics (for debugging)
#[handler]
async fn cache_stats(depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let state = depot
        .obtain::<AppState>()
        .map_err(|_| StatusError::internal_server_error())?;

    let cache = state.cache.lock().await;
    let stats = cache.stats();

    res.render(Json(stats));
    Ok(())
}
