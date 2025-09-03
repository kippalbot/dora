#!/usr/bin/env bash
set -euo pipefail

# Download Dora models inside the Docker image with host mounts.
#
# Usage:
#   docker/download-models.sh [--image IMAGE] [--models-dir DIR] [--cache-dir DIR] \
#       [all|funasr|primespeech|list|voice <NAME>|onnx]
#
# Defaults:
#   IMAGE: dora-voicechat:latest
#   MODELS_DIR: ${HOME}/.dora/models
#   CACHE_DIR: ${HOME}/.cache/huggingface

IMAGE=${IMAGE:-dora-voicechat:latest}
MODELS_DIR=${MODELS_DIR:-"${HOME}/.dora/models"}
CACHE_DIR=${CACHE_DIR:-"${HOME}/.cache/huggingface"}

arg_image="$IMAGE"
arg_models="$MODELS_DIR"
arg_cache="$CACHE_DIR"

cmd=${1:-all}
shift || true

while [[ $# -gt 0 ]]; do
  case "$1" in
    --image)
      arg_image="$2"; shift 2 ;;
    --models-dir)
      arg_models="$2"; shift 2 ;;
    --cache-dir)
      arg_cache="$2"; shift 2 ;;
    *)
      # pass-through (e.g., voice <NAME>)
      break ;;
  esac
done

mkdir -p "$arg_models" "$arg_cache"

run() {
  docker run --rm \
    -u "$(id -u):$(id -g)" \
    -e HOME=/tmp/home \
    -e HF_HOME=/tmp/home/.cache/huggingface \
    -e TRANSFORMERS_CACHE=/tmp/home/.cache/huggingface \
    -e HUGGINGFACE_HUB_CACHE=/tmp/home/.cache/huggingface \
    -v "$arg_models":/tmp/home/.dora/models \
    -v "$arg_cache":/tmp/home/.cache/huggingface \
    --entrypoint bash \
    "$arg_image" -lc "mkdir -p /tmp/home && $1"
}

case "$cmd" in
  all)
    run "python /opt/dora/examples/model-manager/download_models.py --download funasr && python /opt/dora/examples/model-manager/download_models.py --download primespeech && python /opt/dora/examples/model-manager/download_models.py --list" ;;
  funasr)
    run "python /opt/dora/examples/model-manager/download_models.py --download funasr" ;;
  primespeech)
    run "python /opt/dora/examples/model-manager/download_models.py --download primespeech" ;;
  list)
    run "python /opt/dora/examples/model-manager/download_models.py --list" ;;
  voice)
    name=${1:-}
    if [[ -z "$name" ]]; then
      echo "Usage: $0 voice <NAME> [--image ... --models-dir ... --cache-dir ...]" >&2
      exit 1
    fi
    run "python /opt/dora/examples/model-manager/download_models.py --voice '$name'" ;;
  onnx)
    run "python /opt/dora/examples/model-manager/convert_to_onnx.py --convert all" ;;
  *)
    echo "Unknown command: $cmd" >&2
    echo "Valid: all | funasr | primespeech | list | voice <NAME> | onnx" >&2
    exit 1 ;;
esac

echo "\n✓ Done. Models are in: $arg_models"
