# OminiX Local Model Inference

Use this guide to select and configure local backends when running the conversation agent without cloud dependencies. Each subsection links to example configuration files or scripts inside `examples/` and `node-hub/`.

## 1. Large Language Models (LLM)
### MLX
- Works on Apple Silicon via `mlx` runtime; see `mac-aec-chat/README.md` for setup steps.
- Convert models with `python scripts/export_mlx.py --model llama3` and point the LLM node to the resulting `.mlx` directory.

### Llama.cpp
- Builds lightweight binaries under `node-hub/llm/llama_cpp`.
- Use `./quantize.sh --model llama-3-8b-instruct` to prepare GGUF weights; configure `model_path` in `config/llama_cpp.toml`.

### OpenVINO
- Ideal for Intel hardware; follow `node-hub/llm/openvino/INSTALL.md` to enable extensions.
- Convert models with `mo --input_model llama3-fp16.onnx --output_dir models/openvino` and update the node manifest.

### Gemma
- Supported through the `node-hub/llm/gemma` adapter.
- Run `python prepare_gemma.py --hf-token ...` to download and cache the weights locally.

## 2. Automatic Speech Recognition (ASR)
### FunASR (ONNX)
- Export models with `python tools/export_funasr.py --model paraformer` and drop them into `models/funasr/`.
- Set `execution_provider = cuda` or `cpu` in `config/funasr.toml` to match your hardware.

### Whisper (whisper.cpp)
- Build binaries via `scripts/build_whisper_cpp.sh`; the demo dataflow in `chatbot-openai-0905` references the resulting executable.
- Use `./main -m models/ggml-medium.en.bin -f sample.wav` to perform sanity checks before wiring into DORA.

## 3. Text-to-Speech (TTS)
### GTPVitis
- Provides FPGA-accelerated synthesis; install dependencies following `node-hub/tts/gtpvitis/README.md`.
- Configure voice IDs and sample rate in `config/gtpvitis.toml`.

### Personalized Voice
- Clone the `node-hub/tts/personalized` example; populate `voices/` with cloned speaker profiles.
- Use `python tools/build_voice.py --input dataset/ --output voices/user_a` to prepare the model bundle.

> Validation: run `cargo test --package dora-node-hub --features local-models` whenever you modify inference adapters.
