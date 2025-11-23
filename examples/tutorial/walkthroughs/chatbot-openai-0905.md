# Walkthrough: chatbot-openai-0905

This tutorial covers the reference chat bot that streams ASR transcripts to OpenAI and renders replies through configurable TTS backends.

## 1. Configure Credentials
- Export `OPENAI_API_KEY` (and `OPENAI_BASE_URL` if using Azure or a proxy).
- Fill `config/openai.toml` with the desired model (`gpt-4o-mini`, `gpt-4o-realtime`, etc.) and set `stream = true` for duplex output.

## 2. Review Nodes & Routing
- The dataflow (`examples/chatbot-openai-0905/dataflow.yml`) chains audio input → VAD/segmenter → ASR → LLM → TTS.
- Inspect `node-hub` dependencies: Whisper default ASR, `node-hub/tts/piper` for speech output, and optional `node-hub/utils/logger` for tracing.

## 3. Run & Validate
- Lint and format before launch: `cargo fmt --manifest-path binaries/chatbot-openai-0905/Cargo.toml` and `uv run ruff check chatbot-openai-0905`.
- Start the flow: `dora start --dataflow dataflow.yml --detach`; connect a browser or CLI client from `chatbot-openai-websocket-browser` to observe streaming responses.

## 4. Extend the Scenario
- Integrate Alibaba Cloud or MiniMax by swapping the LLM node and updating environment variables; reference `requirements/cloud-models.md`.
- Add MCP integrations to enable tool usage during the conversation, following guidance in `requirements/mcp-tools.md`.
- Document observed latency, token usage, and transcript accuracy in the Lessons Learned section at the end of this tutorial file.
