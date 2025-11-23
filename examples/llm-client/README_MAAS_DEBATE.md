# MaaS-based Debate System with Alibaba Qwen-max

This directory contains a debate system using Alibaba Cloud's Qwen-max model via the `dora-maas-client`.

## Files

### MaaS Configuration Files
- **debate_config_maas_llm1.toml** - Daniu (正方/Pro-side debater)
- **debate_config_maas_llm2.toml** - Yifei (反方/Con-side debater)
- **debate_config_maas_judge.toml** - Judge (主持人/Moderator)

### Dataflow
- **dataflow-maas-debate.yml** - Complete debate system using MaaS clients

### Shared Components
- **debate_monitor.py** - Real-time TUI visualization (same for both OpenAI and MaaS)
- **debate_viewer.py** - Log viewer (same for both OpenAI and MaaS)

## Comparison: OpenAI vs MaaS

| Aspect | OpenAI Version | MaaS Version |
|--------|---------------|--------------|
| **Binary** | `openai-response-client` | `dora-maas-client` |
| **Dataflow** | `dataflow-debate.yml` | `dataflow-maas-debate.yml` |
| **Config Files** | `debate_config_llm*.toml` | `debate_config_maas_*.toml` |
| **Model** | gpt-4o | qwen-max |
| **Provider** | OpenAI | Alibaba Cloud |
| **API Key** | `OPENAI_API_KEY` | `ALIBABA_CLOUD_API_KEY` |
| **API URL** | api.openai.com/v1 | dashscope.aliyuncs.com |
| **Language** | English-capable | Chinese-optimized |

## Prerequisites

### 1. Build dora-maas-client
```bash
cd /Users/yuechen/home/fresh/dora
cargo build --release -p dora-maas-client
```

### 2. Set API Key
```bash
export ALIBABA_CLOUD_API_KEY="your-alibaba-cloud-api-key"
```

You can get an API key from: https://dashscope.console.aliyun.com/

## Running the MaaS Debate

### Start the debate with monitor
```bash
cd examples/llm-client
ALIBABA_CLOUD_API_KEY="your-key" dora start dataflow-maas-debate.yml --attach --name debate-monitor
```

### Using the Monitor

1. **Send debate topic**: Type your debate topic in the input bar and press Enter
   - Example: "人工智能是否会取代人类工作" (Will AI replace human jobs?)

2. **View debate**: The monitor shows three panels:
   - **Left**: LLM1 (Daniu) - Pro-side arguments
   - **Middle**: Judge - Moderation and scoring
   - **Right**: LLM2 (Yifei) - Con-side arguments

3. **Reset bridges**: Click "Reset Bridges" button if needed

4. **Controls**:
   - `Ctrl+C` - Quit
   - `Ctrl+L` - Clear all panels

### Start with viewer (logs only)
```bash
ALIBABA_CLOUD_API_KEY="your-key" dora start dataflow-maas-debate.yml --attach --name viewer
```

## System Prompts

All system prompts are identical to the OpenAI version, ensuring consistent debate behavior:

### Daniu (LLM1 - Pro-side)
- Personality: Rational, calm, objective, aggressive
- Style: Fact-based, data-driven, concise
- Prefix: `[Daniu]`

### Yifei (LLM2 - Con-side)
- Personality: Emotional, empathetic, competitive
- Style: Passionate, determined to win
- Prefix: `[Yifei]`

### Judge (Moderator)
- Role: Impartial moderator
- Responsibilities:
  - Manages debate flow
  - Ensures equal speaking time
  - Scores both debaters after 8 rounds
- Prefix: `[Judge]`

## Architecture

```
┌─────────────┐
│   Judge     │◄────────┐
│  (qwen-max) │         │
└──────┬──────┘         │
       │                │
       ├────────────────┤
       │                │
       │  Conference    │
       │   Bridges      │
       │                │
┌──────▼──────┐  ┌──────▼──────┐
│    LLM1     │  │    LLM2     │
│  (qwen-max) │  │  (qwen-max) │
│   Daniu     │  │   Yifei     │
└─────────────┘  └─────────────┘
```

**Message Flow:**
1. Judge receives control prompt (debate topic)
2. Judge initiates debate and calls on Daniu
3. Bridge forwards Judge + previous opponent → Each LLM
4. Both LLMs respond
5. Bridge bundles both responses → Judge
6. Judge evaluates and continues debate
7. Repeat for 8 rounds, then Judge scores

## Configuration Details

### Provider Configuration
```toml
[[providers]]
id = "alicloud"
kind = "alicloud"
api_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
api_key = "env:ALIBABA_CLOUD_API_KEY"
```

### Model Routing
```toml
[[models]]
id = "qwen-max"
route = { provider = "alicloud", model = "qwen-max" }
```

### Settings
- `max_history_exchanges = 20` - Keeps last 20 message pairs
- `enable_streaming = true` - Real-time response display
- `enable_tools = false` - No MCP tools needed for debate
- `log_level = "INFO"` - Standard logging

## Troubleshooting

### API Key Not Set
```
Error: Environment variable ALIBABA_CLOUD_API_KEY not found
```
**Solution**: Export the API key before running:
```bash
export ALIBABA_CLOUD_API_KEY="your-key"
```

### Binary Not Found
```
Error: No such file: ../../target/release/dora-maas-client
```
**Solution**: Build the MaaS client:
```bash
cargo build --release -p dora-maas-client
```

### Rate Limiting
If you see rate limit errors, the Alibaba Cloud API has usage quotas. Wait a moment or upgrade your API plan.

## Switching Between OpenAI and MaaS

To switch between OpenAI (gpt-4o) and Alibaba (qwen-max):

**Use OpenAI**:
```bash
export OPENAI_API_KEY="your-openai-key"
dora start dataflow-debate.yml --attach --name debate-monitor
```

**Use MaaS (Alibaba)**:
```bash
export ALIBABA_CLOUD_API_KEY="your-alibaba-key"
dora start dataflow-maas-debate.yml --attach --name debate-monitor
```

Both systems use the same:
- Conference bridges
- Debate monitor UI
- Viewer
- Message routing logic

## Next Steps

- Try different debate topics
- Compare debate styles between GPT-4o and Qwen-max
- Experiment with other Alibaba models (qwen-plus, qwen-turbo)
- Add more debaters or judges
