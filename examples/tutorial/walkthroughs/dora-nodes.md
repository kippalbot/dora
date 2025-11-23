# Node Inventory & Configuration Notes

This reference lists key DORA nodes used across the tutorials with pointers to their configuration files and reusable parameters.

## Audio Front-End
- `node-hub/audio/input`: wraps system audio capture; set `device` indices in `config/audio_input.toml`.
- `mac-aec-chat/aec-node`: performs frequency-domain echo cancellation; tune `filter_length` and `adaptation_rate` according to your speakers.
- `node-hub/audio/vad` and `node-hub/audio/segmenter`: share threshold settings; aligning `min_silence_ms` avoids truncated speech.

## ASR
- `node-hub/asr/whisper`: accepts `model_path` (GGUF) and `beam_size`; environment variables select CPU vs. GPU backends.
- `node-hub/asr/funasr`: uses ONNXRuntime; adjust `chunk_size` for latency/accuracy trade-offs.

## LLM
- `chatbot-openai-0905/llm-node`: calls OpenAI streaming APIs; requires API key env vars.
- `node-hub/llm/llama_cpp`: runs local models, supports quantization levels via config.
- `node-hub/llm/openvino`: optimized for Intel iGPUs; ensure runtime libraries are installed.

## TTS
- `node-hub/tts/piper`: configure voice model path and sample rate; pre-download voices in `models/piper/`.
- `node-hub/tts/personalized`: supports multiple cloned voices; route using metadata fields like `voice_id`.
- `podcast-generator/tts`: orchestrates multiple voices using a scheduler node.

## Utilities
- `node-hub/utils/queue`, `selector`, and `router`: manage concurrency, multi-LLM fan-out, and message prioritization.
- `node-hub/utils/logger`: writes structured logs for debugging; enable when collecting data for Lessons Learned sections.

Add new nodes to this inventory with a short description, configuration path, and validation tips.
