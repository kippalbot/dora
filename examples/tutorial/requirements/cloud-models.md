# Cloud Model Integrations

When local inference is impractical, connect the conversation agent to managed LLM, ASR, or TTS services. Keep credentials outside the repository and follow the provider-specific instructions below.

## OpenAI
- Examples: `chatbot-openai-0905`, `openai-server`, `chatbot-openai-websocket-browser`.
- Set `OPENAI_API_KEY` and optional `OPENAI_BASE_URL` in your shell or `.env.local` consumed by the nodes.
- Configure streaming responses in `config/openai.toml` to minimize response latency for duplex conversations.

## Alibaba Cloud
- Refer to `chatbot-alicloud-0908` for a complete dataflow.
- Export `ALIBABA_CLOUD_ACCESS_KEY_ID` and `ALIBABA_CLOUD_ACCESS_KEY_SECRET`; the node reads additional parameters like region and model ID from `config/alicloud.toml`.
- Monitor usage quotas; retry logic is implemented in `node-hub/llm/alicloud/handler.rs` if burst requests are expected.

## MiniMax
- Supported via the `node-hub/llm/minimax` adapter.
- Store `MINIMAX_API_KEY` in your environment and update `model = dialog_release` (or desired variant) in the node configuration.
- For speech synthesis, combine MiniMax LLM output with local TTS nodes to keep outbound traffic manageable.

> Always run `uv run ruff check node-hub` after editing API clients to stay aligned with linting rules.
