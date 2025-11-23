# Develop Full Duplex Conversation Agent with DORA

Build interruption-ready voice agents by mixing the reusable DORA nodes featured in `examples/`. This series converts the mac AEC chat, OpenAI chatbot, and podcast generator demos into a cohesive learning path for contributors.

## Feature Highlights
- Hardware-assisted AEC + VAD via `mac_aec_simple_segmentation.py` keeps microphone input clean while tracking user intent.
- Unified ASR layer (`dora-asr`) switches between FunASR and Whisper automatically based on locale hints.
- Local LLM orchestration (`dora-qwen3`) and cloud bridges (`dora-maas-client`) let you test both OminiX and OpenAI-compatible APIs.
- Dual TTS options—`dora-primespeech` (Chinese, voice clone) and `dora-kokoro-tts` (fast CN/EN)—enable personalized output.
- MCP tooling hooks (`mcp-host`, `mcp-server`) empower chained tool use without leaving the DORA dataflow.

## Sample Dataflow Configuration
```yaml
# snippets from examples/voice-chat-with-aec-maas.yml
nodes:
  - id: mac_aec
    path: mac_aec_simple_segmentation.py
    env:
      SPEECH_END_FRAMES: "12"
      QUESTION_END_SILENCE_MS: "800"
  - id: asr
    uses: node-hub/dora-asr
    env:
      FUNASR_MODEL_DIR: "/models/funasr"
      WHISPER_MODEL_PATH: "/models/ggml-medium.bin"
  - id: llm
    uses: node-hub/dora-maas-client
    env:
      OPENAI_API_KEY: "${OPENAI_API_KEY}"
      OPENAI_BASE_URL: "https://api.openai.com/v1"
  - id: tts
    uses: node-hub/dora-kokoro-tts
    env:
      KOKORO_VOICE_DIR: "/models/kokoro"
links:
  - mac_aec:audio -> asr:audio
  - asr:text -> llm:prompt
  - llm:text -> tts:text
  - tts:audio -> audio_player:audio
```

## Environment Checklist
- `OPENAI_API_KEY`, `OPENAI_BASE_URL` (or provider-specific variants) for `dora-maas-client`.
- `FUNASR_MODEL_DIR`, `WHISPER_MODEL_PATH` or `WHISPER_MODEL` for `dora-asr` local models.
- `QWEN3_MODEL_HOME`, `LLAMA_CPP_MODEL_PATH`, or `MLX_MODEL_DIR` when using `dora-qwen3`.
- `PRIMESPEECH_MODEL_DIR`, `PRIMESPEECH_VOICE_ID`, or `KOKORO_VOICE_DIR` for TTS nodes.
- `SPEECH_END_FRAMES`, `QUESTION_END_SILENCE_MS`, and device selection env vars referenced in `mac_aec_simple_segmentation.py`.

## How to Navigate
1. Read `goal.md` for learning outcomes and prerequisites.
2. Explore `requirements/` for architecture decisions, component deep dives, and model selection.
3. Follow the walkthroughs to reproduce `mac-aec-chat`, `chatbot-openai-0905`, and `podcast-generator`.
4. Capture your observations in the “Lessons Learned” placeholders within each walkthrough.

Keep `cargo check --all` and `dora start --detach` handy to validate dataflows while you iterate on the tutorials.
