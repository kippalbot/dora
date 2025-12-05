# dora-mlx-kokoro

MLX-accelerated Kokoro TTS node for Apple Silicon, providing high-performance text-to-speech with native Metal GPU acceleration.

## Features

- **Fast MLX Inference**: ~24x real-time factor on Apple Silicon
- **High Quality**: #1 ranked in TTS Spaces Arena
- **Lightweight**: 82M parameters model
- **Multilingual**: English (American/British), Chinese, Japanese, Spanish, French, Hindi, Italian, Portuguese
- **Dora Compatible**: Drop-in replacement interface matching dora-primespeech

## Installation

```bash
cd node-hub/dora-mlx-kokoro
pip install -e .
```

## Usage

```yaml
nodes:
  - id: kokoro-tts
    path: dora-mlx-kokoro
    inputs:
      text: llm/text
    outputs:
      - audio
      - status
      - log
    env:
      VOICE_NAME: af_heart
      LANG_CODE: a
      SPEED: 1.0
      LOG_LEVEL: INFO
```

## Environment Variables

| Variable | Description | Default | Options |
|----------|-------------|---------|---------|
| `VOICE_NAME` | Voice character | af_heart | See available voices |
| `LANG_CODE` | Language code | a | a (American), b (British), j (Japanese), z (Chinese) |
| `SPEED` | Speech speed | 1.0 | 0.5-2.0 |
| `MODEL_PATH` | Model path | prince-canuma/Kokoro-82M | HuggingFace model |
| `LOG_LEVEL` | Logging level | INFO | DEBUG/INFO/WARNING/ERROR |

## Available Voices

### American English (lang_code: a)
- af_heart, af_sky, af_bella, af_jessica, af_nicole, af_sarah, af_alloy, af_echo, af_fable, af_onyx, af_nova
- am_adam, am_michael, am_eric, am_lewis, am_dante, am_liam, am_josh, am_glados, am_shimmer

### British English (lang_code: b)
- bf_alice, bf_lily, bf_emma, bf_grace
- bm_george, bm_lewis, bm_daniel, bm_leo

## Performance

Benchmark on Apple Silicon (M-series):
- **Real-time Factor**: ~24x (generates 24 seconds of audio per second)
- **Latency**: First run ~15s compile, subsequent ~2s load
- **Memory**: 1.1-3.4GB peak

## Node Interface

### Inputs
- **text** (string): Text to synthesize
  - Metadata: `session_id`, `request_id`, `participant_id`

- **control** (string): Control commands
  - `stats`, `list_voices`, `change_voice:VoiceName`, `cleanup`

### Outputs
- **audio** (float32 array): Synthesized audio waveform
  - Metadata: `sample_rate`, `duration`, `voice`, `language`

- **status** (string): Synthesis status
  - Values: "completed", "error"
  - Metadata: `session_id`, `request_id`, `error`

- **log** (JSON string): Structured logs
  - Fields: `node`, `level`, `message`, `timestamp`

## License

MIT License. Model license: Apache 2.0 (Kokoro-82M)
