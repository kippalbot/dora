# Front-End UI & Multi-Platform Support

Pairing DORA backends with interactive front-ends helps validate the full duplex experience. The examples in this repository cover native apps and web integrations.

## Moly App
- The `mac-aec-chat` example integrates with the Moly desktop app for low-latency audio routing.
- Configure device bindings inside the app to match the AEC node settings; verify echo suppression by looping mic to speakers and checking for howling.
- Use the app's logging panel to correlate user events with DORA node timestamps during debugging.

## WebSocket APIs
- `chatbot-openai-websocket-browser` exposes WebSocket endpoints that stream ASR text and synthesized audio to a browser client.
- Authentication and session tracking live in `server/app.py`; extend this file when adding multi-user controls.
- For custom front-ends, follow the same message schema: JSON envelopes with `type`, `timestamp`, and payload fields for text or base64 audio chunks.

## Integration Checklist
- Ensure CORS and TLS are configured before exposing WebSocket endpoints externally.
- Run `dora start --detach` alongside your UI to decouple lifecycle management; monitor `logs/*.json` for transport errors.
- Document UI-specific keybindings or affordances in the walkthrough files so readers can reproduce interactions.
