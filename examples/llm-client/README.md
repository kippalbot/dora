# LLM Client - Terminal Chat Interface

A beautiful terminal-based chat interface for interacting with LLMs, powered by Dora dataflow and Textual TUI framework.

## Features

✅ **Rich Terminal UI** - Beautiful chat interface using Textual with Markdown rendering
✅ **Real-time Streaming** - See LLM responses as they're generated
✅ **Multiple LLM Backends** - Support for both local (Qwen3) and cloud (OpenAI) models
✅ **Conversation History** - Smart history management
✅ **Keyboard Shortcuts** - Efficient navigation and control
✅ **Commands** - Built-in commands for chat management
✅ **Markdown Support** - Rich formatting for code blocks, lists, and more

## Architecture

```
┌──────────────────────────┐
│  Textual TUI             │
│  (chat_terminal.py)      │
│  - User input            │
│  - Message display       │
│  - Status updates        │
└──────────┬───────────────┘
           │
           ├─ text ──────────┐
           │                 ▼
           │          ┌─────────────┐
           │          │  dora-qwen3 │
           │          │  Local LLM  │
           │          └─────────────┘
           │                 │
           └─ llm_text ──────┘
              (streaming)
```

## Prerequisites

- **Python 3.10+**
- **Dora CLI** 0.3.12+
- **For macOS with Apple Silicon**: MLX support (automatic)
- **For CPU/CUDA**: GGUF backend support

## Installation

### 1. Install Dependencies

```bash
cd examples/llm-client
pip install -r requirements.txt
```

### 2. Install Dora Qwen3 Node

```bash
cd ../../node-hub/dora-qwen3
pip install -e .
```

### 3. Download Qwen3 Model

**For Apple Silicon (MLX)**:
```bash
cd ../../node-hub/dora-qwen3
python download_models.py
```

This will download the default Qwen3-8B-MLX-4bit model (~5GB).

**For CPU/CUDA (GGUF)**:
```bash
# Download from HuggingFace manually or use:
python download_models.py --backend gguf
```

## Usage

### Option 1: Local LLM (Qwen3)

**Quick Start**:
```bash
# 1. Start the dataflow (launches qwen3-llm node)
cd examples/llm-client
dora start dataflow.yml

# 2. In another terminal, launch the chat interface
python chat_terminal.py
```

### Option 2: OpenAI API (GPT Models)

**Prerequisites**:
- OpenAI API key (set `OPENAI_API_KEY` environment variable)
- Built maas-client binary:
  ```bash
  cd ../../node-hub/dora-maas-client
  cargo build --release
  ```

**Quick Start**:
```bash
# 1. Set your OpenAI API key
export OPENAI_API_KEY="your-api-key-here"

# 2. Start the dataflow (launches maas-client with OpenAI)
cd examples/llm-client
dora start dataflow-openai.yml

# 3. In another terminal, launch the chat interface
python chat_terminal.py
```

**Configuration**:
Edit `maas_config.toml` to change:
- Model (gpt-4o, gpt-4o-mini, gpt-3.5-turbo)
- System prompt
- History settings

The chat interface will appear with a welcome message. Type your question and press Enter!

### Option 3: OpenAI Responses API (openai-response-client)

Use this option to talk to OpenAI models via the `v1/responses` endpoint powered by the `openai-response-client` node.

**Prerequisites**:
- OpenAI API key (set `OPENAI_API_KEY`)
- Build the responses client:
  ```bash
  cd ../../
  cargo build --release -p openai-response-client
  ```

**Quick Start**:
```bash
# 1. Set credentials and optional overrides
export OPENAI_API_KEY="your-api-key-here"
# export OPENAI_RESPONSE_MODEL="gpt-5"              # Optional override
# export SYSTEM_PROMPT="You are a helpful assistant" # Optional

# 2. Launch the full chat dataflow with the new client
cd examples/llm-client
dora start dataflow-openai-responses.yml

# 3. In another terminal, launch the chat interface
python chat_terminal.py
```

The responses client keeps lightweight conversation state per session, honours the `SYSTEM_PROMPT`, and streams chunks with the same `session_status` / `segment_index` metadata as `maas-client` (toggle via `OPENAI_ENABLE_STREAMING`, on by default), so it can act as a drop-in replacement in existing dataflows.

**Note**: The client uses a local patched version of `openai_dive` (via `[patch.crates-io]` in the workspace `Cargo.toml`) with two critical fixes:
1. **Null usage field**: Makes the `usage` field optional (`Option<Usage>`) to handle OpenAI's null `usage` in the initial `response.created` event
2. **Stream completion**: Properly breaks out of the event loop after receiving `ResponseCompleted` instead of treating the natural stream end as an error

These fixes resolve the "invalid type: null, expected struct Usage" and "Stream ended" errors during streaming.

Configuration defaults live in `openai_response_config.toml`; point `OPENAI_RESPONSE_CONFIG_PATH` at a custom file (or edit the sample) to adjust the model, prompt, history window, streaming behaviour, and tool flags. The file mirrors the MaaS client structure (`default_model`, `enable_tools`, `enable_local_mcp`, `[[providers]]`, `[[models]]`, …). Classic environment variables still override individual settings at runtime.

For a quick CLI-only smoke test, try the minimal pipeline:

```bash
export OPENAI_API_KEY="your-api-key-here"
cd examples/llm-client
dora start dataflow-openai-responses-minimal.yml
```

Type prompts into the terminal window; streamed responses, status updates, and any tool-call payloads will print to stdout.

### Example Session

```
┌─────────────────────────────────────────┐
│ Qwen3 Chat (Local LLM)           [X]    │
├─────────────────────────────────────────┤
│                                          │
│ User                                     │
│ Hello, what can you do?                 │
│                                          │
│ Assistant                                │
│ I am Qwen3, a large language model. I   │
│ can help with various tasks like:       │
│ - Answering questions                    │
│ - Writing and coding                     │
│ - Analysis and research                  │
│ - Creative content generation            │
│ How can I assist you today?             │
│                                          │
│ User                                     │
│ Write a Python function to calculate    │
│ fibonacci numbers                        │
│                                          │
│ Assistant                                │
│ Here's a Python function...             │
│                                          │
├─────────────────────────────────────────┤
│ > Type your message...                   │
├─────────────────────────────────────────┤
│ Ctrl+C: Quit | Ctrl+L: Clear | F1: Help│
└─────────────────────────────────────────┘
```

## Commands

Type these commands in the chat input:

| Command | Description |
|---------|-------------|
| `/help` | Show help message |
| `/clear` | Clear chat history (UI only) |
| `/reset` | Reset conversation (clears LLM history) |
| `/quit` | Exit application |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Quit application |
| `Ctrl+L` | Clear chat display |
| `Ctrl+R` | Reset conversation |
| `Enter` | Send message |
| `↑` / `↓` | Scroll chat history |

## Configuration

Edit `dataflow.yml` to customize the LLM behavior:

### Model Selection

**For Apple Silicon (MLX)**:
```yaml
MLX_MODEL: "Qwen/Qwen3-8B-MLX-4bit"          # Balanced (4-bit, ~5GB)
#MLX_MODEL: "Qwen/Qwen3-32B-MLX-6bit"        # Large (6-bit, ~20GB)
#MLX_MODEL: "mlx-community/GLM-4.5-Air-3bit" # Efficient (3-bit, ~2GB)
```

**For CPU/CUDA (GGUF)**:
```yaml
GGUF_MODEL: "Qwen/Qwen3-8B-GGUF"
GGUF_MODEL_FILE: "Qwen3-8B-Q4_K_M.gguf"
```

### Generation Parameters

```yaml
MAX_TOKENS: 512          # Max response length (128-2048)
TEMPERATURE: 0.7         # Creativity (0.0 = deterministic, 1.0 = creative)
TOP_P: 0.9              # Nucleus sampling (0.1-1.0)
```

### History Management

```yaml
HISTORY_STRATEGY: "token_based"     # Options: "fixed", "token_based", "sliding_window"
MAX_HISTORY_TOKENS: "4000"          # Max tokens for history (token_based)
MAX_HISTORY_EXCHANGES: "20"         # Max Q&A pairs (fixed)
```

### System Prompt

Customize the AI's behavior:
```yaml
SYSTEM_PROMPT: "You are a helpful AI assistant. Provide clear, concise, and accurate responses."
```

## Troubleshooting

### Model Not Found

**Error**: `Model not found: Qwen/Qwen3-8B-MLX-4bit`

**Solution**:
```bash
cd ../../node-hub/dora-qwen3
python download_models.py
```

### MLX Not Available

**Error**: `MLX not available, falling back to GGUF`

**Solution**: This is normal on non-Apple Silicon. The GGUF backend will be used automatically.

### Out of Memory

**Error**: `CUDA out of memory` or similar

**Solution**: Use a smaller quantized model:
```yaml
MLX_MODEL: "mlx-community/GLM-4.5-Air-3bit"  # Only ~2GB
```

Or reduce context:
```yaml
MAX_HISTORY_TOKENS: "2000"  # Reduce from 4000
```

### Slow Response Times

**For MLX**: Already optimized for Apple Silicon

**For GGUF**: Ensure you're using a quantized model (Q4_K_M or Q5_K_M):
```yaml
GGUF_MODEL_FILE: "Qwen3-8B-Q4_K_M.gguf"  # Faster, less memory
```

### Chat Interface Not Connecting

**Error**: Messages don't appear or no streaming

**Solution**:
1. Check that dataflow is running: `dora list`
2. Verify qwen3-llm node is active
3. Check logs: `dora logs qwen3-llm`
4. Restart both dataflow and chat interface

### Terminal Display Issues

**Problem**: UI rendering issues or garbled text

**Solution**:
```bash
# Try with explicit terminal
TERM=xterm-256color python chat_terminal.py

# Or update terminal emulator (iTerm2, Alacritty recommended)
```

## Advanced Usage

### Running with Custom Dataflow

Create your own dataflow with additional nodes:

```yaml
nodes:
  - id: chat-terminal
    path: dynamic
    outputs: [text, control]
    inputs:
      llm_text: qwen3-llm/text
      llm_status: qwen3-llm/status

  # Your custom preprocessing node
  - id: preprocessor
    path: your_preprocessor.py
    inputs:
      text: chat-terminal/text
    outputs: [processed_text]

  - id: qwen3-llm
    path: dora-qwen3
    inputs:
      text: preprocessor/processed_text
    # ... rest of config
```

### Logging and Debugging

Enable debug logging:
```yaml
env:
  LOG_LEVEL: DEBUG  # Shows detailed LLM generation info
```

View logs:
```bash
dora logs qwen3-llm
```

### Performance Tuning

**For faster first response**:
```yaml
MAX_TOKENS: 256  # Reduce max response length
```

**For better quality**:
```yaml
TEMPERATURE: 0.7  # Balance between creative and accurate
TOP_P: 0.9        # Nucleus sampling
```

**For deterministic responses**:
```yaml
TEMPERATURE: 0.0  # No randomness
```

## File Structure

```
llm-client/
├── README.md              # This file
├── requirements.txt       # Python dependencies
├── dataflow.yml           # Dora dataflow for Qwen3 (local LLM)
├── dataflow-openai.yml    # Dora dataflow for OpenAI (cloud API)
├── maas_config.toml       # OpenAI/MaaS configuration
├── chat_terminal.py       # Main Textual UI + Dora node
├── viewer.py              # Log viewer (optional)
└── .gitignore             # Git ignore patterns
```

## Development

### Modifying the UI

The Textual UI is in `chat_terminal.py`. Key components:

- `ChatTerminalApp`: Main app class
- `ChatDisplay`: Scrollable message container
- `ChatMessage`: Individual message widget
- `dora_event_loop()`: Handles Dora events in background thread

### Adding Features

1. **Message history persistence**: Add file I/O to save/load chat history
2. **Multiple models**: Add model switching in UI
3. **Voice input**: Integrate with dora-asr node
4. **Markdown rendering**: Use Rich's Markdown support for code blocks

## Related Examples

- `examples/mac-aec-chat` - Voice chat with Qwen3
- `examples/voice-chatbot` - Full voice chatbot pipeline
- `node-hub/dora-qwen3` - Qwen3 LLM node documentation

## License

Same as Dora framework license.

## Support

- [Dora GitHub Issues](https://github.com/dora-rs/dora)
- [Textual Documentation](https://textual.textualize.io/)
- [Qwen3 Model Card](https://huggingface.co/Qwen/Qwen3-8B)
