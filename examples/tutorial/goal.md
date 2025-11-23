# Tutorial Goals

## Learning Outcomes
- Understand the end-to-end topology of a full duplex DORA conversation agent and the purpose of each node in the chain.
- Configure audio front-ends (AEC, VAD, segmenter) so the system stays responsive while capturing clean voice input.
- Integrate ASR, LLM, and TTS backends—local or cloud—to deliver conversational latency under realistic network constraints.
- Compose multi-turn dialogues that support interruption, multiple LLM personalities, and personalized voice output.

## Prerequisites
- Install Rust toolchains and Python tooling listed in project `AGENTS.md`; ensure `dora` CLI is on your PATH.
- Familiarity with the sample projects `mac-aec-chat`, `chatbot-openai-0905`, and `podcast-generator`; clone any external model weights in advance.
- Access credentials for optional cloud providers (OpenAI, Alibaba Cloud, MiniMax) if you plan to follow those variants.

## How to Use This Series
1. Review `requirements/architecture.md` to map dataflows onto example nodes.
2. Jump into a walkthrough that matches your primary use case (interactive chat, podcast generation, or custom nodes).
3. Iterate on the lessons learned by comparing pipeline variants in the OminiX and cloud sections.
4. Contribute improvements or extra scenarios by extending the Markdown files in the `tutorial/` directory.
