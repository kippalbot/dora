# Dora 辩论系统与 AG2 Agent Graphs 对比

## 1. 引言

本文对比两种实现“三方辩论”体验的方案：

- **Dora 辩论实现**（MoFA debate stack），当前位于 `examples/llm-client/`，通过 Dora 数据流连接 Rust 与 Python 节点。
- **AG2 实现设想**，基于 [ag2ai/ag2](https://github.com/ag2ai/ag2) 的开源框架，以 Python Agent Graph 方式编排。

内容依次覆盖架构、核心功能、语音扩展、交付与运维路径，以及开发体验，让我们在“构建、发布、运行”三个阶段全面掌握两套方案的异同。

---

## 2. Dora 架构详解

### 2.1 节点拓扑

- **裁判、正方、反方**：分别是独立可执行文件（如 `openai-response-client`、`dora-maas-client`），通过 TOML 与环境变量配置，由 Dora 运行时启动。
- **会议桥（Conference Bridge）**：`dora-conference-bridge` 负责合并流式输出、维持发言顺序、管理 `question_id` 等元数据。
- **动态节点**：`debate_monitor.py`、`debate_viewer.py` 以及 Rust Ratatui TUI 作为独立进程订阅 Dora 通道，同时发送控制消息（`reset`、`prompt` 等）。
- 所有节点通过 YAML（`dataflow-debate.yml`、`dataflow-maas-debate.yml`）定义的命名通道以 Arrow 批量形式传递数据。

### 2.2 构建与发布流程

1. **编译二进制**：`cargo build --release -p openai-response-client -p dora-maas-client -p dora-conference-bridge`。
2. **撰写配置**：为各节点准备环境变量与 TOML（模型、温度、日志级别等）。
3. **定义数据流**：在 YAML 中声明节点、输入输出映射、运行环境、资源限制。
4. **交付**：打包二进制与 YAML（容器或手动部署），上线时需在不同终端启动数据流与动态节点。

### 2.3 运行时行为

- Dora 守护进程按数据流启动各节点进程，并通过 Arrow 管道推送消息。
- 会议桥解析元数据（如 `session_status`、`segment_index`），判断发言完成、转发组合文本、递增 `question_id`。
- 动态节点需保持连接，使用 Arrow 数组发送控制消息；重置辩论需向桥发送 `{"command": "reset"}`。

### 2.4 语音集成

- Dora 具备成熟的音频节点（`mac-aec`、`dora-asr`、`primespeech`、`kokoro-tts`）。
- 要将辩论扩展为语音版，只需把 ASR 输出接入裁判输入，再将裁判与辩手文本送入 TTS，并用 `audio_player.py` 播放。
- 流式处理、缓冲、节拍控制均由现成节点完成，无需额外粘合代码。

### 2.5 优势与权衡

- **优势**：语言无关、流式语义明确、语音链路成熟、数据流可控、离线运行容易。
- **权衡**：需要维护多进程、理解 Arrow 载荷与 YAML 图，日志与可观测性需自行搭建。

---

## 3. AG2 架构详解

### 3.1 概念模型：Agents、Graphs、Gateways

- **Agent**：Python 对象，封装 LLM 驱动、系统提示词、记忆策略及工具。
- **Graph**：通过 DSL（`with ag.flow():`、`ag.loop` 等）编排，节点之间依赖关系决定执行顺序与并发。
- **Gateway / 存储**：AG2 管理会话历史、持久化记忆（Redis/Postgres/向量库）、重试、日志、运行元数据。

### 3.2 构建-发布-运行流程

**构建**
- 安装 `ag2` 及所需驱动（`pip install ag2[openai,anthropic,ollama]`）。
- 为每个角色配置驱动：`drivers.OpenAIResponses`、`drivers.OpenAIChat`、`drivers.Litellm`、`drivers.HuggingFace` 等。
- 定义 Agent：设置系统提示、温度、输出长度、工具列表与记忆策略（`ag.memory.Buffer`、向量记忆等）。
- 使用 DSL 编写流程，支持循环、分支、评估器、类型化入参/出参。

**发布**
- 本地运行（`python debate_flow.py`）调试。
- 打包成容器或上传至 **AG2 Launchpoints**（官方托管运行时），Launchpoints 负责排队、调度、扩缩容。
- 可将流、Agent、工具发布到 AG2 Registry，供团队复用。

**运行**
- 通过 CLI 或仪表盘触发流程：`ag2 run debate_flow --topic "..."`。
- 在托管 UI 实时查看 Token 流、日志、工具调用、状态转移。
- 借助内建分析工具对比运行、回放数据集；也可接入 Langfuse、Honeycomb 等第三方观测系统。

### 3.3 核心内建功能

- **驱动目录**：支持 OpenAI（chat 与 responses）、Azure OpenAI、Anthropic、Hugging Face、Groq、Ollama、本地模型（LiteLLM）、自定义 HTTP 驱动。
- **工具系统**：`@tools.register` 装饰器即可暴露 Python 函数，AG2 自动生成 JSON Schema 并处理工具调用。
- **评估与守卫**：内建 Rubric Judge、毒性过滤、奖励模型、人工审核面板，亦可自定义。
- **记忆**：缓冲记忆、Postgres 存储、以及主流向量库（Pinecone、Weaviate、Chroma）连接器。
- **可观测性**：实时流查看、运行对比、数据集复现、日志聚合。
- **部署生态**：Launchpoints 托管、Airflow/Dagster 集成、定时任务调度。
- **开发体验**：单个 Python 项目描述一切，CLI 支持热重载，无需 YAML/多进程管理。

### 3.4 AG2 辩论流程实现示例

```python
import ag2 as ag
from ag2 import Agent, drivers, tools

# 为不同角色选择不同驱动
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

# 共享工具示例
@tools.register(name="citations_search")
def citations_search(query: str) -> str:
    ...

# 定义各 Agent
judge = Agent(
    name="Moderator",
    driver=judge_driver,
    system_prompt="""
        你是辩论裁判，负责维持秩序，并在满意时给出最终判决。
        当辩论结束时，请以“Final verdict:”开头宣布结果。
    """,
    tools=[citations_search],
    temperature=0.2,
)

debater_a = Agent(
    name="Daniu",
    driver=debater_a_driver,
    system_prompt="请从科学角度支持该命题。",
    tools=[],
    memory=ag.memory.Buffer(max_turns=4),
)

debater_b = Agent(
    name="Yifei",
    driver=debater_b_driver,
    system_prompt="请从伦理与动物福利角度反对该命题。",
    tools=[citations_search],
)

# 构建辩论流程
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

- 依赖拓扑自动保证顺序：`debater_b` 在裁判开场与正方发言完成后才执行，裁判需等待两位辩手。
- 循环体使用 `loop.write` 保存上一轮裁判发言，当判断到“Final verdict”时跳出循环，下一次运行恢复初始状态。
- 每个 Agent 独立配置模型、提示词、工具与记忆策略，无需共享队列或手动同步。

### 3.5 语音集成注意事项

- AG2 目前以文本为核心。若要接入语音，需要：
  - 新增工具调用外部 ASR（如 Whisper、Deepgram），负责音频上传与转写。
  - 新增工具调用 TTS，将文本转换为音频，并将音频 URL 或数据返回客户端。
  - 在 AG2 外部（例如 Streamlit、FastAPI）处理麦克风采集、音频播放与流式交互。
- 流程仍由 AG2 驱动，但音频输入输出需自行实现或依赖第三方服务。

### 3.6 优势与权衡

- **优势**：全 Python 开发、迭代效率高，自带工具/评估/监控生态，可选择托管运行；轻松混用云端或本地模型。
- **权衡**：缺乏原生语音栈，需要 AG2 运行环境（自建或托管）；底层流式语义由框架掌控，粒度不及 Dora。

---

## 4. 正面对比

### 4.1 控制 vs. 便利

- Dora 给予最大控制权：元数据、背压、日志、编排全部自行把控，适合追求可预测性与复杂 IO 场景。
- AG2 屏蔽底层细节，让开发者关注“谁调用谁”，框架负责排队、重试、记忆、观测。

### 4.2 语音支持

| 能力          | Dora 实现                                                        | AG2 实现                                                                      |
|---------------|------------------------------------------------------------------|--------------------------------------------------------------------------------|
| ASR           | 直接复用 `mac-aec` + `dora-asr`，成果接入数据流                   | 实现自定义工具调用 ASR API，音频采集需自行处理                                 |
| TTS           | 复用 `primespeech` / `kokoro-tts`，接入播放器即可                | 实现工具调用 TTS 服务，将音频交付给客户端                                     |
| 播放          | 有现成的 `audio_player.py` 动态节点                             | 需在 Web/原生应用中自行实现播放                                               |

结论：Dora 在语音链路上拥有即插即用能力；AG2 虽可扩展，但所有音频流处理由开发者负责。

### 4.3 学习曲线

- **Dora**：需掌握 Dora CLI、YAML 语法、Arrow 数据格式、动态节点生命周期、Rust 构建与多进程调试。
- **AG2**：单一 Python API。熟悉 Python/异步开发后即可快速上手；流程 DSL 类似 Prefect/Airflow。

### 4.4 组合性与可扩展性

| 方面            | Dora                                                         | AG2                                                                                       |
|-----------------|--------------------------------------------------------------|--------------------------------------------------------------------------------------------|
| 新增能力        | 编写新节点（Rust/Python）、声明通道、修改 YAML                | 新增工具或 Agent，流可拆分为子图，并可发布到 AG2 Registry 供复用                           |
| 工具集成        | 需在独立节点中包装外部服务                                  | `@tools.register` 直接暴露函数，AG2 自动处理参数与调用路由                                 |
| 可观测性        | 自建（如 `debate_viewer.py` 或第三方监控）                   | 内建仪表盘、运行历史、数据集回放，支持导出到 Langfuse / Honeycomb 等                        |
| 部署            | 自行 orchestrate，多进程或容器化                              | 本地 CLI、团队自建，或使用 Launchpoints 托管                                             |
| 持久化          | 节点自行落地（写文件/数据库）                               | AG2 提供缓冲记忆、Postgres、向量库等开箱即用的存储选项                                    |

### 4.5 配置灵活性

- **Dora**：模型与提示词位于 TOML/环境变量中，调整角色模型需要编辑配置并可能重新编译。
- **AG2**：每个 Agent 在 Python 中绑定驱动、提示词、工具；轻松混用不同云端/本地供应商，并支持温度、Token、Fallback 等细粒度参数。

### 4.6 部署流程

- **Dora**：完全自管——打包二进制、分发 YAML、在服务器上启动多进程，或采用 Docker Compose/Kubernetes 等方案。
- **AG2**：
  - 本地运行：CLI 或 Jupyter 直接执行。
  - Launchpoints：`ag2 launch deploy debate_flow` 即可托管，平台负责密钥、调度、扩缩容。
  - 调度集成：支持 Airflow、Dagster 或 AG2 自身的定时任务。

---

## 5. 总结

1. **控制权 vs. 快速装配**  
   - Dora 提供最大化的可控性、语言无关节点、成熟语音链路。  
   - AG2 侧重高效迭代与生态配套，Python 一体化、托管基础设施、工具评估齐全。

2. **语音能力差异**  
   - Dora 现成语音节点可迅速搭建实时语音辩论。  
   - AG2 需额外构建音频采集/播放与服务调用逻辑。

3. **团队技能匹配**  
   - Dora 更适合具备系统编程、多进程、数据流经验的团队。  
   - AG2 适合 Python/LLM 工程团队，寻求快速迭代与托管运维。

4. **扩展与运维**  
   - Dora 扩展靠增加节点与 YAML 连接，运维成本由团队承担。  
   - AG2 借助 Launchpoints 等服务可快速上线，内建的观察、评估、记忆机制便于运营。

5. **选择建议**  
   - 当你需要可预测的数据流控制、快速语音整合或完全离线环境时，选择 **Dora**。  
   - 当你希望在 Python 中高速迭代、混用多模型、享受托管平台与工具生态时，选择 **AG2**，同时自行补齐语音链路。

---

*文档位置*: `examples/llm-client/ag2-mofa-comparision.zh.md`  
*最近更新*: 2025-11-06（请根据实际更新时间替换）
