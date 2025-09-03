#!/bin/sh
set -euo pipefail

echo "[entrypoint] Ensuring clean Dora state..."
dora destroy || true

echo "[entrypoint] Starting Dora background services..."
dora up

echo "[entrypoint] Waiting for Dora to be ready..."
for i in $(seq 1 30); do
  if dora list >/dev/null 2>&1; then
    echo "[entrypoint] Dora is ready."
    break
  fi
  sleep 0.5
done

echo "[entrypoint] Launching WebSocket server on ${HOST:-0.0.0.0}:${PORT:-8123}"
exec dora-openai-websocket
