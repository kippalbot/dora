# Glossary & Further Reading

## Glossary
- **AEC (Acoustic Echo Cancellation):** Audio processing that removes speaker playback from the microphone feed to avoid feedback loops.
- **VAD (Voice Activity Detection):** Classifier that tags active speech segments in streaming audio; used to trigger downstream processing.
- **Full Duplex:** Ability to send and receive audio simultaneously, enabling natural interruption and backchanneling.
- **MCP (Model Context Protocol):** Specification for connecting LLMs to external tools over a shared schema.
- **OminiX:** Local inference stack referenced by DORA nodes for running LLM, ASR, and TTS models on-device.

## Further Reading & Assets
- DORA documentation: `docs/` directory and https://github.com/dora-rs/dora/tree/main/docs for official guides.
- Example-specific notes: `examples/mac-aec-chat/README.md`, `examples/chatbot-openai-0905/README.md`, and `examples/podcast-generator/README.md`.
- Audio processing primers: signal chain guidelines in `docs/audio_processing.md` (create if missing) and external resources such as "Real-Time Speech Enhancement" (IEEE Communications).

Add links to whitepapers, recordings, or troubleshooting threads here to capture ongoing lessons learned.
