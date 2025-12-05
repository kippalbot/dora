# Dora Model Manager

`download_models.py` is a universal downloader for Hugging Face models plus curated shortcuts for Dora voice pipelines (FunASR, PrimeSpeech, Kokoro, Qwen MLX, etc.). This README consolidates the quick start and detailed usage notes into one place, including the latest Kokoro features.

---

## 1. Installation

If you have not yet provisioned a Dora development environment, follow `examples/setup-new-chat/README.md` first (it installs `uv`, `rustup`, base Python tooling, etc.).

Inside this directory the model manager installs extra Python packages lazily, but you can pre-install them:

```bash
pip install huggingface-hub tqdm
```

The FunASR shortcuts rely on ModelScope; the setup instructions in `setup-new-chat` already cover the required runtime, so no separate `setup.sh` invocation is necessary.

---

## 2. Quick Start

All commands below assume you are in `examples/model-manager/`.

### List cached models

```bash
python download_models.py --list
```

Scans `~/.cache/huggingface/hub/` and `~/.dora/models/` (PrimeSpeech/Kokoro/FunASR) and prints sizes plus file counts.

### Typical downloads

```bash
# ASR models (FunASR Paraformer + punctuation)
python download_models.py --download funasr

# PrimeSpeech base (Chinese HuBERT & RoBERTa) + all voices
python download_models.py --download primespeech

# Kokoro base + all voices (config.json, kokoro-v1_0.pth, voices/*.pt)
python download_models.py --download kokoro

# Qwen3 MLX (choose one)
python download_models.py --download Qwen/Qwen3-8B-MLX-4bit
```

### Removing artefacts

```bash
python download_models.py --remove funasr
python download_models.py --remove primespeech-base
python download_models.py --remove all-voices
python download_models.py --remove kokoro        # base + voices + HF cache
```

---

## 3. Full Command Reference

### 3.1 Hugging Face repositories

```bash
# Download entire repo snapshot
python download_models.py --download mlx-community/gemma-3-12b-it-4bit

# Custom cache directory
python download_models.py --download meta-llama/Llama-2-7b-hf --hf-dir ~/llama-cache

# Select file types only
python download_models.py --download mlx-community/gemma-3-12b-it-4bit --patterns "*.safetensors" "*.json"

# Specific revision
python download_models.py --download openai/whisper-large-v3 --revision main

# Remove cached repo
python download_models.py --remove mlx-community/gemma-3-12b-it-4bit
```

### 3.2 FunASR

```bash
# Download FunASR models (PyTorch format)
python download_models.py --download funasr

# Remove FunASR models
python download_models.py --remove funasr
```

Content lands in `~/.dora/models/asr/funasr` by default.

**Note**: Downloaded models are in PyTorch format and work immediately with GPU acceleration. ONNX conversion is optional (see section 3.6).

### 3.3 PrimeSpeech

```bash
# Base models only
python download_models.py --download primespeech-base

# List available voices
python download_models.py --list-voices

# All voices
python download_models.py --voice all

# Specific voice
python download_models.py --voice "Luo Xiang"

# Removal
python download_models.py --remove "Luo Xiang"
python download_models.py --remove all-voices
python download_models.py --remove primespeech-base
```

PrimeSpeech assets are stored under `~/.dora/models/primespeech` unless you pass `--models-dir`.

### 3.4 Kokoro TTS

Kokoro supports **two backends**: CPU (PyTorch) and MLX (Apple Silicon GPU). These use **different models** from different repositories.

#### CPU Backend (Cross-Platform)

```bash
# Base files (config.json + kokoro-v1_0.pth) and cache refresh
python download_models.py --download kokoro-base

# All voices only
python download_models.py --download kokoro-voices

# Both base and voices
python download_models.py --download kokoro

# Specific voice (comma-separated list allowed)
python download_models.py --kokoro-voice af_heart

# List available voices on Hugging Face
python download_models.py --list-kokoro-voices

# Remove CPU models
python download_models.py --remove kokoro-base
python download_models.py --remove kokoro-voices
python download_models.py --remove kokoro
```

**CPU Backend Details:**
- **Repository**: `hexgrad/Kokoro-82M`
- **Storage**: `~/.dora/models/kokoro`
- **Platform**: All platforms (Linux, macOS, Windows)
- **Best for**: Short text (<150 chars) - 1.8x faster than MLX

#### MLX Backend (Apple Silicon GPU)

```bash
# Download MLX-optimized model
python download_models.py --download kokoro-mlx

# Remove MLX model
python download_models.py --remove kokoro-mlx
```

**MLX Backend Details:**
- **Repository**: `prince-canuma/Kokoro-82M` (DIFFERENT from CPU version)
- **Storage**: `~/.cache/huggingface/hub/` (HuggingFace cache)
- **Platform**: macOS Apple Silicon only (M1/M2/M3)
- **Best for**: Long text (>200 chars) - up to 3x faster than CPU

#### Performance Comparison

| Text Length | Best Backend | Performance |
|-------------|--------------|-------------|
| Short (12 chars) | **CPU** | 4.35x faster than MLX |
| Medium (79 chars) | **CPU** | 1.83x faster than MLX |
| Long (363 chars) | **MLX** | 1.62x faster than CPU |
| Ultra-long (1542 chars) | **MLX** | 3.02x faster than CPU |

**Crossover point**: ~150-200 characters

#### Backend Selection in Dora

Set the `BACKEND` environment variable in your dataflow YAML:

```yaml
nodes:
  - id: tts
    operator:
      python: ../../node-hub/dora-kokoro-tts
    env:
      BACKEND: "auto"      # auto-select (tries MLX first, falls back to CPU)
      # BACKEND: "cpu"     # force CPU backend
      # BACKEND: "mlx"     # force MLX backend
      VOICE_NAME: "af_heart"
```

For validation tests and performance benchmarks, see:
- `/Users/yuechen/home/fresh/dora/examples/setup-new-chatbot/kokoro-tts-validation/`

### 3.5 Other shortcuts

The script recognises many common repos used in Dora voice demos. Examples:

```bash
python download_models.py --download openai/whisper-base
python download_models.py --download Qwen/Qwen3-14B-MLX-4bit
python download_models.py --download mlx-community/gemma-2-9b-it-4bit
```

Run `python download_models.py --help` for the full option list.

### 3.6 ONNX Model Conversion (Recommended for CPU)

FunASR models support **dual backends** (PyTorch and ONNX). ONNX conversion is **highly recommended for CPU deployment** for optimal performance.

#### When is ONNX conversion needed?

**Short answer: Recommended for CPU systems (Mac, Windows, Linux servers).**

- **ONNX models (quantized)**: Best performance on CPU systems - **2.4x faster than PyTorch**
- **PyTorch models (default)**: Best for GPU acceleration (NVIDIA CUDA)

#### Performance Comparison

| Backend | Device | System | Processing Time | Speed | When to Use |
|---------|--------|--------|----------------|-------|-------------|
| **PyTorch** | **GPU (CUDA)** | RTX 4090 | 0.282s (17.35s audio) | 61.6x real-time | ✅ **Best for GPU** - NVIDIA systems |
| **ONNX** | **CPU** | **MacBook M3 Pro** | **0.104s (3s audio)** | **28.6x real-time** | ✅ **Best for CPU** - Mac/Windows/Linux |
| PyTorch | CPU | MacBook M3 Pro | 0.250s (3s audio) | 11.9x real-time | Slower fallback (2.4x slower than ONNX) |

#### When ONNX conversion is beneficial:

- **CPU-only systems** (Mac, Windows, Linux) - 2.4x faster than PyTorch
- **Cross-platform deployment** (Windows/macOS/Linux/ARM)
- **Embedded systems** requiring ONNX Runtime
- **Inference-only containers** with minimal dependencies

#### How to convert models to ONNX:

```bash
# Convert all FunASR models
python convert_to_onnx.py --convert all

# Convert specific model
python convert_to_onnx.py --model paraformer --input-dir ~/.dora/models/asr/funasr

# The download_all_models.sh script will prompt for ONNX conversion at the end
./download_all_models.sh  # Prompts: "Do you want to convert models to ONNX format? (y/n)"
```

#### Backend Selection

The ASR engine automatically selects the best available backend:

1. **PyTorch GPU** (if GPU available and `USE_GPU=true`)
2. ONNX GPU (if ONNX models present)
3. PyTorch CPU (default fallback)
4. ONNX CPU (last resort)

**Recommendation**:
- **CPU systems (Mac/Windows/Linux)**: Convert to ONNX for 2.4x performance boost
- **GPU systems (NVIDIA CUDA)**: Use PyTorch models (no conversion needed)

---

## 4. Storage Layout

| Location | Contents |
|----------|----------|
| `~/.cache/huggingface/hub/` | Hugging Face snapshots (e.g. `hexgrad--Kokoro-82M`) |
| `~/.dora/models/primespeech/` | PrimeSpeech base + voices |
| `~/.dora/models/kokoro/` | Kokoro base + voices |
| `~/.dora/models/asr/funasr/` | FunASR ASR models |

Override with `--hf-dir`, `--models-dir`, or `--kokoro-dir` when necessary.

---

## 5. Troubleshooting

- **“Model not found”** – ensure the repo ID is correct (case-sensitive). Use `--list` to confirm downloads.
- **Permission errors** – use a user-writable path via `--hf-dir` / `--models-dir`, or adjust filesystem permissions.
- **Interrupted downloads** – the script uses `resume_download=True`; re-run the same command to continue.
- **PrimeSpeech warning** – even if `dora-primespeech` isn’t installed, you can still fetch the models; install the node before running the TTS pipeline.

---

## 6. File Overview

- `download_models.py` – main CLI for downloading models
- `download_all_models.sh` – convenience script for bulk downloads (includes optional ONNX conversion prompt)
- `convert_to_onnx.py` – optional ONNX conversion utility for FunASR models (see section 3.6)

Use this tool to keep Dora voice demos stocked with the correct ASR, LLM, and TTS assets—especially PrimeSpeech and Kokoro, which rely on precise directory structures.
