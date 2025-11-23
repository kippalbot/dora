# MCP Tool Integrations

Model Context Protocol (MCP) connectors extend the agent with external tool access. The examples below demonstrate both voice-first and multi-LLM orchestration approaches.

## OpenAI Voice-Based MCP
- The `mcp-host` and `mcp-server` examples show how to bridge DORA with OpenAI's Voice API.
- Configure credentials via `.env` files and register the MCP tool URL inside the LLM node configuration (`config/openai_mcp.toml`).
- The voice MCP streams actions back into DORA, allowing the agent to query knowledge bases or trigger automations mid-conversation.

## Chained LLM MCP
- `mcp-passthrough-test` demonstrates a mediator node that hands off tool calls to specialized LLMs.
- Route intent classification results through `node-hub/utils/router` so each downstream LLM handles a specific toolset (e.g., calendar, search, code execution).
- Capture tool invocation logs in `logs/mcp/` for postmortem analysis; include notable lessons in the walkthrough appendices.

> Keep MCP schema definitions versioned in `docs/mcp/` to ease upgrades when OpenAI or community toolkits release new capabilities.
