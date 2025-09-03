# OpenAI WebSocket Browser Example (Docker)

This example runs a voice chat pipeline via a WebSocket server inside Docker. It dynamically spawns a Dora dataflow from a template and routes audio + text between the client and the nodes.

## Architecture

The system uses a WebSocket server (`dora-openai-websocket`) that:
1. Accepts connections from clients (like Moly)
2. Dynamically spawns a Dora dataflow based on the template
3. Routes audio and text between the client and the dataflow

## Quick Start (Ubuntu 24.04)

1) Install Docker + Compose
- sudo apt-get update && sudo apt-get install -y ca-certificates curl gnupg lsb-release
- Install Docker Engine: https://docs.docker.com/engine/install/ubuntu/
- Install Compose plugin: https://docs.docker.com/compose/install/linux/
- Optional: add your user to docker group and re-login: sudo usermod -aG docker $USER

2) Clone, build image
- git clone https://github.com/dora-rs/dora.git && cd dora
- docker build -t dora-voicechat:latest -f docker/Dockerfile .

3) Configure environment
- Create repo root .env with your key:
  OPENAI_API_KEY=sk-...
- Export absolute host paths (no ~):
  export MODELS_DIR="$(realpath ~/.dora/models)"
  export HF_CACHE="$(realpath ~/.cache/huggingface)"
  export REPO_DIR="$(pwd)"

4) Download models (outside the container)
- If you already have models under $MODELS_DIR, skip.
- Or run the helper to populate host dirs:
  bash docker/download-models.sh all

5) Start the server
- docker compose -f docker/docker-compose.yml up -d server
- Logs: docker compose -f docker/docker-compose.yml logs -f --tail=200 server

### Quick Test After Build
- Ensure env vars are exported (same shell):
  - export MODELS_DIR="$(realpath ~/.dora/models)"
  - export HF_CACHE="$(realpath ~/.cache/huggingface)"
  - export REPO_DIR="$(pwd)"
- Run end-to-end ASR + TTS tests via compose:
  - bash docker/compose-tests.sh
  - Choose a different TTS voice:
    - bash docker/compose-tests.sh --voice maple

## Configuration

The system uses `whisper-template-metal.yml` as a template. When a client connects, the WebSocket server:
1. Receives a session configuration from the client
2. Replaces `NODE_ID` in the template with a unique server ID (e.g., `server-12345`)
3. Creates a new dataflow YAML file (e.g., `whisper-12345.yml`)
4. Starts the dataflow automatically

### MaaS Configuration
The MaaS client is configured via `maas_mcp_browser_config.toml`. Prefer storing keys as env references in the file:
`api_key = "env:OPENAI_API_KEY"` (OPENAI_API_KEY is picked from ../.env by compose).

## Running the System

### Step 1: Start the Mock Weather Server (Optional)

If you want to test browser automation features:
```bash
python mock_weather_server.py
```

### Run ASR + TTS validation (from host)
- ASR:
```
docker compose -f docker/docker-compose.yml exec \
  -e ASR_MODELS_DIR=/root/.dora/models/asr \
  server bash -lc 'cd /opt/dora/examples/setup-new-chatbot/asr-validation && python test_basic_asr.py'
```
- TTS:
```
docker compose -f docker/docker-compose.yml exec \
  -e PRIMESPEECH_MODEL_DIR=/root/.dora/models/primespeech \
  server bash -lc 'cd /opt/dora/examples/setup-new-chatbot/primespeech-validation && python test_tts_direct.py --device cpu --voice doubao'
```
Alternatively:
- `bash docker/compose-tests.sh` (uses compose `server` container)
- `bash docker/run-tests.sh` (runs tests in a one-off container)

### Step 3: Connect with Moly Client

1. Open the Moly client application
2. Configure it to connect to: `ws://localhost:8123` (or `ws://0.0.0.0:8123`)
3. Start a conversation

When Moly connects:
- It sends a `session.update` message with configuration
- The server creates a new dataflow instance
- Moly can send an initial greeting via `response.create` 
- The greeting is routed to the MaaS client which generates a response

## Data Flow

1. **Client → Server**: 
   - Audio data via `input_audio_buffer.append` 
   - Text/greetings via `response.create`

2. **Server → Dataflow**:
   - Audio → Speech Monitor → ASR → MaaS Client
   - Greetings → MaaS Client (via `text_to_audio` input)

3. **Dataflow → Server → Client**:
   - MaaS response → Text Segmenter → PrimeSpeech TTS → Audio output

## Troubleshooting

### "Connection reset without closing" error
- Check that the WebSocket server is running
- Verify the template file exists: `whisper-template-metal.yml`
- Check server logs for any parsing errors

### No audio output
- Verify PrimeSpeech models are installed
- Check ASR is receiving audio (view logs)
- Ensure audio sample rates are correct (24kHz input, 24kHz output)

### MaaS not responding
- Check `maas_mcp_browser_config.toml` has valid API keys
- Verify network connectivity to cloud providers
- Check MaaS client logs for errors

## Jupyter Notebook (in-container editing)
- Optional notebook service is included in compose.
- Set auth (choose one) in .env:
  - `JUPYTER_PASSWORD=change-me` (recommended, disables token)
  - OR `JUPYTER_TOKEN=dora`
- Start: `docker compose -f docker/docker-compose.yml up -d notebook`
- Open: `http://<server-ip>:${JUPYTER_PORT:-8888}`
- Files under `/opt/dora` (mounted from `${REPO_DIR}`) — edits to templates (e.g., `whisper-template.yml`) take effect immediately.

## Notes

- Each client connection creates a separate dataflow instance
- The `NODE_ID` placeholder in the template is replaced with a unique ID
- Generated dataflow files (whisper-*.yml) can be deleted after use
- The system supports multiple concurrent connections
