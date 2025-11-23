# Walkthrough: mac-aec-chat

This guide traces the low-latency duplex chat example on macOS with acoustic echo cancellation.

## 1. Prepare the Environment
- Install dependencies: `brew install portaudio sox` and run `cargo check --manifest-path libraries/mac-aec-chat/Cargo.toml`.
- Enable the Moly app (or equivalent audio router) and map microphone/speaker channels to match `config/aec.toml`.

## 2. Inspect the Dataflow
- Open `examples/mac-aec-chat/dataflow.yml` to review node wiring (audio input → AEC → VAD → Whisper ASR → OpenAI LLM → TTS).
- Run `dora build --dataflow dataflow.yml --graph --output graphs/mac-aec-chat.dot` to visualize the pipeline.

## 3. Launch the Pipeline
- Start supporting services: `uv run ruff check mac-aec-chat` (lint) and `cargo fmt --manifest-path binaries/mac-aec-chat/Cargo.toml`.
- Execute `dora start --dataflow dataflow.yml` and speak into the microphone; watch logs in `logs/mac-aec-chat/` for queue status and latency metrics.

## 4. Experiment & Iterate
- Swap ASR to Whisper.cpp by editing `dataflow.yml` and `config/asr.toml`; rebuild via `dora build`.
- Enable interruption: add the queue control node described in `requirements/dialog-control.md` and connect it between ASR and LLM outputs.
- Capture lessons learned (latency, echo suppression quality, prompt tweaks) at the bottom of this file so future readers see practical tuning tips.
