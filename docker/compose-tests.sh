#!/usr/bin/env bash
set -euo pipefail

# Run ASR and TTS validation tests inside the already-running compose `server`.
# Requires: MODELS_DIR and HF_CACHE set as in docker/docker-compose.yml, and the
# server service either running or startable by this script.
#
# Usage:
#   docker/compose-tests.sh [--compose-file docker/docker-compose.yml] [--voice doubao]

COMPOSE_FILE="docker/docker-compose.yml"
VOICE="doubao"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --compose-file|-f) COMPOSE_FILE="$2"; shift 2 ;;
    --voice) VOICE="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

compose() { docker compose -f "$COMPOSE_FILE" "$@"; }

echo "[tests] Ensuring server is up..."
compose up -d server

echo "[tests] Running ASR validation..."
compose exec \
  -e ASR_MODELS_DIR=/root/.dora/models/asr \
  server bash -lc "cd /opt/dora/examples/setup-new-chatbot/asr-validation && python test_basic_asr.py"

echo "[tests] Running TTS validation (voice=$VOICE)..."
compose exec \
  -e PRIMESPEECH_MODEL_DIR=/root/.dora/models/primespeech \
  server bash -lc "cd /opt/dora/examples/setup-new-chatbot/primespeech-validation && python test_tts_direct.py --device cpu --voice '$VOICE'"

echo "[tests] Done. If TTS ran, audio is under examples/setup-new-chatbot/primespeech-validation/tts_output (host-mounted)."

