# Conference Bridge & Controller Architecture

# 会议桥接器与控制器架构设计

---

## Table of Contents / 目录

1. [Overview / 概述](#overview--概述)
2. [Architecture Design / 架构设计](#architecture-design--架构设计)
3. [Key Design Decisions / 关键设计决策](#key-design-decisions--关键设计决策)
4. [Race Condition Handling / 竞态条件处理](#race-condition-handling--竞态条件处理)
5. [Node Specifications / 节点规格](#node-specifications--节点规格)
6. [Dataflow Configuration / 数据流配置](#dataflow-configuration--数据流配置)
7. [Challenges & Solutions / 挑战与解决方案](#challenges--solutions--挑战与解决方案)
8. [Debugging Guide / 调试指南](#debugging-guide--调试指南)

---

## Overview / 概述

### English

The Conference Bridge & Controller system implements a multi-participant real-time conversation framework using Dora's dataflow architecture. It enables structured debates, discussions, or any turn-based conversation between multiple LLM participants with a policy-driven speaking order.

**Key Components:**
- **Conference Controller**: The "brain" that determines who speaks next based on configurable policies
- **Conference Bridge**: The "switch" that buffers and forwards messages to participants based on controller commands
- **LLM Participants**: AI models that engage in the conversation (e.g., debaters, judge)

### 中文

会议桥接器与控制器系统使用 Dora 数据流架构实现了多参与者实时对话框架。它支持多个 LLM 参与者之间的结构化辩论、讨论或任何基于轮次的对话，并通过策略驱动的发言顺序进行控制。

**核心组件：**
- **会议控制器 (Conference Controller)**：决定下一个发言者的"大脑"，基于可配置的策略
- **会议桥接器 (Conference Bridge)**：根据控制器命令缓冲和转发消息的"交换机"
- **LLM 参与者**：参与对话的 AI 模型（如辩论者、裁判）

---

## Architecture Design / 架构设计

### System Architecture Diagram / 系统架构图

```mermaid
flowchart TB
    subgraph Participants["LLM Participants / LLM 参与者"]
        LLM1[("llm1<br/>正方/Affirmative")]
        LLM2[("llm2<br/>反方/Negative")]
        JUDGE[("judge<br/>裁判/Judge")]
    end

    subgraph Controller["Conference Controller / 会议控制器"]
        POLICY["Policy Engine<br/>策略引擎"]
        CTRL["Control Logic<br/>控制逻辑"]
    end

    subgraph Bridges["Conference Bridges / 会议桥接器"]
        B1["bridge-to-llm1<br/>桥接到LLM1"]
        B2["bridge-to-llm2<br/>桥接到LLM2"]
        B3["bridge-to-judge<br/>桥接到裁判"]
    end

    %% LLM outputs to Controller
    LLM1 -->|text| CTRL
    LLM2 -->|text| CTRL
    JUDGE -->|text| CTRL

    %% LLM outputs to Bridges
    LLM2 -->|text| B1
    JUDGE -->|text| B1
    LLM1 -->|text| B2
    JUDGE -->|text| B2
    LLM1 -->|text| B3
    LLM2 -->|text| B3

    %% Controller to Bridges (control signals)
    CTRL -->|control_llm1<br/>resume| B1
    CTRL -->|control_llm2<br/>resume| B2
    CTRL -->|control_judge<br/>resume| B3

    %% Bridges to LLMs
    B1 -->|text| LLM1
    B2 -->|text| LLM2
    B3 -->|text| JUDGE

    %% Policy influence
    POLICY -.->|determines<br/>next speaker| CTRL

    style Controller fill:#e1f5fe
    style Bridges fill:#fff3e0
    style Participants fill:#e8f5e9
```

### Data Flow Sequence / 数据流序列图

```mermaid
sequenceDiagram
    participant J as Judge
    participant Ctrl as Controller
    participant B1 as Bridge-to-LLM1
    participant L1 as LLM1
    participant B2 as Bridge-to-LLM2
    participant L2 as LLM2

    Note over J,L2: Round 1: Judge opens debate / 第一轮：裁判开场

    J->>+Ctrl: text (streaming chunks)
    J->>+B1: text (streaming chunks)
    J->>+B2: text (streaming chunks)

    Note over J: session_status: "ended"
    J->>Ctrl: completion signal
    J->>B1: completion signal
    J->>B2: completion signal

    Ctrl->>Ctrl: Policy: next = llm1
    Ctrl->>B1: resume

    B1->>B1: Check: ready_inputs={judge}, any_streaming=false
    B1->>L1: Forward judge's message

    Note over L1,L2: Round 2: LLM1 speaks / 第二轮：LLM1 发言

    L1->>+Ctrl: text (streaming)
    L1->>+B2: text (streaming)
    L1->>+B1: text (to self-bridge, ignored)

    L1->>Ctrl: completion signal
    Ctrl->>Ctrl: Policy: next = llm2
    Ctrl->>B2: resume

    B2->>L2: Forward llm1 + judge messages
```

### Three-Bridge Architecture / 三桥架构

```mermaid
flowchart LR
    subgraph "Message Flow to LLM1"
        LLM2_out1[LLM2] --> B1[bridge-to-llm1]
        JUDGE_out1[Judge] --> B1
        B1 --> LLM1_in[LLM1]
    end

    subgraph "Message Flow to LLM2"
        LLM1_out2[LLM1] --> B2[bridge-to-llm2]
        JUDGE_out2[Judge] --> B2
        B2 --> LLM2_in[LLM2]
    end

    subgraph "Message Flow to Judge"
        LLM1_out3[LLM1] --> B3[bridge-to-judge]
        LLM2_out3[LLM2] --> B3
        B3 --> JUDGE_in[Judge]
    end

    style B1 fill:#ffcc80
    style B2 fill:#ffcc80
    style B3 fill:#ffcc80
```

---

## Key Design Decisions / 关键设计决策

### 1. Separation of Control and Forwarding / 控制与转发分离

#### English

**Decision**: Separate the "who speaks next" logic (Controller) from the "message buffering and forwarding" logic (Bridge).

**Rationale**:
- **Single Responsibility**: Controller focuses on policy decisions; Bridge focuses on message handling
- **Flexibility**: Different policies can be applied without modifying bridge logic
- **Testability**: Each component can be tested independently
- **Scalability**: Multiple bridges can share one controller, or vice versa

**Implementation**:
```
Controller: Observes all participants → Applies policy → Sends "resume" to appropriate bridge
Bridge: Buffers incoming messages → Waits for "resume" → Forwards ready messages
```

#### 中文

**决策**：将"谁下一个发言"的逻辑（控制器）与"消息缓冲和转发"的逻辑（桥接器）分离。

**理由**：
- **单一职责**：控制器专注于策略决策；桥接器专注于消息处理
- **灵活性**：可以应用不同策略而无需修改桥接器逻辑
- **可测试性**：每个组件可以独立测试
- **可扩展性**：多个桥接器可以共享一个控制器，反之亦然

**实现**：
```
控制器：观察所有参与者 → 应用策略 → 向适当的桥接器发送 "resume"
桥接器：缓冲传入消息 → 等待 "resume" → 转发就绪的消息
```

### 2. Policy-Based Controller / 基于策略的控制器

#### English

**Decision**: Use configurable policies to determine speaking order.

**Supported Policies**:
- **Sequential**: Fixed rotation (e.g., `[llm1 → llm2 → judge]`)
- **Ratio-based**: Weighted speaking time allocation
- **Custom**: Extensible policy interface

**Configuration**:
```yaml
env:
  DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
```

#### 中文

**决策**：使用可配置策略来确定发言顺序。

**支持的策略**：
- **顺序策略**：固定轮换（如 `[llm1 → llm2 → judge]`）
- **比例策略**：加权发言时间分配
- **自定义策略**：可扩展的策略接口

**配置**：
```yaml
env:
  DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
```

### 3. Streaming Support with Completion Detection / 流式支持与完成检测

#### English

**Decision**: Support streaming LLM outputs with explicit completion signals.

**Completion Detection**:
```rust
fn is_message_complete(&self, metadata: &BTreeMap<String, Parameter>) -> bool {
    // Check session_status
    if let Some(Parameter::String(status)) = metadata.get("session_status") {
        if status == "ended" {
            return true;
        }
    }
    // Check is_complete flag
    if let Some(Parameter::Bool(true)) = metadata.get("is_complete") {
        return true;
    }
    false
}
```

**Message States**:
```
None → Streaming { chunks: [...] } → Complete { content: "..." }
```

#### 中文

**决策**：支持带有明确完成信号的流式 LLM 输出。

**完成检测**：
```rust
fn is_message_complete(&self, metadata: &BTreeMap<String, Parameter>) -> bool {
    // 检查 session_status
    if let Some(Parameter::String(status)) = metadata.get("session_status") {
        if status == "ended" {
            return true;
        }
    }
    // 检查 is_complete 标志
    if let Some(Parameter::Bool(true)) = metadata.get("is_complete") {
        return true;
    }
    false
}
```

**消息状态**：
```
None → Streaming { chunks: [...] } → Complete { content: "..." }
```

### 4. Signal Classification System / 信号分类系统

#### English

**Decision**: Classify incoming signals by type to handle reset, cancellation, and error scenarios gracefully.

**Signal Types**:
```rust
enum SignalType {
    ResetSignal,      // session_status: "reset" - drop silently
    CancelledSignal,  // session_status: "cancelled" - drop silently
    TechnicalError,   // session_status: "error" - forward template message
    ContentError,     // Text starts with "Error:" - forward template message
    NormalContent,    // Regular content - forward as-is
}
```

**Handling Rules**:
| Signal Type | Action | Reason |
|------------|--------|--------|
| `ResetSignal` | Drop silently | Control signal, not content |
| `CancelledSignal` | Drop silently | Interrupted, incomplete response |
| `TechnicalError` | Forward template | Notify other participants |
| `ContentError` | Forward template | Notify other participants |
| `NormalContent` | Forward as-is | Normal conversation content |

**Error Message Template**:
```yaml
env:
  ERROR_MESSAGE_TEMPLATE: "[{participant} is experiencing technical difficulties.]"
```

#### 中文

**决策**：按类型分类传入信号，以优雅地处理重置、取消和错误场景。

**信号类型**：
```rust
enum SignalType {
    ResetSignal,      // session_status: "reset" - 静默丢弃
    CancelledSignal,  // session_status: "cancelled" - 静默丢弃
    TechnicalError,   // session_status: "error" - 转发模板消息
    ContentError,     // 文本以 "Error:" 开头 - 转发模板消息
    NormalContent,    // 正常内容 - 原样转发
}
```

**处理规则**：
| 信号类型 | 动作 | 原因 |
|---------|------|------|
| `ResetSignal` | 静默丢弃 | 控制信号，非内容 |
| `CancelledSignal` | 静默丢弃 | 已中断，响应不完整 |
| `TechnicalError` | 转发模板 | 通知其他参与者 |
| `ContentError` | 转发模板 | 通知其他参与者 |
| `NormalContent` | 原样转发 | 正常对话内容 |

### 5. Reset Handling with State Recovery / 重置处理与状态恢复

#### English

**Decision**: Implement coordinated reset across controller and bridges with proper state recovery.

**Controller Reset Flow**:
1. Set `reset_pending = true` BEFORE sending reset commands
2. Send "reset" to all bridges and LLMs
3. Ignore all incoming signals until `session_status: "started"` arrives
4. Clear `reset_pending` when new conversation starts

**Bridge Reset Flow**:
1. Receive "reset" command → call `reset_state()`
2. Clear all input buffers and arrival queue
3. Set `resume_mode = false`
4. When new content arrives with `session_status: "started"`:
   - Reset message state to fresh `Streaming` state
   - Allow new content accumulation

**Key Fix - New Message Start Detection**:
```rust
// Detect new message start and reset state
let is_new_start = metadata.get("session_status")
    .map_or(false, |status| status == "started");

if is_new_start {
    self.message_state = Some(MessageState::new_streaming());
    self.ready = false;
    self.was_already_ready = false;
}
```

#### 中文

**决策**：在控制器和桥接器之间实现协调的重置，并正确恢复状态。

**控制器重置流程**：
1. 在发送重置命令之前设置 `reset_pending = true`
2. 向所有桥接器和 LLM 发送 "reset"
3. 忽略所有传入信号，直到收到 `session_status: "started"`
4. 新对话开始时清除 `reset_pending`

**桥接器重置流程**：
1. 收到 "reset" 命令 → 调用 `reset_state()`
2. 清空所有输入缓冲区和到达队列
3. 设置 `resume_mode = false`
4. 当收到带有 `session_status: "started"` 的新内容时：
   - 将消息状态重置为新的 `Streaming` 状态
   - 允许新内容累积

**关键修复 - 新消息开始检测**：
```rust
// 检测新消息开始并重置状态
let is_new_start = metadata.get("session_status")
    .map_or(false, |status| status == "started");

if is_new_start {
    self.message_state = Some(MessageState::new_streaming());
    self.ready = false;
    self.was_already_ready = false;
}
```

### 6. Two-Path Forwarding Strategy / 双路径转发策略

#### English

**Decision**: Implement two paths for message forwarding to handle race conditions.

**Path 1 - Input Event Loop**:
- Triggered when: An input completes AND bridge is in resume_mode
- Action: Check if no other inputs are streaming, then forward

**Path 2 - Control Event Loop**:
- Triggered when: "resume" command is received
- Action: If ready inputs exist AND no ongoing streaming → forward immediately
- Otherwise: Stay in resume_mode, let Path 1 handle it

```rust
// Path 2: Control resume handler
if !ready_inputs.is_empty() && !any_streaming {
    forward_bundle();  // Forward immediately
} else {
    // Stay in resume_mode, Path 1 will forward when streaming completes
}

// Path 1: Input completion handler
if bridge.resume_mode && input_ready {
    if !any_streaming && !ready_inputs.is_empty() {
        forward_bundle();  // Forward when all streaming done
    }
}
```

#### 中文

**决策**：实现两条转发路径以处理竞态条件。

**路径1 - 输入事件循环**：
- 触发条件：输入完成 且 桥接器处于 resume_mode
- 动作：检查是否没有其他输入正在流式传输，然后转发

**路径2 - 控制事件循环**：
- 触发条件：收到 "resume" 命令
- 动作：如果存在就绪输入 且 没有进行中的流式传输 → 立即转发
- 否则：保持 resume_mode，让路径1处理

```rust
// 路径2：控制恢复处理器
if !ready_inputs.is_empty() && !any_streaming {
    forward_bundle();  // 立即转发
} else {
    // 保持 resume_mode，路径1 将在流式传输完成时转发
}

// 路径1：输入完成处理器
if bridge.resume_mode && input_ready {
    if !any_streaming && !ready_inputs.is_empty() {
        forward_bundle();  // 当所有流式传输完成时转发
    }
}
```

---

## Race Condition Handling / 竞态条件处理

### Problem Statement / 问题描述

```mermaid
sequenceDiagram
    participant J as Judge (Streaming)
    participant Ctrl as Controller
    participant B as Bridge

    J->>B: chunk 1
    J->>B: chunk 2
    J->>B: chunk 3
    J->>Ctrl: completion signal

    Note over Ctrl: Determines next speaker
    Ctrl->>B: resume

    Note over B: Race Condition!<br/>Did we process judge's<br/>completion before resume?

    J->>B: completion signal (may arrive after resume!)
```

### English

**The Race Condition Problem**:

When the controller receives a completion signal from a participant, it immediately determines the next speaker and sends a "resume" to the appropriate bridge. However, due to Dora's event scheduling, the bridge might receive the "resume" command BEFORE it has processed the participant's completion signal.

**Scenarios**:

| Scenario | Resume Arrives | Streaming State | Action |
|----------|---------------|-----------------|--------|
| 1 | After completion processed | `any_streaming=false` | Forward immediately (Path 2) |
| 2 | Before completion processed | `any_streaming=true` | Stay in resume_mode, wait for Path 1 |
| 3 | During streaming | `any_streaming=true` | Stay in resume_mode, wait for Path 1 |

**Solution - Two-Path Forwarding**:

1. **Path 2 (Resume Handler)**: Checks if forwarding is safe
   - If `ready_inputs` exist AND `!any_streaming` → Forward now
   - Otherwise → Set `resume_mode=true`, defer to Path 1

2. **Path 1 (Input Handler)**: Handles deferred forwarding
   - If `resume_mode=true` AND input just completed AND `!any_streaming` → Forward now

**Key Invariant**: `resume_mode` is only set to `false` AFTER successful forwarding.

### 中文

**竞态条件问题**：

当控制器收到参与者的完成信号时，它立即确定下一个发言者并向适当的桥接器发送 "resume"。然而，由于 Dora 的事件调度，桥接器可能在处理参与者的完成信号之前收到 "resume" 命令。

**场景**：

| 场景 | Resume 到达时机 | 流式传输状态 | 动作 |
|-----|----------------|------------|------|
| 1 | 完成处理之后 | `any_streaming=false` | 立即转发（路径2） |
| 2 | 完成处理之前 | `any_streaming=true` | 保持 resume_mode，等待路径1 |
| 3 | 流式传输期间 | `any_streaming=true` | 保持 resume_mode，等待路径1 |

**解决方案 - 双路径转发**：

1. **路径2（Resume 处理器）**：检查转发是否安全
   - 如果存在 `ready_inputs` 且 `!any_streaming` → 立即转发
   - 否则 → 设置 `resume_mode=true`，交给路径1

2. **路径1（输入处理器）**：处理延迟转发
   - 如果 `resume_mode=true` 且 输入刚完成 且 `!any_streaming` → 立即转发

**关键不变量**：`resume_mode` 只在成功转发后才设置为 `false`。

### Event Queue Configuration / 事件队列配置

```yaml
# Increase queue size to prevent control messages from being dropped
control:
  source: conference-controller/control_llm1
  queue_size: 10  # Default is 1, which can cause message loss
```

---

## Node Specifications / 节点规格

### Conference Controller / 会议控制器

```mermaid
flowchart LR
    subgraph Inputs
        I1[llm1/text]
        I2[llm2/text]
        I3[judge/text]
    end

    subgraph Controller["Conference Controller"]
        P[Policy Engine]
        S[State Machine]
    end

    subgraph Outputs
        O1[control_llm1]
        O2[control_llm2]
        O3[control_judge]
        O4[status]
    end

    I1 --> Controller
    I2 --> Controller
    I3 --> Controller
    Controller --> O1
    Controller --> O2
    Controller --> O3
    Controller --> O4
```

| Property | Value |
|----------|-------|
| **Binary** | `dora-conference-controller` |
| **Language** | Rust |
| **Purpose** | Determine speaking order based on policy |

**Inputs**:
| Input | Source | Description |
|-------|--------|-------------|
| `llm1` | `llm1/text` | Text output from LLM1 |
| `llm2` | `llm2/text` | Text output from LLM2 |
| `judge` | `judge/text` | Text output from Judge |

**Outputs**:
| Output | Description |
|--------|-------------|
| `control_llm1` | Resume signal to bridge-to-llm1 |
| `control_llm2` | Resume signal to bridge-to-llm2 |
| `control_judge` | Resume signal to bridge-to-judge |
| `status` | Policy statistics (JSON) |

**Environment Variables**:
| Variable | Description | Example |
|----------|-------------|---------|
| `DORA_POLICY_PATTERN` | Speaking order pattern | `[llm1 → llm2 → judge]` |
| `LOG_LEVEL` | Logging verbosity | `INFO` |

### Conference Bridge / 会议桥接器

```mermaid
flowchart LR
    subgraph Inputs
        I1[participant1/text]
        I2[participant2/text]
        IC[control]
    end

    subgraph Bridge["Conference Bridge"]
        BUF[Message Buffer]
        FWD[Forward Logic]
        MODE[Resume Mode]
    end

    subgraph Outputs
        O1[text]
        O2[status]
        O3[log]
    end

    I1 --> Bridge
    I2 --> Bridge
    IC --> Bridge
    Bridge --> O1
    Bridge --> O2
    Bridge --> O3
```

| Property | Value |
|----------|-------|
| **Binary** | `dora-conference-bridge` |
| **Language** | Rust |
| **Purpose** | Buffer and forward messages on command |

**Inputs**:
| Input | Source | Description |
|-------|--------|-------------|
| `<participant>` | `<participant>/text` | Text from other participants |
| `control` | `controller/control_<target>` | Control commands (resume/reset) |

**Outputs**:
| Output | Description |
|--------|-------------|
| `text` | Forwarded message bundle |
| `status` | Bridge status (waiting/forwarded/reset) |
| `log` | Debug logs (JSON) |

**Environment Variables**:
| Variable | Description | Example |
|----------|-------------|---------|
| `STREAMING_PORTS` | Comma-separated streaming input names | `llm1,llm2,judge` |
| `LOG_LEVEL` | Logging verbosity | `INFO` |
| `INC_QUESTION_ID` | Auto-increment question ID | `false` |

**Control Commands**:
| Command | Action |
|---------|--------|
| `resume` | Enter resume mode, forward ready inputs |
| `reset` | Clear all buffers, reset state |

---

## Dataflow Configuration / 数据流配置

### Sequential Debate Dataflow / 顺序辩论数据流

```yaml
# dataflow-debate-sequential.yml
# Conference Debate Example - Sequential Policy (3-Bridge Architecture)
#
# Implements a 3-person debate using conference controller and bridge
# with sequential policy: llm1 → llm2 → judge → repeat

nodes:
  # ============ LLM Participants (MaaS) ============
  - id: llm1
    path: ../../target/release/dora-maas-client
    inputs:
      text: bridge-to-llm1/text
    outputs:
      - text
      - status
      - log
    env:
      MAAS_CONFIG_PATH: debate_config_maas_llm1.toml
      LOG_LEVEL: INFO

  - id: llm2
    path: ../../target/release/dora-maas-client
    inputs:
      text: bridge-to-llm2/text
    outputs:
      - text
      - status
      - log
    env:
      MAAS_CONFIG_PATH: debate_config_maas_llm2.toml
      LOG_LEVEL: INFO

  - id: judge
    path: ../../target/release/dora-maas-client
    inputs:
      text: bridge-to-judge/text
    outputs:
      - text
      - status
      - log
    env:
      MAAS_CONFIG_PATH: debate_config_maas_judge.toml
      LOG_LEVEL: INFO

  # ============ 3 Conference Bridges (Switch) ============
  # Bridge 1: LLM2 + Judge → LLM1
  - id: bridge-to-llm1
    path: ../../target/release/dora-conference-bridge
    env:
      LOG_LEVEL: INFO
      STREAMING_PORTS: llm1,judge,llm2
    inputs:
      llm2: llm2/text
      judge: judge/text
      control:
        source: conference-controller/control_llm1
        queue_size: 10
    outputs:
      - text
      - status
      - log

  # Bridge 2: LLM1 + Judge → LLM2
  - id: bridge-to-llm2
    path: ../../target/release/dora-conference-bridge
    env:
      LOG_LEVEL: INFO
      STREAMING_PORTS: llm1,judge,llm2
    inputs:
      llm1: llm1/text
      judge: judge/text
      control:
        source: conference-controller/control_llm2
        queue_size: 10
    outputs:
      - text
      - status
      - log

  # Bridge 3: LLM1 + LLM2 → Judge
  - id: bridge-to-judge
    path: ../../target/release/dora-conference-bridge
    env:
      LOG_LEVEL: INFO
      STREAMING_PORTS: llm1,judge,llm2
    inputs:
      llm1: llm1/text
      llm2: llm2/text
      control:
        source: conference-controller/control_judge
        queue_size: 10
    outputs:
      - text
      - status
      - log

  # ============ Conference Controller (Brain) ============
  - id: conference-controller
    path: ../../target/release/dora-conference-controller
    env:
      DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
    inputs:
      llm1: llm1/text
      llm2: llm2/text
      judge: judge/text
    outputs:
      - control_judge
      - control_llm2
      - control_llm1
      - status
```

---

## Challenges & Solutions / 挑战与解决方案

### Challenge 1: Event Ordering in Distributed Systems / 分布式系统中的事件排序

#### English

**Problem**: Dora's event scheduler uses LRU fairness to interleave events from different inputs. This can cause control messages to be processed out of order relative to data messages.

**Solution**:
- Increase `queue_size` for control inputs to prevent message loss
- Implement two-path forwarding to handle timing variations
- Use `resume_mode` flag to maintain state across event processing

#### 中文

**问题**：Dora 的事件调度器使用 LRU 公平性来交错处理来自不同输入的事件。这可能导致控制消息相对于数据消息乱序处理。

**解决方案**：
- 增加控制输入的 `queue_size` 以防止消息丢失
- 实现双路径转发以处理时序变化
- 使用 `resume_mode` 标志在事件处理之间维护状态

### Challenge 2: Streaming Message Completion Detection / 流式消息完成检测

#### English

**Problem**: LLM outputs are streamed in chunks. We need to know when a complete message has been received before forwarding.

**Solution**:
- Use metadata signals (`session_status: "ended"` or `is_complete: true`)
- Maintain message state machine: `None → Streaming → Complete`
- Only forward when message state is `Complete`

#### 中文

**问题**：LLM 输出以块的形式流式传输。我们需要知道何时收到完整消息才能转发。

**解决方案**：
- 使用元数据信号（`session_status: "ended"` 或 `is_complete: true`）
- 维护消息状态机：`None → Streaming → Complete`
- 仅在消息状态为 `Complete` 时转发

### Challenge 3: Partial Participant Availability / 部分参与者可用性

#### English

**Problem**: In sequential debates, not all participants speak every turn. Bridge may receive messages from only a subset of its configured inputs.

**Solution**:
- Forward when ANY ready inputs exist (not waiting for ALL)
- Check `!ready_inputs.is_empty()` instead of `all_complete`
- Allow partial message bundles

**Before (Incorrect)**:
```rust
let all_complete = bridge.expected_ports.iter().all(|port| ...);
if all_complete { forward(); }
```

**After (Correct)**:
```rust
if !ready_inputs.is_empty() && !any_streaming {
    forward();  // Forward whatever is ready
}
```

#### 中文

**问题**：在顺序辩论中，并非所有参与者每轮都发言。桥接器可能只收到其配置输入的子集的消息。

**解决方案**：
- 当存在任何就绪输入时转发（不等待所有输入）
- 检查 `!ready_inputs.is_empty()` 而不是 `all_complete`
- 允许部分消息包

**之前（错误）**：
```rust
let all_complete = bridge.expected_ports.iter().all(|port| ...);
if all_complete { forward(); }
```

**之后（正确）**：
```rust
if !ready_inputs.is_empty() && !any_streaming {
    forward();  // 转发任何就绪的内容
}
```

### Challenge 4: Synchronous Event Loop Blocking / 同步事件循环阻塞

#### English

**Problem**: Bridge uses synchronous `events.recv()` which blocks until an event arrives. While processing one event, other events queue up.

**Impact**:
- Control messages may be delayed while processing long streaming inputs
- No parallel event processing

**Mitigation**:
- Keep event handlers fast (no blocking I/O)
- Use `queue_size` to buffer events during processing
- Process control events with priority (check `port_name == "control"` first)

#### 中文

**问题**：桥接器使用同步的 `events.recv()` 阻塞直到事件到达。在处理一个事件时，其他事件排队等待。

**影响**：
- 处理长流式输入时，控制消息可能被延迟
- 没有并行事件处理

**缓解措施**：
- 保持事件处理器快速（无阻塞 I/O）
- 使用 `queue_size` 在处理期间缓冲事件
- 优先处理控制事件（首先检查 `port_name == "control"`）

---

## Debugging Guide / 调试指南

### Key Debug Points / 关键调试点

```rust
// Event received (top of loop)
println!("[BRIDGE-STDOUT] ⚡ EVENT RECEIVED: Input from '{}'", id);

// Control command processing
println!("[BRIDGE-STDOUT] 🎮 CONTROL PAYLOAD: '{}' (length: {})", trimmed, trimmed.len());

// Resume state check
println!("[BRIDGE-STDOUT] 🚀 RESUME: ready_inputs={:?}, any_streaming={}", ready_inputs, any_streaming);

// Input completion in resume mode
println!("[BRIDGE-STDOUT] 📥 INPUT COMPLETE in resume_mode: port={}, any_streaming={}", port_name, any_streaming);

// Forwarding
println!("[BRIDGE-STDOUT] 🚀 FORWARDING: {} ready inputs", ready_inputs.len());
```

### Common Issues / 常见问题

| Symptom | Cause | Solution |
|---------|-------|----------|
| Resume not received | Queue overflow | Increase `queue_size: 10` |
| Resume received but not forwarding | `all_complete` check | Use `!ready_inputs.is_empty()` |
| Forwarding during streaming | Missing `any_streaming` check | Add `&& !any_streaming` |
| Duplicate forwards | `resume_mode` not reset | Only reset after successful forward |

### Log Analysis / 日志分析

**Successful flow should show**:
```
⚡ EVENT RECEIVED: Input from 'judge'
📝 TEXT INPUT PROCESSING: port=judge
📥 RAW INPUT RECEIVED from judge: '...'
...
⚡ EVENT RECEIVED: Input from 'control'
🎮 CONTROL PAYLOAD: 'resume'
🚀 RESUME: ready_inputs={"judge"}, any_streaming=false
🚀 RESUME: READY TO FORWARD
🚀 forward_bundle() called!
✅ forward_bundle() completed successfully
```

---

## Version History / 版本历史

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2024-01 | Initial design with single bridge |
| 2.0 | 2024-02 | Three-bridge architecture |
| 2.1 | 2024-03 | Two-path forwarding for race condition handling |
| 2.2 | 2024-03 | Fixed `all_complete` to `!ready_inputs.is_empty()` |
| 2.3 | 2024-11 | Signal classification system (Reset/Cancelled/Error handling) |
| 2.4 | 2024-11 | Reset state recovery fix - detect `session_status: "started"` |
| 2.5 | 2024-11 | Error message template support for participant failures |

---

## References / 参考资料

- Dora Framework Documentation: https://dora.carsmos.ai/
- Conference Bridge Source: `node-hub/dora-conference-bridge/src/main.rs`
- Conference Controller Source: `node-hub/dora-conference-controller/src/main.rs`
- Example Dataflow: `examples/conference/dataflow-debate-sequential.yml`
