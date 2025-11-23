# Full Duplex & Interruptible Dialog Control

Building a responsive voice agent requires careful queue management and orchestration between nodes. This guide outlines patterns tested in the sample projects.

## Queue Control
- Use DORA queues (`node-hub/utils/queue`) to buffer ASR transcripts while the LLM streams responses.
- Apply backpressure by setting `max_items` in the queue manifest; when the user speaks again, earlier prompts are discarded to prioritize the latest intent.
- Integrate a cancellation channel from the VAD node to the TTS node so playback stops immediately when fresh speech is detected.

## Multiple LLM Orchestration
- Fan-out transcripts to multiple LLM nodes (e.g., persona-specific or fallback models) using `node-hub/utils/broadcast`.
- Merge responses with `node-hub/utils/selector` that picks the fastest completion or combines insights before handing off to TTS.
- In `podcast-generator`, the planning LLM produces structure while a second LLM drafts final scripts—reuse that pattern for multi-agent chat scenarios.

## Testing Responsiveness
- Record simultaneous speaking sessions and replay them through `dora start --replay traces/full_duplex` to verify interruption handling.
- Capture metrics by enabling the `monitor` node in `mac-aec-chat`, which logs latency per stage; include these findings in the lessons learned section.
