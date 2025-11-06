# LLM Debate Client Run Guide

This directory contains Dora dataflows and terminal user interfaces for running structured LLM debates. Two classes of reasoning nodes are available:
- **OpenAI Responses API** via `openai-response-client`
- **Alibaba Cloud MaaS** via `dora-maas-client`

Each flow can be paired with either the **Python Textual TUI** (`debate_monitor.py`) or the **Rust Ratatui TUI** (`debate-monitor-tui`). This guide explains how to mix and match those components.

---

## 1. Prerequisites

1. **Build Dora binaries** from the repository root (needed for both dataflows):
   ```bash
   cargo build --release \
     -p openai-response-client \
     -p dora-maas-client \
     -p dora-conference-bridge
   ```
   The resulting executables live under `target/release/`.

2. **Install Python dependencies** for the Textual TUI:
   ```bash
   cd examples/llm-client
   pip install -r requirements.txt
   ```

3. **API keys and configuration**
   - OpenAI flow: export `OPENAI_API_KEY` and review `debate_config_llm*.toml`.
   - MaaS flow: export `ALIBABA_CLOUD_API_KEY` and review `debate_config_maas_*.toml`.

4. **Optional viewer**: the dynamic log viewer can be launched with `python debate_viewer.py` after the dataflow starts.

---

## 2. Combination Matrix

| Reasoning backend | Dataflow file | Python TUI command | Rust TUI command |
|-------------------|---------------|--------------------|------------------|
| OpenAI Responses API (`openai-response-client`) | `dataflow-debate.yml` | `python debate_monitor.py` | `cd debate-monitor-tui && cargo run -r` |
| Alibaba Cloud MaaS (`dora-maas-client`) | `dataflow-maas-debate.yml` | `python debate_monitor.py` | `cd debate-monitor-tui && cargo run -r` |

All commands below assume you are inside `examples/llm-client/`.

**Start every debate with a topic prompt.** When the monitor launches, click the input field at the bottom of the screen and paste or type a kickoff message for the judge, for example:

```
今天辩论的题目是动物实验是满足人类和动物长期健康福祉必要的牺牲，开始辩论
```

The judge only begins moderating after receiving this control message.

---

## 3. OpenAI Responses API Flows

### 3.1 Python Textual TUI

1. Prepare environment:
   ```bash
   export OPENAI_API_KEY="your-openai-key"
   cd examples/llm-client
   ```
2. Start the Dora graph:
   ```bash
   dora start dataflow-debate.yml
   ```
3. In a second terminal, launch the Textual monitor:
   ```bash
   cd examples/llm-client
   python debate_monitor.py
   ```
   - Use the on-screen **Reset** button (or type `reset`) whenever you want to clear bridge state and start a fresh debate round.
4. (Optional) In a third terminal, start the viewer:
   ```bash
   python debate_viewer.py
   ```
5. When finished, press `Ctrl+C` in each terminal and stop the graph:
   ```bash
   dora stop
   ```

### 3.2 Rust Ratatui TUI

1. Ensure the OpenAI dataflow is running as shown above.
2. In a new terminal:
   ```bash
   cd examples/llm-client/debate-monitor-tui
   cargo run -r
   ```
   The binary automatically registers as the dynamic node `debate-monitor`.
   - Press `Ctrl+R` at any time to reset the bridges and restart the debate.
3. Quit with `q`, then stop the dataflow:
   ```bash
   cd ../
   dora stop
   ```

---

## 4. Alibaba Cloud MaaS Flows

### 4.1 Python Textual TUI

1. Set credentials and enter the workspace:
   ```bash
   export ALIBABA_CLOUD_API_KEY="your-maas-key"
   cd examples/llm-client
   ```
2. Start the MaaS dataflow:
   ```bash
   dora start dataflow-maas-debate.yml
   ```
3. In a second terminal, run the Textual monitor:
   ```bash
   cd examples/llm-client
   python debate_monitor.py
   ```
   - Click **Reset** (or type `reset`) to flush the conference bridges before starting a new topic.
4. Optionally run the viewer (`python debate_viewer.py`) in another terminal.
5. Shut everything down with `Ctrl+C` followed by `dora stop`.

### 4.2 Rust Ratatui TUI

1. Keep `dataflow-maas-debate.yml` running as described above.
2. Launch the Rust monitor:
   ```bash
   cd examples/llm-client/debate-monitor-tui
   cargo run -r
   ```
   - Use `Ctrl+R` to reset the debate and send a new topic prompt.
3. Exit with `q` and stop the dataflow (`dora stop`).

---

## 5. Tips and Troubleshooting

- **Dynamic node order**: always start the Dora graph before launching either TUI; dynamic nodes will connect to the running graph.
- **Resetting bridges**: both TUIs accept the command `reset` (followed by Enter) to broadcast `{"command": "reset"}` over `bridge_control`.
- **Logs**: use the optional `debate_viewer.py` or `dora logs <node-id>` to inspect errors if a participant stalls.
- **API limits**: the MaaS flow drives a Playwright session under the hood; long pauses may appear during remote inference. The Python TUI auto-completes messages if no chunk arrives for roughly two seconds.
- **Stopping cleanly**: run `dora stop` whenever you switch between dataflows to avoid stale dynamic node registrations.

This README replaces the previous general chat guide and focuses solely on running the debate flows with each LLM backend and terminal UI. Let me know if you need automation scripts or TMUX layouts for the multi-terminal setup.
