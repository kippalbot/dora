#############################
# Builder: Rust binaries
#############################
FROM rust:1.85-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates pkg-config libssl-dev build-essential \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
# Use local workspace so image contains your current code/templates
COPY . /src

# Build the WebSocket server and CLI (used by the server to orchestrate dataflows)
RUN cargo build --release -p dora-openai-websocket \
 && cargo build --release -p dora-maas-client \
 && cargo install dora-cli --version 0.3.12 --locked

#############################
# Runtime: Python + system deps
#############################
FROM python:3.12-slim AS runtime

ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    HOST=0.0.0.0 \
    PORT=8123 \
    PIP_NO_CACHE_DIR=1

# System libs for audio/ML and dev tooling (no conda)
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates \
    build-essential pkg-config \
    libopenblas-dev openssl libssl-dev \
    portaudio19-dev libsndfile1 libgomp1 libomp-dev \
    ffmpeg \
 && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/dora
# Copy only needed parts of your (local) repo to reduce final image size
RUN mkdir -p /opt/dora/examples/chatbot-openai-websocket-browser \
             /opt/dora/examples/model-manager \
             /opt/dora/examples/setup-new-chatbot \
             /opt/dora/node-hub
COPY --from=builder /src/examples/chatbot-openai-websocket-browser /opt/dora/examples/chatbot-openai-websocket-browser
COPY --from=builder /src/examples/model-manager /opt/dora/examples/model-manager
COPY --from=builder /src/examples/setup-new-chatbot /opt/dora/examples/setup-new-chatbot
COPY --from=builder /src/node-hub/dora-asr /opt/dora/node-hub/dora-asr
COPY --from=builder /src/node-hub/dora-primespeech /opt/dora/node-hub/dora-primespeech
COPY --from=builder /src/node-hub/dora-speechmonitor /opt/dora/node-hub/dora-speechmonitor
COPY --from=builder /src/node-hub/dora-text-segmenter /opt/dora/node-hub/dora-text-segmenter

# Bring in Python dependency manifests (managed via pip-tools)
COPY docker/requirements.in /tmp/requirements.in
COPY docker/requirements.txt /tmp/requirements.txt

# Copy built Rust binaries and dora CLI
COPY --from=builder /src/target/release/dora-openai-websocket /usr/local/bin/dora-openai-websocket
COPY --from=builder /src/target/release/dora-maas-client /usr/local/bin/dora-maas-client
COPY --from=builder /usr/local/cargo/bin/dora /usr/local/bin/dora

# Python deps (CPU-only torch; adjust if you add CUDA)
RUN pip install --no-cache-dir --upgrade pip wheel setuptools \
 && pip install --no-cache-dir numpy==1.26.4 \
 && pip install --no-cache-dir --index-url https://download.pytorch.org/whl/cpu \
      torch==2.2.0 torchvision==0.17.0 torchaudio==2.2.0 \
 && pip install --no-cache-dir -r /tmp/requirements.txt \
 && pip install --no-cache-dir pyaudio sounddevice \
 && pip install --no-cache-dir funasr-onnx onnxruntime onnxruntime-extensions

# Install pywhispercpp from GitHub (requires git); install git temporarily and purge afterwards to save space
RUN apt-get update && apt-get install -y --no-install-recommends git \
 && pip install --no-cache-dir "git+https://github.com/absadiki/pywhispercpp.git" \
 && apt-get purge -y git && apt-get autoremove -y \
 && apt-get clean && rm -rf /var/lib/apt/lists/*

# JupyterLab for in-container editing (optional service)
RUN pip install --no-cache-dir jupyterlab

# Sanity check core ASR deps (non-fatal: print warning if missing)
RUN python - <<'PY'
try:
    import funasr_onnx, onnxruntime as ort
    print('funasr_onnx:', getattr(funasr_onnx, '__version__', 'unknown'))
    print('onnxruntime providers:', ort.get_available_providers())
except Exception as e:
    print('WARN: funasr_onnx/onnxruntime import issue:', e)
try:
    import pywhispercpp
    print('pywhispercpp OK')
except Exception as e:
    print('WARN: pywhispercpp import issue:', e)
PY

# Install Dora Python nodes (editable) used by the voice/chatbot flows
RUN pip install -e node-hub/dora-asr --no-deps \
               -e node-hub/dora-primespeech --no-deps \
               -e node-hub/dora-speechmonitor --no-deps \
               -e node-hub/dora-text-segmenter --no-deps || true

# Default model directory: mount your host path here
VOLUME ["/root/.dora/models"]

# The WebSocket server expects to find dataflow templates in CWD
WORKDIR /opt/dora/examples/chatbot-openai-websocket-browser

# Expose WebSocket port
EXPOSE 8123

# Start Dora and the WebSocket server (listens on HOST:PORT)
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh
COPY docker/jupyter-entrypoint.sh /usr/local/bin/jupyter-entrypoint.sh
RUN chmod +x /usr/local/bin/jupyter-entrypoint.sh
CMD ["/usr/local/bin/entrypoint.sh"]
