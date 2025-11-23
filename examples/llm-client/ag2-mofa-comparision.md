# Dora Debate Stack vs. AG2 Agent Graphs

## 1. Introduction

This note compares two ways to build the “three-agent debate” experience that currently lives in `examples/llm-client/`:

- **Dora-based implementation** (MoFA debate stack), which wires Rust and Python nodes into a Dora dataflow.
- **Hypothetical AG2 implementation**, using the `ag2` open-source framework from [ag2ai/ag2](https://github.com/ag2ai/ag2).

Rather than squeezing everything into a single table, this document walks through architecture, core functionality, voice integration, deployment concerns, and the developer experience end-to-end. The focus is on what you would actually touch when building, shipping, and running a debate product.

---

## 2. Dora Architecture in Detail

### 2.1 Node Topology

- **Judge, Debater A, Debater B** are individual executables (`openai-response-client`, `dora-maas-client`, etc.) configured via TOML files and launched by the Dora runtime.
- **Conference bridges** (`dora-conference-bridge`) merge streaming outputs, enforce turn order, and maintain `question_id` metadata.
- **Dynamic nodes** (`debate_monitor.py`, `debate_viewer.py`, Rust Ratatui TUI) subscribe to streams, render UI, and emit control messages (`reset`, `prompt`).
- Every node exchanges data as Arrow batches over named channels declared in YAML (`dataflow-debate.yml`, `dataflow-maas-debate.yml`).

### 2.2 Build & Ship Workflow

1. **Build binaries**: `cargo build --release -p openai-response-client -p dora-maas-client -p dora-conference-bridge`.
2. **Author configuration**: environment variables and TOML files per node (model IDs, temperature, logging).
3. **Define dataflow**: YAML describes exact connectivity and environment for each node.
4. **Ship**: package binaries + YAML into a container or deploy manually; operators launch the dataflow and dynamic nodes in separate terminals.

### 2.3 Run-Time Behavior

- The Dora daemon launches each node as a process and pushes Arrow messages across channels.
- Bridges inspect metadata (`session_status`, `segment_index`) to know when a speaker is finished, when to forward bundles, and when to increment `question_id`.
- Dynamic nodes are long-lived processes that must stay connected; they send control payloads as Arrow arrays (`reset`, `prompt`).
- To reset the debate, the monitor emits `{"command": "reset"}` to the bridges; everything else is manual.

### 2.4 Voice Integration

- Dora already has reusable audio nodes (`mac-aec`, `dora-asr`, `primespeech`, `kokoro-tts`).
- Extending the debate flow to voice is wiring: feed ASR output into the judge bridge; route judge/debater responses into TTS nodes and play them back with `audio_player.py`.
- Streaming, buffering, and timing are handled by the dedicated nodes; no extra glue code needed.

### 2.5 Strengths & Trade-Offs

- **Strengths**: Language-agnostic, consistent streaming semantics, battle-tested voice components, deterministic dataflow, easy to run offline.
- **Trade-offs**: Requires building & orchestrating multiple processes, understanding Arrow payloads, maintaining YAML graphs, adding manual logging/observability.

---

## 3. AG2 Architecture in Detail

### 3.1 Conceptual Model: Agents, Graphs, Gateways

- **Agents**: Python objects encapsulating an LLM driver, system prompt, memory policy, and optional tools. Example include `Agent(name="Judge", driver=drivers.OpenAIResponses(...))`.
- **Graphs**: Authored with the AG2 DSL (`with ag.flow() as debate:`). Nodes in the graph are calls to agents, tools, evaluators, or ops; dependencies define execution order and parallelism.
- **Gateways / Storage**: AG2 handles message history, persistent memory (Redis/Postgres/vector stores), retries, logging, and run metadata automatically.

### 3.2 Build-Ship-Run Workflow

**Build**
- Install `ag2` and driver dependencies (`pip install ag2[openai,anthropic,ollama]` etc.).
- Define drivers per role: `drivers.OpenAIResponses`, `drivers.OpenAIChat`, `drivers.Litellm`, `drivers.HuggingFace`, custom `BaseDriver` subclasses.
- Compose agents with system prompts, temperature, max tokens, tool lists, and memory policies (`ag.memory.Buffer`, vector memory drivers).
- Author flows using the DSL with loops, branches, evaluators, and typed inputs/outputs.

**Ship**
- Run locally (`python debate_flow.py`) for development.
- Package as a container or publish to **AG2 Launchpoints** (the hosted “build once deploy anywhere” product). Launchpoints handle provisioning, autoscaling, and scheduling for you.
- Optionally register flows/agents/tools into the AG2 registry for reuse across teams.

**Run**
- Use the AG2 CLI or dashboard to trigger flows with parameters (e.g., `ag2 run debate_flow --topic "…"`).
- Observe live token streams, logs, tool invocations, and state transitions in the hosted UI.
- Analyze run history with built-in analytics, attach feedback evaluators, and log to external systems (Langfuse, Honeycomb) via connectors.

### 3.3 Key Built-in Functionality

- **Driver catalog**: OpenAI (chat & responses), Azure OpenAI, Anthropic, Hugging Face, Groq, Ollama, local model runners through LiteLLM, custom HTTP drivers.
- **Tools**: `@tools.register` decorator wraps Python callables; AG2 manages JSON schema & tool routing automatically.
- **Evaluators and guards**: Built-in rubric judge, toxicity filters, reward models, manual review dashboards.
- **Memory**: Pluggable buffer memory, Postgres storage, vector DB integrations (Pinecone, Weaviate, Chromadb) with minimal config.
- **Observability**: Real-time stream viewer, run comparison, dataset playback.
- **Deployment**: Launchpoints (managed), Airflow/Dagster connectors, scheduled jobs.
- **Developer Ergonomics**: Single Python file defines everything; hot reload via the CLI; no YAML or separate processes required.

### 3.4 Debate Flow Implementation (AG2)

```python
import ag2 as ag
from ag2 import Agent, drivers, tools

# Drivers per role
judge_driver = drivers.OpenAIResponses(
    model="gpt-4o",
    api_key=os.environ["OPENAI_RESPONSES_KEY"],
)

debater_a_driver = drivers.OpenAIChat(
    model="gpt-4o-mini",
    api_key=os.environ["OPENAI_CHAT_KEY"],
)

debater_b_driver = drivers.Litellm(
    provider="ollama",
    model="llama3",
    base_url="http://localhost:11434",
)

# Optional tool shared by judge and debaters
@tools.register(name="citations_search")
def citations_search(query: str) -> str:
    ...  # HTTP call to search provider

# Configure agents
judge = Agent(
    name="Moderator",
    driver=judge_driver,
    system_prompt="""
        You are the debate judge. Maintain order and announce a final verdict
        when satisfied. Mention 'Final verdict:' when the debate should end.
    """,
    tools=[citations_search],
    temperature=0.2,
)

debater_a = Agent(
    name="Daniu",
    driver=debater_a_driver,
    system_prompt="Argue FOR the motion using empirical evidence.",
    tools=[],
    memory=ag.memory.Buffer(max_turns=4),
)

debater_b = Agent(
    name="Yifei",
    driver=debater_b_driver,
    system_prompt="Argue AGAINST the motion focusing on ethics.",
    tools=[citations_search],
)

# Debate flow with explicit turn order
with ag.flow(name="debate") as debate:
    topic = ag.input("topic_prompt")
    judge_intro = judge(topic)

    loop = ag.loop(max_iters=8)
    with loop:
        a_reply = debater_a(ag.join(judge_intro, loop.last("judge_turn", judge_intro)))
        b_reply = debater_b(ag.join(judge_intro, a_reply))
        judge_turn = judge(ag.join(a_reply, b_reply))

        loop.write("judge_turn", judge_turn)
        loop.continue_if(~ag.contains(judge_turn, "Final verdict"))

    ag.output(judge_turn)
```

- The **flow graph** ensures `debater_b` fires only after `debater_a`; the judge waits for both debaters.
- The **loop** stores the judge’s last message (`loop.write`) and stops when the judge outputs “Final verdict”.
- Every agent retains its own model, prompt, toolset, and optional memory—no shared queues needed.

### 3.5 Voice Integration Considerations

- AG2 is text-first. To add ASR/TTS you would:
  - Create a tool that posts audio to Whisper/Deepgram/etc. and returns text.
  - Create another tool that takes generated text and returns an audio file/URL via a TTS API.
  - Manage microphone capture and playback outside AG2 (e.g., a Streamlit or FastAPI wrapper).
- The flow still orchestrates the debate, but you own the surrounding streaming stack.

### 3.6 Strengths & Trade-Offs

- **Strengths**: Python-only development, fast iteration, built-in registry/tooling, hosted infrastructure, rich observability, easy mixing of cloud/local models per agent.
- **Trade-offs**: No native audio stack, requires AG2 runtime (either self-hosted or Launchpoints), less control over low-level streaming semantics, dependency on AG2-specific abstractions.

---

## 4. Head-to-Head Comparison

### 4.1 Control vs. Convenience

- Dora exposes everything: you decide how metadata is passed, how backpressure is handled, and even how logs are collected. That is powerful but verbose.
- AG2 hides orchestration details. You think in terms of Python functions and let the runtime queue tasks, retry failures, store transcripts, and deliver telemetry.

### 4.2 Voice Enablement

| Capability     | Dora Implementation                                                | AG2 Implementation                                                                 |
|----------------|--------------------------------------------------------------------|------------------------------------------------------------------------------------|
| ASR            | Use `mac-aec` + `dora-asr` node; outputs stream automatically      | Write a custom tool calling external ASR service; manage audio capture manually    |
| TTS            | Use `primespeech` / `kokoro-tts` nodes; connect to audio player    | Write a tool calling TTS service; deliver audio to UI yourself                     |
| Playback       | `audio_player.py` dynamic node                                     | Build custom playback in your web/mobile client                                    |

### 4.3 Learning Curve

- **Dora**: Expect to learn Dora CLI, YAML graph syntax, Arrow schemas, dynamic node lifecycle, Rust build pipeline, and debugging across processes.
- **AG2**: One Python API. If you know Python + async tooling, you can be productive quickly. Loop/evaluator syntax is conceptually similar to orchestrators like Prefect or Airflow.

### 4.4 Composability & Expandability

| Aspect             | Dora                                                         | AG2                                                                                                     |
|--------------------|--------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| New nodes / skills | Implement a node (Rust/Python), expose channels, edit YAML   | Add a tool (`@tools.register`) or agent; reuse flows as subgraphs; publish to AG2 registry               |
| Tool integration   | Wrap external service inside Dora node                       | Built-in tool registry + auto JSON schema handling                                                       |
| Observability      | Custom solutions (e.g., `debate_viewer.py`, Grafana)         | AG2 dashboard, run history, dataset playback, external integrations (Langfuse, Honeycomb, Weights & Biases) |
| Deployment         | User-managed processes / containers                          | Local CLI or Launchpoints (autoscaled managed service)                                                   |
| Persistence        | Implement yourself (write transcripts to files/DBs)          | Built-in memory backends (Redis, Postgres, vector DB) with minimal configuration                         |

### 4.5 Configuration Flexibility

- **Dora**: Model selection and prompts live in TOML/env per node. Changing a role’s model means editing the configuration and rebuilding if the binary changes.
- **AG2**: Each `Agent` binds its driver and prompt in Python. Mixing providers (OpenAI responses for judge, azure-openai for Debater A, local Ollama for Debater B) is straightforward. Temperature, tokens, tools, and fallbacks are arguments on the agent/driver, not separate files.

### 4.6 Deployment Workflows

- **Dora**: You own everything—package binaries, distribute YAML, orchestrate processes, and run multi-terminal setups or container orchestrators (Docker Compose, Kubernetes).
- **AG2**:
  - Run locally via CLI or Jupyter.
  - Deploy to Launchpoints with `ag2 launch deploy debate_flow`. Launchpoints manage secrets, scaling, queueing.
  - Schedule jobs via Airflow, Dagster, or AG2’s scheduler integrations.

---

## 5. Takeaways

1. **Control vs. Rapid Assembly**
   - Dora offers maximum control, language-agnostic nodes, and native voice pipelines.
   - AG2 offers rapid Python-native development with a batteries-included ecosystem (drivers, tools, dashboards, hosted runtime).

2. **Voice Support**
   - Dora wins out-of-the-box thanks to existing ASR/TTS nodes.
   - AG2 requires building audio ingestion/playback yourself or integrating third-party streaming services.

3. **Team Skill Set Alignment**
   - Dora fits teams comfortable with systems programming, multi-process orchestration, and explicit dataflow management.
   - AG2 fits Python-first ML teams that want single-language development, fast iteration, and hosted operations.

4. **Scalability & Operations**
   - Dora scales by running more nodes; you manage infrastructure.
   - AG2 scales via Launchpoints; you inherit observability, queuing, and storage without extra scaffolding.

5. **When to Pick Which**
   - **Choose Dora** if you need deterministic low-level control, plan to integrate audio quickly, or want full offline capability.
   - **Choose AG2** if you want to leverage managed tooling, compose complex multi-agent workflows in Python, and benefit from AG2’s build/ship/run tooling (registry, Launchpoints, dashboards, evaluators).

---

*Document location*: `examples/llm-client/ag2-mofa-comparision.md`

*Last updated*: `2025-11-06`
