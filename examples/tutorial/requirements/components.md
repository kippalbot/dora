# Core Components

Each component points to the code used in `mac-aec-chat` and includes real configuration taken from the example dataflows. Use the snippets as anchors when wiring new tutorials.

## Acoustic Echo Cancellation (AEC)

**Feature Highlights**

- `mac-aec-chat/mac_aec_simple_segmentation.py` wraps the MAC dynamic node, combining hardware AEC with segmentation and VAD cues (`speech_started`, `speech_ended`, `question_ended`).
- Automatically loads `node-hub/dora-aec` when available, falling back to `node-hub/dora-mac-aec`.

### Sample YAML (`voice-chat-with-aec.yml`)

```yaml
- id: mac-aec
  path: dynamic
  outputs:
    - audio
    - is_speaking
    - speech_started
    - speech_ended
    - audio_segment
    - question_ended
    - log
```

### Key Environment Variables

- `SPEECH_END_FRAMES` – silence frames required before cutting a segment (default 10).
- `QUESTION_END_SILENCE_MS` – extra silence before emitting `question_ended` (default 1000–3000 ms).
- Ensure `lib/libAudioCapture.dylib` stays on the library search path for the dynamic node.

## Speech Monitor & VAD

**Feature Highlights**

- Hardware VAD is delivered by the same `mac_aec_simple_segmentation.py`, emitting `is_speaking` and `speech_started/ended` signals.
- ML-based VAD is available through `node-hub/dora-speechmonitor`; integrate it to add Silero-based confidence scores when hardware support is absent.

### Sample YAML (`voice-chat-with-aec.yml` hardware VAD)

```yaml
- id: mac-aec
  path: dynamic
  outputs:
    - audio
    - is_speaking
    - speech_started
    - speech_ended
    - audio_segment
    - question_ended
    - log
```

### Key Environment Variables

- `SPEECH_END_FRAMES`, `QUESTION_END_SILENCE_MS` – align hardware VAD timing with downstream queues.
- `MIN_AUDIO_AMPLITUDE`, `VAD_THRESHOLD`, `SAMPLE_RATE` – primary knobs when using `dora-speechmonitor` (see its README for defaults).

## Segmenter

**Feature Highlights**

- `node-hub/dora-text-segmenter` (referenced here as `text-segmenter`) buffers LLM output and sends manageable chunks to TTS, respecting backpressure signals.

### Sample YAML (`voice-chat-with-aec.yml`)

```yaml
  # Text Segmenter - buffers LLM output and sends to TTS one segment at a time
  - id: text-segmenter
    build: pip install -e ../../node-hub/dora-text-segmenter
    path: dora-text-segmenter
    inputs:
      text: qwen3-llm/text  # From LLM
      tts_complete: primespeech/segment_complete  # TTS completion signal
      reset: mac-aec/question_ended  # Clear queue when new question detected
    outputs:
      - text_segment
      - status
      - metrics
      - log
    env:
      ENABLE_BACKPRESSURE: "false"  # Don't wait initially - send first segment immediately
      SEGMENT_MODE: "sentence"  # sentence, punctuation, or fixed
      MIN_SEGMENT_LENGTH: "5"
      MAX_SEGMENT_LENGTH: "20"
      PUNCTUATION_MARKS: "。！？.!?，,"
      LOG_LEVEL: "DEBUG"
```

### Key Environment Variables

- `ENABLE_BACKPRESSURE` – toggle queue waiting behaviour.
- `SEGMENT_MODE`, `MIN_SEGMENT_LENGTH`, `MAX_SEGMENT_LENGTH`, `PUNCTUATION_MARKS` – define slicing heuristics.
- `LOG_LEVEL` – raise to `DEBUG` when tuning latency.

## Automatic Speech Recognition (ASR)

**Feature Highlights**

- `node-hub/dora-asr` routes to FunASR for Mandarin and Whisper for English/Chinese mixed speech, exposing language detection and confidence metrics.

### Sample YAML (`voice-chat-with-aec.yml`)

```yaml
  # ASR transcription
  - id: asr
    build: pip install -e ../../node-hub/dora-asr
    path: dora-asr
    inputs:
      audio:
        source: mac-aec/audio_segment
        queue_size: 10
    outputs:
      - transcription
      - language_detected
      - processing_time
      - confidence
      - log
    env:
      ASR_ENGINE: funasr
      LANGUAGE: zh
      WHISPER_MODEL: large
      ENABLE_PUNCTUATION: true
      ENABLE_LANGUAGE_DETECTION: true
      ENABLE_CONFIDENCE_SCORE: false
      ASR_MODELS_DIR: $HOME/.dora/models/asr # Relative path (recommended)
      LOG_LEVEL: INFO
```

### Key Environment Variables

- `ASR_ENGINE` (`funasr`, `whisper`, or `auto`) and `LANGUAGE` (`zh`, `en`, or `auto`).
- `WHISPER_MODEL`, `ASR_MODELS_DIR` – point to downloaded GGUF/ONNX assets.
- `ENABLE_PUNCTUATION`, `ENABLE_LANGUAGE_DETECTION`, `ENABLE_CONFIDENCE_SCORE`, `LOG_LEVEL` – feature toggles.

## Large Language Models (LLM)

**Feature Highlights**

- Local inference relies on `node-hub/dora-qwen3`, which supports MLX, llama.cpp, Gemma, Qwen, and GLM Air through one interface.
- Cloud inference uses the `dora-maas-client` binary to reach OpenAI-compatible providers (OpenAI, Alibaba Cloud, MiniMax) configured via `maas_config.local.toml`.

### Sample YAML (`voice-chat-with-aec.yml` local LLM)

```yaml
  # Direct connection: ASR -> LLM (with streaming for fast response)
  - id: qwen3-llm
    build: pip install -e ../../node-hub/dora-qwen3
    path: dora-qwen3
    inputs:
      text: asr/transcription  # Direct from ASR
    outputs:
      - text
      - status
      - log
    env:
      # Model Configuration
      USE_MLX: auto

      # MLX model settings (for Apple Silicon)

      # Choose your model (uncomment one):
      #MLX_MODEL: "mlx-community/GLM-4.5-Air-3bit"  # GLM-4.5 Air - Very efficient 3-bit Chinese model
      #MLX_MODEL: "Qwen/Qwen3-32B-MLX-6bit"  # Qwen3 32B - Large model
      MLX_MODEL: "Qwen/Qwen3-8B-MLX-4bit"  # Qwen3 8B - Balanced
      #MLX_MODEL: "mlx-community/gemma-2-9b-it-4bit"  # Gemma 2 - For English/mixed

      MLX_MAX_TOKENS: 256
      # Note: GLM and Qwen models work best without explicit temperature settings

      # Generation settings (these are for GGUF backend, not MLX)
      MAX_TOKENS: 256
      TEMPERATURE: 0.7
      ENABLE_THINKING: false

      # Enable streaming for faster TTS
      LLM_ENABLE_STREAMING: "true"

      # History management - choose ONE strategy:
      HISTORY_STRATEGY: "token_based"  # Options: "fixed", "token_based", or "sliding_window"

      # For "fixed" strategy (currently active):
      MAX_HISTORY_EXCHANGES: "10"  # Keep last 10 Q&A pairs (20 messages total)

      # For "token_based" strategy (not active unless you change HISTORY_STRATEGY):
      MAX_HISTORY_TOKENS: "3000"  # Max tokens for history (only used when HISTORY_STRATEGY="token_based")

      # System prompt
      SYSTEM_PROMPT: "你是AI助手。请以自然流畅的中文口语化表达直接回答问题，避免冗余的思考过程。如果问题不明确，请礼貌地请求澄清。回答既不要过短也不要过长，以适应对话语境。使用口语化输出，回答应准确、精炼且有依据。不要输出 markdown 格式，不要生成不能够被语音合成的内容，不要使用 1, 2， 3 这种书面化的表达方式。"

      LOG_LEVEL: INFO
```

### Sample YAML (`voice-chat-with-aec-maas.yml` cloud LLM)

```yaml
  # MaaS Client with Playwright Browser Tools
  - id: maas-client
    path: ../../target/release/dora-maas-client
    inputs:
      text: asr/transcription  # Direct from ASR
    outputs:
      - text
      - status
      - log
    env:
      # Local config file in mac-aec-chat directory
      MAAS_CONFIG_PATH: maas_config.local.toml
      # Forward API credentials into the MaaS client process
      OPENAI_API_KEY: ${OPENAI_API_KEY:-}
      ALIBABA_CLOUD_API_KEY: ${ALIBABA_CLOUD_API_KEY:-}
      LOG_LEVEL: INFO
```

### Key Environment Variables

- `USE_MLX`, `MLX_MODEL`, `MLX_MAX_TOKENS`, `MAX_TOKENS`, `TEMPERATURE`, `LLM_ENABLE_STREAMING`, `HISTORY_STRATEGY`, `MAX_HISTORY_EXCHANGES`, `MAX_HISTORY_TOKENS`, `SYSTEM_PROMPT` – govern local LLM behaviour.
- `MAAS_CONFIG_PATH` – points to the TOML file describing providers and tools.
- `OPENAI_API_KEY`, `ALIBABA_CLOUD_API_KEY` (and other provider-specific keys declared in the config) – credentials for the MaaS client.

## Text-to-Speech (TTS)

**Feature Highlights**

- `node-hub/dora-primespeech` delivers cloned and fine-tuned Chinese voices suited for high-fidelity responses.
- `node-hub/dora-kokoro-tts` enables fast duplex playback for Chinese and English.

### Sample YAML (`voice-chat-with-aec.yml` PrimeSpeech)

```yaml
  # PrimeSpeech TTS
  - id: primespeech
    build: pip install -e ../../node-hub/dora-primespeech
    path: dora-primespeech
    inputs:
      text: text-segmenter/text_segment  # From text segmenter (not directly from LLM)
    outputs:
      - audio
      - segment_complete
      - log

    env:
      # Allow transformers to load models (temporary workaround for CVE-2025-32434)
      TRANSFORMERS_OFFLINE: "1"
      HF_HUB_OFFLINE: "1"

      # Voice selection
      VOICE_NAME: Doubao  # Available: Doubao, Luo Xiang, Yang Mi, Zhou Jielun, Ma Yun, Maple, Cove
      PRIMESPEECH_MODEL_DIR: $HOME/.dora/models/primespeech
      # Language settings
      TEXT_LANG: zh  # zh for Chinese, en for English, auto for detection
      PROMPT_LANG: zh  # Language of the reference prompt

      # Inference parameters
      TOP_K: 5
      TOP_P: 1.0
      TEMPERATURE: 1.0
      SPEED_FACTOR: 1.0  # Speech speed multiplier

      # Performance
      USE_GPU: false
      NUM_THREADS: 4

      RETURN_FRAGMENT: "false"  # Disable streaming TTS for now
      LOG_LEVEL: "INFO"
      # Internal text segmentation for faster TTS
      ENABLE_INTERNAL_SEGMENTATION: "true"  # Split long text internally
      TTS_MAX_SEGMENT_LENGTH: "100"  # Max chars per TTS segment
      TTS_MIN_SEGMENT_LENGTH: "20"   # Min chars per TTS segment

      # Logging
      LOG_LEVEL: INFO  # DEBUG, INFO, WARNING, ERROR
```

### Sample YAML (`voice-chat-with-aec-maas-kokoro.yml` Kokoro)

```yaml
  # Kokoro TTS - Fast multi-language TTS (replacing PrimeSpeech)
  - id: kokoro-tts
    build: pip install -e ../../node-hub/dora-kokoro-tts
    path: dora-kokoro-tts
    inputs:
      text:
        source: text-segmenter/text_segment  # From text segmenter (not directly from LLM)
        queue_size: 10
    outputs:
      - audio
      - segment_complete  # For backpressure control
      - log
    env:
      # Language and voice settings
      LANGUAGE: en  # en, zh, ja, ko (auto-detects Chinese characters)
      VOICE: af_bella  # Voice selection (af_heart, bf_emma, am_adam, etc.)
      SPEED: "1.0"  # Speech speed multiplier (0.5-2.0)

      # Logging
      LOG_LEVEL: DEBUG  # DEBUG for testing
```

### Key Environment Variables

- PrimeSpeech: `VOICE_NAME`, `PRIMESPEECH_MODEL_DIR`, `TEXT_LANG`, `PROMPT_LANG`, `TOP_K`, `TOP_P`, `TEMPERATURE`, `SPEED_FACTOR`, `USE_GPU`, `RETURN_FRAGMENT`, `ENABLE_INTERNAL_SEGMENTATION`.
- Kokoro: `LANGUAGE`, `VOICE`, `SPEED`, `LOG_LEVEL`.
- Share segmenter outputs (`segment_complete`) for flow control; adjust `queue_size` to balance latency.

> Keep these snippets in sync with the actual dataflows whenever you update node parameters.
