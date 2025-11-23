# Personalized Voice Strategies

Realistic, branded responses help conversation agents stand out. This guide captures two approaches used in the examples and their configuration tips.

## Multiple TTS Instances
- Spin up parallel TTS nodes (e.g., `node-hub/tts/piper`, `node-hub/tts/personalized`) and route prompts based on speaker tags.
- Use DORA routing metadata (`payload.headers.voice_id`) to pick the correct voice at runtime.
- Balance throughput by staggering sample rates; low-latency voice responses can run at 16 kHz while high-fidelity podcast narration targets 48 kHz.

## Voice Cloning Workflow
- Collect consented reference audio clips and preprocess with `python tools/segment_voice.py --input raw/ --output processed/`.
- Train or fine-tune the personalized voice model via `python tools/build_voice.py --dataset processed/ --speaker-id guest_01`.
- Update `config/personalized_voice.toml` with the generated voice profile path and test using `dora start --node personalized_tts --inspect` to validate timbre.

## Deployment Tips
- Cache synthesized responses that repeat across sessions (`node-hub/utils/cache`) to reduce compute overhead.
- Document voice ownership and licensing in `docs/voices.md` to simplify reviews.
