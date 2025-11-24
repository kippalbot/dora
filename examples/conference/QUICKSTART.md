# Conference Demo Quick Start Guide

This guide shows how to run the **Debate** and **Study** demos using the conference architecture.

## Overview

| Demo | Participants | Policy | Purpose |
|------|-------------|--------|---------|
| **Debate** | llm1 (Daniu), llm2 (Yifei), judge | `[llm1 → llm2 → judge]` | 3-person debate competition |
| **Study** | student1 (Daniu), student2 (Yifei), tutor (Sunwen) | `[student2 → student1 → tutor]` | Interactive learning discussion |

---

## Prerequisites

### 1. Build Required Components

```bash
cd /Users/yuechen/home/fresh/dora

# Build conference bridge (message routing)
cargo build -p dora-conference-bridge --release

# Build conference controller (speaking order management)
cargo build -p dora-conference-controller --release

# Build MaaS client (LLM API integration)
cargo build -p dora-maas-client --release

# Or build all three at once:
cargo build -p dora-conference-bridge -p dora-conference-controller -p dora-maas-client --release
```

### 2. Set API Keys

Choose your provider(s) and set the corresponding API key(s):

```bash
# DeepSeek (default for most configs)
export DEEPSEEK_API_KEY="your-deepseek-api-key"

# OpenAI (for gpt-4o, gpt-4.1, etc.)
export OPENAI_API_KEY="your-openai-api-key"

# Alibaba Cloud (for qwen-max, qwen-plus, etc.)
export ALIBABA_CLOUD_API_KEY="your-alibaba-api-key"
```

---

## Running the Debate Demo

### Step 1: Configure Models (Optional)

Edit the TOML configs to select your preferred model:

**debate_config_maas_llm1.toml** (Daniu - 正方):
```toml
default_model = "deepseek-chat"  # Change to "gpt-4o", "qwen-max", etc.
```

**debate_config_maas_llm2.toml** (Yifei - 反方):
```toml
default_model = "deepseek-chat"
```

**debate_config_maas_judge.toml** (Judge - 主持人):
```toml
default_model = "deepseek-chat"
```

### Step 2: Customize System Prompts (Optional)

Edit the `system_prompt` field in each TOML config to define participant personalities:

```toml
# Example: debate_config_maas_llm1.toml
system_prompt = """你的名字叫Daniu，你在参加一场三人辩论，回答必须用中文，
你的特点是理性冷静和客观，攻击性很强，非常坚持自己的观点...
- 你的回答必须以[Daniu]为起始
- 观点简明，不要长篇大论，使用事实和数据说话
"""
```

### Step 3: Start the Dataflow

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Start the debate dataflow
dora start dataflow-debate-sequential.yml
```

### Step 4: Start the Debate Monitor (TUI)

In a **new terminal**:

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Start the 3-panel TUI
dora run --attach dataflow-debate-sequential.yml --name debate-monitor \
    python debate_monitor.py
```

The monitor shows:
```
┌──────────────────┬──────────────────┐
│  LLM1 (Daniu)    │  LLM2 (Yifei)    │
│  [正方]          │  [反方]          │
└──────────────────┴──────────────────┘
┌─────────────────────────────────────┐
│  Judge (主持人)                     │
└─────────────────────────────────────┘
```

**Controls:**
- `r` - Reset the debate
- `q` or `Ctrl-C` - Quit

### Step 5: Start the Viewer (Optional)

For detailed logging, in **another terminal**:

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

dora run --attach dataflow-debate-sequential.yml --name viewer \
    python ./debate_viewer.py
```

---

## Running the Study Demo

The study demo simulates an interactive learning session with 2 students and 1 tutor discussing a topic from a context file.

### Step 1: Configure the Context File

The context file `study-context.md` contains the learning material with anchor points `[A0]-[A11]`. Participants reference these anchors in their discussion.

Edit `study-context.md` to change the study topic, or create your own context file and update the configs:

```toml
# In study_config_maas_*.toml
anchor_context = "study-context.md"  # Change to your context file
```

### Step 2: Configure Models

**study_config_maas_student1.toml** (Daniu - 理性学霸):
```toml
default_model = "gpt-4.1"  # Uses OpenAI
```

**study_config_maas_student2.toml** (Yifei - 感性学生):
```toml
default_model = "deepseek-chat"  # Uses DeepSeek
```

**study_config_maas_tutor.toml** (Sunwen - 导师):
```toml
default_model = "deepseek-chat"  # Uses DeepSeek
```

### Step 3: Customize Student/Tutor Personalities (Optional)

Each participant has a distinct personality defined in the system prompt:

**Student1 (Daniu)**: 理性学霸 - logical, focused on concepts
```toml
system_prompt = """你是学生 Daniu，非常聪明理性，逻辑强，但不太懂人情世故、幽默感弱。
讨论时以锚点 [A0–A11] 为唯一依据...
"""
```

**Student2 (Yifei)**: 感性学生 - emotional, asks questions
```toml
system_prompt = """你是学生 Yifei，感性、好奇心强，经常提问...
"""
```

**Tutor (Sunwen)**: 苏格拉底式导师 - guides with questions
```toml
system_prompt = """你是小组讨论的引导者 Sunwen，一位中年男性物理老师，
幽默风趣，擅长苏格拉底式"接生婆"学习法...
"""
```

### Step 4: Start the Study Dataflow

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Start the study dataflow
dora start dataflow-study-sequential.yml
```

### Step 5: Start the Study Monitor

In a **new terminal**:

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Set study mode environment variable
export DORA_STUDY_MODE=true

# Start the monitor
dora run --attach dataflow-study-sequential.yml --name debate-monitor \
    python debate_monitor.py
```

### Step 6: Start the Viewer (Optional)

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

dora run --attach dataflow-study-sequential.yml --name viewer \
    python ./debate_viewer.py
```

---

## Configuration Reference

### TOML Config Structure

```toml
# Model selection
default_model = "deepseek-chat"  # Model ID to use

# Personality/behavior
system_prompt = """Your instructions here..."""

# Session settings
max_history_exchanges = 100      # Context window size
enable_streaming = true          # Stream responses
enable_tools = false             # Function calling
status_timeout_seconds = 60      # Timeout per turn

# Study mode only
anchor_context = "study-context.md"  # Context file for study discussions

# Provider definitions
[[providers]]
id = "deepseek"
kind = "deepseek"
api_key = "env:DEEPSEEK_API_KEY"
proxy = false

# Model routes
[[models]]
id = "deepseek-chat"
route = { provider = "deepseek", model = "deepseek-chat" }
```

### Available Models

| Provider | Model ID | Description |
|----------|----------|-------------|
| DeepSeek | `deepseek-chat` | Default, good for Chinese |
| DeepSeek | `deepseek-reasoner` | Advanced reasoning |
| OpenAI | `gpt-4o` | GPT-4 Omni |
| OpenAI | `gpt-4o-mini` | Faster, cheaper GPT-4 |
| OpenAI | `gpt-4.1` | Latest GPT-4 |
| Alibaba | `qwen-max` | Qwen flagship |
| Alibaba | `qwen-plus` | Balanced |
| Alibaba | `qwen-turbo` | Fast, cheap |

### Policy Patterns

The `DORA_POLICY_PATTERN` environment variable controls speaking order:

```yaml
# Sequential (fixed order, loops forever)
DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"

# Ratio-based (proportional speaking time)
DORA_POLICY_PATTERN: "[(judge, 2), (llm1, 1), (llm2, 1)]"

# Priority-based (priority speakers go first)
DORA_POLICY_PATTERN: "[(judge, *), (llm1, 1), (llm2, 1)]"
```

---

## Troubleshooting

### API Key Issues

```bash
# Verify keys are set
echo "DEEPSEEK: ${DEEPSEEK_API_KEY:0:10}..."
echo "OPENAI: ${OPENAI_API_KEY:0:10}..."
echo "ALIBABA: ${ALIBABA_CLOUD_API_KEY:0:10}..."
```

### No Response from LLMs

1. Check the viewer for error messages
2. Verify the model ID matches a configured model in the TOML
3. Check API key for the provider you're using

### Monitor Shows Blank Panels

1. Ensure dataflow is started first: `dora start dataflow-*.yml`
2. Then attach the monitor: `dora run --attach ...`

### Study Context Not Loaded

1. Verify `anchor_context` path in TOML is correct
2. Check that the context file exists: `ls -la study-context.md`

### Reset Not Working

Press `r` in the debate-monitor to reset. The reset signal:
1. Cancels all ongoing LLM responses
2. Clears conversation history
3. Restarts the speaking order

---

## File Structure

```
examples/conference/
├── README_QUICKSTART.md              # This file
├── dataflow-debate-sequential.yml    # Debate dataflow
├── dataflow-study-sequential.yml     # Study dataflow
│
├── debate_config_maas_llm1.toml      # Debate: Daniu (正方)
├── debate_config_maas_llm2.toml      # Debate: Yifei (反方)
├── debate_config_maas_judge.toml     # Debate: Judge (主持人)
│
├── study_config_maas_student1.toml   # Study: Daniu (学霸)
├── study_config_maas_student2.toml   # Study: Yifei (感性)
├── study_config_maas_tutor.toml      # Study: Sunwen (导师)
├── study-context.md                  # Study: Learning context
│
├── debate_viewer.py                  # Log viewer
└── README.md                         # Architecture overview
```

---

## Quick Commands Summary

```bash
# === BUILD (run once) ===
cd /Users/yuechen/home/fresh/dora
cargo build -p dora-conference-bridge -p dora-conference-controller -p dora-maas-client --release

# === DEBATE ===
# Terminal 1: Start dataflow
cd /Users/yuechen/home/fresh/dora/examples/conference
dora start dataflow-debate-sequential.yml

# Terminal 2: Start monitor
dora run --attach dataflow-debate-sequential.yml --name debate-monitor \
    python debate_monitor.py

# Terminal 3 (optional): Start viewer
dora run --attach dataflow-debate-sequential.yml --name viewer \
    python ./debate_viewer.py


# === STUDY ===
# Terminal 1: Start dataflow
dora start dataflow-study-sequential.yml

# Terminal 2: Start monitor
DORA_STUDY_MODE=true dora run --attach dataflow-study-sequential.yml --name debate-monitor \
    python debate_monitor.py

# Terminal 3 (optional): Start viewer
dora run --attach dataflow-study-sequential.yml --name viewer \
    python ./debate_viewer.py


# === STOP ===
dora stop
```
