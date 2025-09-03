#!/bin/sh
set -euo pipefail

cd /opt/dora/examples/chatbot-openai-websocket-browser || cd /opt/dora

# Determine auth
EXTRA_ARGS=""
if [ -n "${JUPYTER_PASSWORD:-}" ]; then
  HASH=$(python - <<'PY'
import os
try:
    from notebook.auth import passwd
except Exception:
    from jupyter_server.auth.security import passwd
print(passwd(os.environ.get('JUPYTER_PASSWORD', 'change-me')))
PY
)
  EXTRA_ARGS="--ServerApp.password=${HASH} --ServerApp.token=''"
else
  TOKEN="${JUPYTER_TOKEN:-dora}"
  EXTRA_ARGS="--ServerApp.token=${TOKEN}"
fi

PORT="${JUPYTER_PORT:-8888}"
echo "[jupyter] Starting JupyterLab on 0.0.0.0:${PORT}"
exec jupyter lab --ip=0.0.0.0 --port="${PORT}" --no-browser \
  --ServerApp.allow_origin='*' --ServerApp.allow_remote_access=True \
  ${EXTRA_ARGS}

