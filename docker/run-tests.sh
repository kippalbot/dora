#!/usr/bin/env bash
set -euo pipefail

# Run ASR and TTS validation tests inside the Docker image, with mounted models.
#
# Usage:
#   docker/run-tests.sh [--image IMAGE] [--models-dir DIR] [--cache-dir DIR] [--tts-out DIR]
#
# Defaults:
#   IMAGE: dora-voicechat:latest
#   MODELS_DIR: ${HOME}/.dora/models
#   CACHE_DIR: ${HOME}/.cache/huggingface
#   TTS_OUT: ${PWD}/tts_output

IMAGE=${IMAGE:-dora-voicechat:latest}
MODELS_DIR=${MODELS_DIR:-"${HOME}/.dora/models"}
CACHE_DIR=${CACHE_DIR:-"${HOME}/.cache/huggingface"}
TMP_HOME_DIR_DEFAULT="${HOME}/.dora/docker-home"
TMP_HOME_DIR=${TMP_HOME_DIR:-"${TMP_HOME_DIR_DEFAULT}"}
TTS_OUT_DEFAULT_DIR="$(pwd)/tts_output"
TTS_OUT=${TTS_OUT:-"${TTS_OUT_DEFAULT_DIR}"}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --image) IMAGE="$2"; shift 2 ;;
    --models-dir) MODELS_DIR="$2"; shift 2 ;;
    --cache-dir) CACHE_DIR="$2"; shift 2 ;;
    --tmp-home) TMP_HOME_DIR="$2"; shift 2 ;;
    --tts-out) TTS_OUT="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

mkdir -p "$MODELS_DIR" "$CACHE_DIR" "$TTS_OUT" "$TMP_HOME_DIR/.cache/numba" "$TMP_HOME_DIR/.cache/huggingface" "$TMP_HOME_DIR/.dora"

run() {
  docker run --rm \
    -u "$(id -u):$(id -g)" \
    -e HOME=/tmp/home \
    -e ASR_MODELS_DIR=/tmp/home/.dora/models/asr \
    -e ASR_ENGINE=funasr \
    -e PRIMESPEECH_MODEL_DIR=/tmp/home/.dora/models/primespeech \
    -e NUMBA_DISABLE_CACHING=1 \
    -e NUMBA_CACHE_DIR=/tmp/home/.cache/numba \
    -e HF_HOME=/tmp/home/.cache/huggingface \
    -e TRANSFORMERS_CACHE=/tmp/home/.cache/huggingface \
    -e HUGGINGFACE_HUB_CACHE=/tmp/home/.cache/huggingface \
    -v "$TMP_HOME_DIR":/tmp/home \
    -v "$MODELS_DIR":/tmp/home/.dora/models \
    -v "$CACHE_DIR":/tmp/home/.cache/huggingface \
    -v "$TTS_OUT":/opt/dora/examples/setup-new-chatbot/primespeech-validation/tts_output \
    --entrypoint bash \
    "$IMAGE" -lc "$1"
}

echo "[ASR] Checking ASR Python deps..."
run "python -c \"import importlib,sys; mods=['funasr_onnx','onnxruntime','pywhispercpp']; g=globals(); imported=[]; errors={}; code='for m in mods:\\n try:\\n  g[m]=importlib.import_module(m); imported.append(m)\\n except Exception as e:\\n  errors[m]=str(e)'; exec(code); print('Imported:', imported); print('Errors:', errors); print('providers:', g['onnxruntime'].get_available_providers() if 'onnxruntime' in g else 'n/a')\""

echo "[ASR] Converting FunASR models to ONNX (if needed)..."
run "python /opt/dora/examples/model-manager/convert_to_onnx.py --convert all || true"

echo "[ASR] Running basic ASR test..."
run "cd /opt/dora/examples/setup-new-chatbot/asr-validation && python test_basic_asr.py"

echo "[TTS] Running PrimeSpeech TTS test..."
run "cd /opt/dora/examples/setup-new-chatbot/primespeech-validation && python test_tts_direct.py --device cpu --voice doubao"

echo "\n✓ Tests completed. TTS output (if any) is in: $TTS_OUT"
