# openai-response-client

A Dora node that talks to the OpenAI `v1/responses` API using the `openai_dive` crate. It exposes the same `text`, `status`, and `log` ports as `dora-maas-client`, so you can drop it into existing chat dataflows that expect those outputs.

## Build

```bash
cargo build --release -p openai-response-client
```

The compiled binary is placed at `target/release/openai-response-client`.

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | _required_ | API key for the OpenAI account. |
| `OPENAI_RESPONSE_CONFIG_PATH` | `openai_response_config.toml` | Path to a TOML file with defaults (model, prompt, streaming, etc.). |
| `OPENAI_RESPONSE_MODEL` | `gpt-5` | Model identifier passed to the responses API (overrides `default_model`). |
| `SYSTEM_PROMPT` | _none_ | Optional system instructions inserted at the start of the conversation. |
| `MAX_HISTORY_MESSAGES` | `12` | Maximum number of exchanges kept in the rolling history. |
| `OPENAI_ENABLE_STREAMING` | `true` | Stream responses and emit segmented chunks identical to `dora-maas-client`. |
| `OPENAI_ENABLE_TOOLS` | `false` | Enable remote tool execution (matches `enable_tools`). |
| `OPENAI_ENABLE_LOCAL_MCP` | `false` | Enable the embedded MCP host (matches `enable_local_mcp`). |
| `LOG_LEVEL` | `INFO` | Log verbosity (`ERROR`, `WARN`, `INFO`, `DEBUG`). |
| `OPENAI_API_BASE` | `https://api.openai.com/v1` | Override the base URL (useful for proxies). |
| `OPENAI_STATUS_TIMEOUT_SECONDS` | `60` | Timeout applied to each API call. |

## Runtime Behaviour

- Maintains independent chat history per `session_id` metadata value (defaults to `default` if none is present).
- Emits status updates: `processing`, `complete`, `reset`, `empty`, `timeout`, or `error`.
- Streams response text with the same segmentation and `session_status` / `segment_index` metadata that `dora-maas-client` produces (falls back to a single message + end marker when streaming is disabled).
- Emits `tool_calls` output whenever the model requests a function call so existing tool-handling nodes keep working.
- Configuration is loaded from `OPENAI_RESPONSE_CONFIG_PATH` (TOML) and then overridden by the environment variables listed above. The file mirrors the MaaS client structure; a minimal example:

  ```toml
  default_model = "gpt-5"
  system_prompt = "You are a helpful AI assistant..."
  enable_streaming = true
  enable_tools = false
  enable_local_mcp = false

  [[providers]]
  id = "openai"
  kind = "openai"
  api_key = "env:OPENAI_API_KEY"
  api_url = "https://api.openai.com/v1"

  [[models]]
  id = "gpt-5"
  route = { provider = "openai", model = "gpt-5" }
  ```
- Resets a session when it receives a `reset` control message.

## Example

Use the `examples/llm-client/dataflow-openai-responses.yml` dataflow to try it with the Textual chat UI:

```bash
export OPENAI_API_KEY=sk-...
cargo build --release -p openai-response-client
cd examples/llm-client
dora start dataflow-openai-responses.yml
python chat_terminal.py
```

For a minimal validation of streaming and metadata, use `examples/llm-client/dataflow-openai-responses-minimal.yml`. Both the full and minimal flows look for an `openai_response_config.toml` alongside the dataflow; set `OPENAI_RESPONSE_CONFIG_PATH` to point elsewhere if needed.
