# MoFA-Dora Integration Plan

## Overview

Replace the monolithic conference-dashboard approach with a modular, fine-grained architecture where:
- Individual widgets (mic, speaker, log viewer, chat) connect as separate dynamic nodes
- Each MoFA app corresponds to one dataflow
- Dataflows are parsed to auto-generate UI components and env configuration

## Current Architecture Problems

| Problem | Impact |
|---------|--------|
| Hardcoded 3-participant node names | Cannot dynamically add/remove participants |
| Single monolithic dora_bridge.rs (1300 lines) | All logic coupled, hard to extend |
| Static input/output naming convention | Renaming requires code changes |
| Dashboard is single dynamic node | All widgets share one connection |
| Env vars scattered across nodes | No central configuration |
| Log aggregation in single vector | No per-node filtering |

## New Architecture Vision

```
┌─────────────────────────────────────────────────────────────────┐
│                        MoFA Studio Shell                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  MoFA App   │  │  MoFA App   │  │  MoFA App   │              │
│  │  (Voice)    │  │  (Agent)    │  │  (Custom)   │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────┴──────┐              │
│  │ Dora Bridge │  │ Dora Bridge │  │ Dora Bridge │              │
│  │ Dispatcher  │  │ Dispatcher  │  │ Dispatcher  │              │
│  └──────┬──────┘  └──────┴──────┘  └──────┬──────┘              │
└─────────┼───────────────────────────────────┼───────────────────┘
          │                                   │
    ┌─────┴─────┐                       ┌─────┴─────┐
    │ Dataflow  │                       │ Dataflow  │
    │  (voice   │                       │  (agent   │
    │   chat)   │                       │   task)   │
    └───────────┘                       └───────────┘
```

---

## Phase 1: Core Infrastructure

### P1.1 - Dora Bridge Abstraction Layer
**Goal**: Create reusable bridge traits and types for widget-to-dora communication

- [ ] **Create `mofa-dora-bridge` crate** in `mofa-studio/mofa-dora-bridge/`
  - [ ] `Cargo.toml` with dora-node-api, tokio, serde dependencies
  - [ ] `src/lib.rs` - public exports

- [ ] **Define `DoraBridge` trait**
  ```rust
  pub trait DoraBridge: Send + Sync {
      fn node_id(&self) -> &str;
      fn connect(&mut self, dataflow_id: &str) -> Result<()>;
      fn disconnect(&mut self) -> Result<()>;
      fn is_connected(&self) -> bool;
      fn send(&self, output_id: &str, data: DoraData) -> Result<()>;
      fn subscribe(&mut self, input_id: &str, handler: InputHandler) -> Result<()>;
  }
  ```

- [ ] **Define `DoraData` enum** for typed data exchange
  ```rust
  pub enum DoraData {
      Audio(AudioData),      // sample_rate, channels, samples
      Text(String),
      Json(serde_json::Value),
      Binary(Vec<u8>),
      Control(ControlCommand),
      Log(LogEntry),
  }
  ```

- [ ] **Define `InputHandler` callback type**
  ```rust
  pub type InputHandler = Box<dyn Fn(DoraData, Metadata) + Send + Sync>;
  ```

- [ ] **Implement `DynamicNodeBridge`** - connects as dora dynamic node
  - [ ] `new(node_id: &str)` constructor
  - [ ] Background tokio task for event loop
  - [ ] Input routing to registered handlers
  - [ ] Output queue with backpressure

- [ ] **Unit tests** for bridge trait and data types

### P1.2 - Widget Bridge Protocol
**Goal**: Define how widgets communicate with dora bridges

- [ ] **Create `WidgetBridgePort` struct** - widget-side connection point
  ```rust
  pub struct WidgetBridgePort {
      pub port_id: String,
      pub direction: PortDirection,  // Input, Output, Bidirectional
      pub data_type: DoraDataType,
      sender: Option<Sender<DoraData>>,
      receiver: Option<Receiver<DoraData>>,
  }
  ```

- [ ] **Create `BridgeRegistry`** - manages all widget-bridge connections
  ```rust
  pub struct BridgeRegistry {
      bridges: HashMap<String, Arc<dyn DoraBridge>>,
      port_mappings: HashMap<String, PortMapping>,
  }
  ```

- [ ] **Define port mapping configuration**
  ```rust
  pub struct PortMapping {
      pub widget_port: String,      // e.g., "mic_input.audio"
      pub dora_node: String,        // e.g., "mic_bridge"
      pub dora_output: String,      // e.g., "audio"
  }
  ```

- [ ] **Implement thread-safe message passing** using crossbeam channels

### P1.3 - Dataflow Parser
**Goal**: Parse YAML dataflows to extract node info, env vars, and generate UI hints

- [ ] **Create `DataflowParser` struct**
  ```rust
  pub struct DataflowParser {
      pub dataflow_path: PathBuf,
      pub parsed: Option<ParsedDataflow>,
  }
  ```

- [ ] **Define `ParsedDataflow` structure**
  ```rust
  pub struct ParsedDataflow {
      pub id: String,
      pub nodes: Vec<ParsedNode>,
      pub dynamic_nodes: Vec<DynamicNodeSpec>,
      pub env_requirements: Vec<EnvRequirement>,
      pub log_sources: Vec<LogSource>,
  }
  ```

- [ ] **Define `ParsedNode` structure**
  ```rust
  pub struct ParsedNode {
      pub id: String,
      pub node_type: NodeType,        // Operator, Custom, Dynamic
      pub inputs: Vec<InputDef>,
      pub outputs: Vec<OutputDef>,
      pub env_vars: HashMap<String, EnvVarDef>,
      pub metadata: HashMap<String, String>,
  }
  ```

- [ ] **Define `EnvRequirement` for UI configuration**
  ```rust
  pub struct EnvRequirement {
      pub key: String,
      pub description: String,
      pub required: bool,
      pub default: Option<String>,
      pub secret: bool,              // API keys should be masked
      pub node_ids: Vec<String>,     // Which nodes use this var
  }
  ```

- [ ] **Define `LogSource` for log viewer generation**
  ```rust
  pub struct LogSource {
      pub node_id: String,
      pub output_id: String,
      pub log_level: LogLevel,
      pub display_name: String,
  }
  ```

- [ ] **Implement YAML parsing** using serde_yaml
  - [ ] Parse nodes array
  - [ ] Extract env sections
  - [ ] Identify dynamic nodes (path: dynamic)
  - [ ] Detect log outputs by naming convention (*_log, *_status)
  - [ ] Extract input/output connections

- [ ] **Add dataflow validation**
  - [ ] Check for missing env vars
  - [ ] Validate dynamic node references
  - [ ] Warn about unconnected outputs

---

## Phase 2: Widget Bridges

### P2.1 - Mic Input Bridge
**Goal**: Connect mic widget to dora as dynamic audio source node

- [ ] **Create `MicBridge` in `mofa-dora-bridge/src/widgets/mic.rs`**
  ```rust
  pub struct MicBridge {
      node_id: String,
      sample_rate: u32,
      channels: u16,
      audio_sender: Sender<AudioData>,
  }
  ```

- [ ] **Implement audio capture to dora output**
  - [ ] Connect to cpal audio input
  - [ ] Convert to Float32 samples
  - [ ] Send via `audio` output
  - [ ] Handle sample rate conversion if needed

- [ ] **Add VAD (Voice Activity Detection) output**
  - [ ] Send `speech_started` / `speech_ended` events
  - [ ] Configurable VAD threshold

- [ ] **Implement `DoraBridge` trait for MicBridge**

### P2.2 - Speaker Output Bridge
**Goal**: Connect speaker widget to dora as dynamic audio sink node

- [ ] **Create `SpeakerBridge` in `mofa-dora-bridge/src/widgets/speaker.rs`**
  ```rust
  pub struct SpeakerBridge {
      node_id: String,
      audio_receiver: Receiver<AudioData>,
      buffer: CircularBuffer<f32>,
  }
  ```

- [ ] **Implement audio playback from dora input**
  - [ ] Subscribe to `audio` input
  - [ ] Buffer incoming audio chunks
  - [ ] Feed to cpal audio output
  - [ ] Handle buffer underrun/overrun

- [ ] **Add buffer status output**
  - [ ] Send `buffer_status` (fill percentage)
  - [ ] Send `playback_started` / `playback_stopped`

- [ ] **Implement `DoraBridge` trait for SpeakerBridge**

### P2.3 - System Log Bridge
**Goal**: Connect system log widget to dora log streams

- [ ] **Create `LogBridge` in `mofa-dora-bridge/src/widgets/log.rs`**
  ```rust
  pub struct LogBridge {
      node_id: String,
      log_sources: Vec<LogSource>,
      log_receiver: Receiver<LogEntry>,
      filter_level: LogLevel,
  }
  ```

- [ ] **Implement multi-source log aggregation**
  - [ ] Subscribe to multiple `*_log` inputs
  - [ ] Parse JSON log format
  - [ ] Filter by log level
  - [ ] Add source node identification

- [ ] **Support dynamic log source registration**
  - [ ] Add/remove log sources at runtime
  - [ ] Per-source log level filtering

- [ ] **Implement `DoraBridge` trait for LogBridge**

### P2.4 - Chat Viewer Bridge
**Goal**: Connect chat widget to dora text/message streams

- [ ] **Create `ChatBridge` in `mofa-dora-bridge/src/widgets/chat.rs`**
  ```rust
  pub struct ChatBridge {
      node_id: String,
      participants: HashMap<String, ParticipantInfo>,
      message_receiver: Receiver<ChatMessage>,
  }
  ```

- [ ] **Implement text stream handling**
  - [ ] Subscribe to `*_text` inputs
  - [ ] Handle streaming text (partial messages)
  - [ ] Detect message boundaries
  - [ ] Track participant metadata

- [ ] **Support bidirectional chat**
  - [ ] `user_input` output for user messages
  - [ ] `control` output for chat commands

- [ ] **Implement `DoraBridge` trait for ChatBridge**

---

## Phase 3: Dataflow Lifecycle Management

### P3.1 - Dataflow Controller
**Goal**: Manage dataflow lifecycle (start, stop, status)

- [ ] **Create `DataflowController` in `mofa-dora-bridge/src/controller.rs`**
  ```rust
  pub struct DataflowController {
      dataflow_path: PathBuf,
      dataflow_id: Option<String>,
      state: DataflowState,
      dora_process: Option<Child>,
  }
  ```

- [ ] **Define `DataflowState` enum**
  ```rust
  pub enum DataflowState {
      Stopped,
      Starting,
      Running { started_at: Instant },
      Stopping,
      Error { message: String },
  }
  ```

- [ ] **Implement start_dataflow()**
  - [ ] Ensure dora daemon running
  - [ ] Validate dataflow YAML
  - [ ] Apply env var configuration
  - [ ] Execute `dora start --detach`
  - [ ] Wait for dataflow ready

- [ ] **Implement stop_dataflow()**
  - [ ] Disconnect all widget bridges
  - [ ] Execute `dora stop`
  - [ ] Cleanup resources
  - [ ] Reset state

- [ ] **Implement get_status()**
  - [ ] Query dora daemon for dataflow status
  - [ ] Return node health info
  - [ ] Detect failed nodes

### P3.2 - Dynamic Node Dispatcher
**Goal**: Manage dynamic node connections for widgets

- [ ] **Create `DynamicNodeDispatcher` struct**
  ```rust
  pub struct DynamicNodeDispatcher {
      controller: Arc<DataflowController>,
      bridges: HashMap<String, Arc<dyn DoraBridge>>,
      connection_state: HashMap<String, ConnectionState>,
  }
  ```

- [ ] **Implement connect_widget()**
  - [ ] Create appropriate bridge for widget type
  - [ ] Register as dynamic node with dora
  - [ ] Set up input/output mappings
  - [ ] Return bridge handle

- [ ] **Implement disconnect_widget()**
  - [ ] Gracefully disconnect from dora
  - [ ] Cleanup bridge resources
  - [ ] Update connection state

- [ ] **Implement reconnect_all()**
  - [ ] Reconnect all widgets after dataflow restart
  - [ ] Preserve widget state

### P3.3 - Resource Cleanup
**Goal**: Ensure proper resource release on dataflow stop

- [ ] **Implement `Drop` trait for all bridges**
  - [ ] Close dora connections
  - [ ] Stop background threads
  - [ ] Release audio devices

- [ ] **Add cleanup hooks to DataflowController**
  - [ ] Pre-stop hook (notify widgets)
  - [ ] Post-stop hook (cleanup resources)

- [ ] **Handle abnormal termination**
  - [ ] Detect dora daemon crash
  - [ ] Auto-reconnect or notify user
  - [ ] Prevent resource leaks

---

## Phase 4: UI Integration

### P4.1 - Dataflow Configuration Panel
**Goal**: UI for configuring dataflow env vars before start

- [ ] **Create `DataflowConfigPanel` widget**
  - [ ] Display parsed env requirements
  - [ ] Text input for each env var
  - [ ] Password field for secrets (API keys)
  - [ ] Validation indicators

- [ ] **Implement env var persistence**
  - [ ] Save to preferences file
  - [ ] Load defaults from dataflow YAML
  - [ ] Secure storage for API keys

- [ ] **Add "Start Dataflow" button**
  - [ ] Validate all required vars set
  - [ ] Show progress indicator
  - [ ] Display error on failure

### P4.2 - Dynamic Log Viewer Generation
**Goal**: Auto-generate log tabs/filters from parsed dataflow

- [ ] **Extend `LogBridge` with dynamic source info**
  - [ ] Parse log sources from dataflow
  - [ ] Generate tab for each source
  - [ ] Color-code by node type

- [ ] **Create `LogViewerConfig` from ParsedDataflow**
  ```rust
  pub struct LogViewerConfig {
      pub sources: Vec<LogSourceConfig>,
      pub default_level: LogLevel,
      pub show_timestamps: bool,
  }
  ```

- [ ] **Implement log source filtering UI**
  - [ ] Checkbox per source
  - [ ] Log level dropdown
  - [ ] Search/filter text

### P4.3 - Dataflow Status Dashboard
**Goal**: Show real-time dataflow and node status

- [ ] **Create `DataflowStatusWidget`**
  - [ ] Dataflow state indicator (running/stopped/error)
  - [ ] Node health list
  - [ ] Connection status per widget

- [ ] **Add node graph visualization** (future)
  - [ ] Show nodes and connections
  - [ ] Highlight active data flow
  - [ ] Click to focus node logs

---

## Phase 5: MoFA App Integration

### P5.1 - MoFA App Dataflow Binding
**Goal**: Each MoFA app declares its associated dataflow

- [ ] **Add `DataflowBinding` to MoFA app manifest**
  ```rust
  pub struct DataflowBinding {
      pub dataflow_path: PathBuf,
      pub widget_ports: Vec<WidgetPortBinding>,
      pub required_env: Vec<String>,
  }
  ```

- [ ] **Define `WidgetPortBinding`**
  ```rust
  pub struct WidgetPortBinding {
      pub widget_id: String,        // e.g., "mic_input"
      pub bridge_type: BridgeType,  // Mic, Speaker, Log, Chat
      pub dora_node_id: String,     // Dynamic node ID in dataflow
  }
  ```

- [ ] **Implement app-to-dataflow resolution**
  - [ ] Locate dataflow YAML for app
  - [ ] Validate widget bindings match dataflow

### P5.2 - App Lifecycle Hooks
**Goal**: Connect app lifecycle to dataflow lifecycle

- [ ] **Add `on_app_start` hook**
  - [ ] Parse and validate dataflow
  - [ ] Show config panel if env vars needed
  - [ ] Optionally auto-start dataflow

- [ ] **Add `on_app_stop` hook**
  - [ ] Stop dataflow
  - [ ] Disconnect all bridges
  - [ ] Cleanup resources

- [ ] **Add `on_app_focus` hook**
  - [ ] Resume paused dataflow
  - [ ] Reconnect if disconnected

### P5.3 - Multi-App Dataflow Isolation
**Goal**: Support multiple apps with separate dataflows

- [ ] **Implement dataflow namespacing**
  - [ ] Unique dataflow ID per app instance
  - [ ] Isolated dynamic node IDs

- [ ] **Handle dataflow conflicts**
  - [ ] Prevent same dataflow started twice
  - [ ] Warn on resource conflicts (audio devices)

---

## Phase 6: Advanced Features

### P6.1 - Hot Reload Support
**Goal**: Update dataflow without full restart

- [ ] **Detect dataflow YAML changes**
  - [ ] File watcher on dataflow path
  - [ ] Compare with running config

- [ ] **Implement partial reload**
  - [ ] Identify changed nodes
  - [ ] Restart only affected nodes
  - [ ] Preserve widget connections

### P6.2 - Dataflow Templates
**Goal**: Generate dataflows from templates with variable substitution

- [ ] **Create template syntax**
  ```yaml
  nodes:
    - id: "{{participant_id}}_tts"
      operator:
        python: ../../node-hub/dora-primespeech
      env:
        VOICE_NAME: "{{voice_name}}"
  ```

- [ ] **Implement template rendering**
  - [ ] Variable substitution
  - [ ] Conditional sections
  - [ ] Loop expansion (for participants)

### P6.3 - Remote Dataflow Support
**Goal**: Connect to dataflows running on remote machines

- [ ] **Implement remote bridge transport**
  - [ ] WebSocket connection to remote dora
  - [ ] Secure authentication
  - [ ] Handle network latency

- [ ] **Add remote dataflow discovery**
  - [ ] Scan network for dora instances
  - [ ] List available dataflows

---

## File Structure

```
mofa-studio/
├── mofa-dora-bridge/                    # NEW: Bridge crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                       # Public exports
│       ├── bridge.rs                    # DoraBridge trait
│       ├── data.rs                      # DoraData types
│       ├── parser.rs                    # DataflowParser
│       ├── controller.rs                # DataflowController
│       ├── dispatcher.rs                # DynamicNodeDispatcher
│       ├── registry.rs                  # BridgeRegistry
│       └── widgets/
│           ├── mod.rs
│           ├── mic.rs                   # MicBridge
│           ├── speaker.rs               # SpeakerBridge
│           ├── log.rs                   # LogBridge
│           └── chat.rs                  # ChatBridge
│
├── mofa-widgets/
│   └── src/
│       ├── dataflow_config_panel.rs     # NEW: Env config UI
│       ├── dataflow_status.rs           # NEW: Status dashboard
│       └── log_viewer.rs                # MODIFY: Dynamic sources
│
├── mofa-studio-shell/
│   └── src/
│       └── app.rs                       # MODIFY: Bridge integration
│
└── apps/
    └── mofa-fm/
        ├── dataflow/                    # NEW: App dataflows
        │   └── voice-chat.yml
        └── src/
            └── screen.rs                # MODIFY: Widget bindings
```

---

## Migration Path

### Step 1: Create mofa-dora-bridge crate
- Implement core traits and types
- No changes to existing code

### Step 2: Implement MicBridge and SpeakerBridge
- Test with simple dataflow
- Validate audio quality

### Step 3: Implement LogBridge
- Parse existing log format
- Verify backward compatibility

### Step 4: Add DataflowController
- Test start/stop lifecycle
- Validate resource cleanup

### Step 5: Integrate with mofa-fm
- Replace conference-dashboard bridge
- Migrate one widget at a time

### Step 6: Add UI components
- DataflowConfigPanel
- DataflowStatusWidget

---

## Implementation Notes

### Signal Flow for Conference Controller Integration

The MoFA Studio bridges must send specific signals to integrate with the `conference-controller` node. This section documents the required signals and their timing.

#### Audio Player Bridge (`mofa-audio-player`)

**Outputs:**
- `session_start` - Signals that audio playback has begun for a participant
- `audio_complete` - Signals that an audio chunk was received (flow control)
- `buffer_status` - Audio buffer fill percentage (backpressure)
- `status` - General status updates
- `log` - Debug logging

**Critical: `session_start` Signal**

The conference-controller uses `session_start` to know when to advance to the next speaker:

```
Controller: Resume student2 (question_id=32)
Controller: waiting_for_session_start = Some(32)
Audio arrives with session_status="started"
Audio Player: sends session_start with question_id=32
Controller: receives session_start, advances to next speaker
```

**Implementation (audio_player.rs):**
```rust
// Send session_start ONLY when session_status is "started"
// This marks the first chunk of a new LLM/TTS response
if session_status.map(|s| s == "started").unwrap_or(false) {
    let mut params: BTreeMap<String, Parameter> = BTreeMap::new();
    params.insert("question_id".to_string(), Parameter::String(qid.to_string()));
    params.insert("participant".to_string(), Parameter::String(participant.to_string()));
    params.insert("source".to_string(), Parameter::String("mofa-audio-player".to_string()));

    node.send_output(
        DataId::from("session_start".to_string()),
        params,
        vec!["audio_started".to_string()].into_arrow(),
    )?;
}
```

**Common Mistake:** Sending `session_start` for every audio chunk floods the controller and causes timing issues. Only send when `session_status == "started"`.

**Critical: `audio_complete` Signal**

The text-segmenter waits for `audio_complete` before releasing the next text segment:

```
TTS sends audio chunk → mofa-audio-player receives
mofa-audio-player sends audio_complete → text-segmenter
text-segmenter releases next segment → TTS
```

**Implementation (audio_player.rs):**
```rust
// Send after receiving each audio chunk
let mut params: BTreeMap<String, Parameter> = BTreeMap::new();
params.insert("participant".to_string(), Parameter::String(participant.to_string()));
if let Some(qid) = metadata.get("question_id") {
    params.insert("question_id".to_string(), Parameter::String(qid.to_string()));
}
if let Some(status) = metadata.get("session_status") {
    params.insert("session_status".to_string(), Parameter::String(status.to_string()));
}

node.send_output(
    DataId::from("audio_complete".to_string()),
    params,
    vec!["received".to_string()].into_arrow(),
)?;
```

#### Participant Panel Bridge (`mofa-participant-panel`)

**Active Speaker Tracking:**

Only ONE participant should be shown as "active" at a time - the one currently speaking.

**Implementation (participant_panel.rs):**
```rust
// Track active participant - only ONE can be active at a time
let mut active_participant: Option<String> = None;

// When session_status == "started", switch active speaker
if session_status.map(|s| s == "started").unwrap_or(false) {
    if active_participant.as_ref() != Some(&participant_id) {
        // Deactivate previous speaker
        if let Some(ref prev) = active_participant {
            let deactivate = ParticipantAudioData {
                participant_id: prev.clone(),
                audio_level: 0.0,
                bands: [0.0; 8],
                is_active: false,
            };
            audio_data_sender.send(deactivate)?;
        }
        *active_participant = Some(participant_id.clone());
    }
}

// Only show as active if this is the currently active participant
let is_active = active_participant.as_ref() == Some(&participant_id);
```

**Common Mistake:** Setting `is_active: true` for any participant receiving audio causes all LED panels to light up simultaneously.

#### Audio Data Format (Arrow)

PrimeSpeech TTS sends audio as `ListArray<Float32>`:
```python
# Python side
pa.array([audio_array])  # Wraps float32 array in ListArray
```

**Extraction (Rust side):**
```rust
// Handle ListArray<Float32> - primespeech sends pa.array([audio_array])
DataType::List(_) | DataType::LargeList(_) => {
    if let Some(list_arr) = array.as_any().downcast_ref::<ListArray>() {
        if list_arr.len() > 0 {
            let first_value = list_arr.value(0);
            if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>() {
                return Some(float_arr.values().to_vec());
            }
        }
    }
}
```

### Dataflow Controller Integration

**Dataflow ID Extraction:**

`dora start --detach` outputs the dataflow ID to **stderr**, not stdout:

```rust
// Parse dataflow ID from output (check both stdout and stderr)
let stdout = String::from_utf8_lossy(&output.stdout);
let stderr = String::from_utf8_lossy(&output.stderr);
let dataflow_id = Self::parse_dataflow_id(&stderr)
    .or_else(|| Self::parse_dataflow_id(&stdout))
    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
```

### Metadata Pass-through

TTS nodes include important metadata that must be preserved:

| Key | Description | Used By |
|-----|-------------|---------|
| `session_status` | "started", "streaming", "complete" | session_start signal, active speaker |
| `question_id` | Enhanced 16-bit ID (RxPy/z format) | Controller flow control |
| `participant` | Speaker identifier | LED panel routing |
| `sample_rate` | Audio sample rate (default 32000) | Audio playback |

### Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Conversation stops after 2 rounds | `session_start` not sent | Check `session_status == "started"` condition |
| All LED panels light up | `is_active: true` for all | Track active participant, only one active |
| Only first word plays | `audio_complete` not sent | Send after each audio chunk |
| Controller floods with session_start | Sending for every chunk | Only send when `session_status == "started"` |
| Dataflow "stopped unexpectedly" | Wrong dataflow ID | Check stderr for ID, not stdout |

---

## Success Criteria

| Criteria | Metric |
|----------|--------|
| Widget independence | Each widget can connect/disconnect independently |
| Dataflow portability | Same widget code works with different dataflows |
| Resource cleanup | No zombie processes after dataflow stop |
| Log flexibility | Logs filtered by source and level |
| Env configuration | API keys configured in UI, not hardcoded |
| Startup time | Dataflow connects within 5 seconds |
| Error handling | Clear error messages for connection failures |

---

## Open Questions

1. **Audio synchronization**: How to sync audio across multiple speaker bridges?
2. **State persistence**: Should widget state survive dataflow restart?
3. **Error recovery**: Auto-reconnect or require manual intervention?
4. **Security**: How to securely store API keys in preferences?
5. **Performance**: Impact of multiple dynamic nodes vs single node?

---

## Timeline Estimate

| Phase | Effort |
|-------|--------|
| P1: Core Infrastructure | Foundation |
| P2: Widget Bridges | Core feature |
| P3: Lifecycle Management | Core feature |
| P4: UI Integration | Enhancement |
| P5: App Integration | Enhancement |
| P6: Advanced Features | Future |

---

## References

- Current architecture: `examples/conference-dashboard/`
- Dora dynamic nodes: `dora-node-api` crate
- Makepad widgets: `mofa-widgets/`
- Existing bridge: `examples/conference-dashboard/src/dora_bridge.rs`
