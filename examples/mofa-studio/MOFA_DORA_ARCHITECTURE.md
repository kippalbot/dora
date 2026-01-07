# MoFA Studio - Dora Integration Architecture

This document describes the architecture of MoFA Studio's integration with the Dora dataflow framework for real-time multi-participant voice conversations.

## System Overview

```mermaid
flowchart TB
    subgraph UI["MoFA Studio UI (Makepad)"]
        direction TB
        MofaHero["MofaHero Widget<br/>- Audio Buffer Gauge<br/>- Connection Status<br/>- Mic/AEC Toggle"]
        ParticipantPanel["Participant Panel<br/>- LED Bars<br/>- Waveform Display<br/>- Active Speaker"]
        PromptInput["Prompt Input<br/>- Text Entry<br/>- Send Button"]
        SystemLog["System Log<br/>- Filtered Logs<br/>- Node Status"]
    end

    subgraph Bridges["MoFA-Dora Bridge Layer"]
        direction TB
        AudioPlayerBridge["AudioPlayerBridge<br/>node: mofa-audio-player"]
        ParticipantPanelBridge["ParticipantPanelBridge<br/>node: mofa-participant-panel"]
        PromptInputBridge["PromptInputBridge<br/>node: mofa-prompt-input"]
        SystemLogBridge["SystemLogBridge<br/>node: mofa-system-log"]
    end

    subgraph AudioPlayback["Audio Playback"]
        CircularBuffer["Circular Buffer<br/>30s @ 32kHz"]
        CpalStream["CPAL Stream<br/>Audio Output"]
    end

    subgraph DoraDataflow["Dora Dataflow"]
        direction TB

        subgraph LLMs["LLM Participants (MaaS)"]
            Student1["student1<br/>dora-maas-client"]
            Student2["student2<br/>dora-maas-client"]
            Tutor["tutor<br/>dora-maas-client"]
        end

        subgraph ConferenceBridges["Conference Bridges"]
            BridgeToS1["bridge-to-student1"]
            BridgeToS2["bridge-to-student2"]
            BridgeToTutor["bridge-to-tutor"]
        end

        Controller["conference-controller<br/>- Turn Management<br/>- Question ID Tracking<br/>- Policy Enforcement"]

        subgraph AudioPipeline["Audio Pipeline"]
            TextSegmenter["multi-text-segmenter<br/>- FIFO Queue<br/>- Sentence Segmentation<br/>- Backpressure Control"]
            TTS_S1["primespeech-student1<br/>Voice: Zhao Daniu"]
            TTS_S2["primespeech-student2<br/>Voice: Chen Yifan"]
            TTS_Tutor["primespeech-tutor<br/>Voice: Luo Xiang"]
        end

        subgraph DynamicNodes["Dynamic Nodes (UI-Connected)"]
            DN_Audio["mofa-audio-player"]
            DN_Panel["mofa-participant-panel"]
            DN_Prompt["mofa-prompt-input"]
            DN_Log["mofa-system-log"]
        end
    end

    %% UI to Bridge connections
    MofaHero <--> AudioPlayerBridge
    ParticipantPanel <--> ParticipantPanelBridge
    PromptInput <--> PromptInputBridge
    SystemLog <--> SystemLogBridge

    %% Bridge to Audio Playback
    AudioPlayerBridge --> CircularBuffer
    CircularBuffer --> CpalStream

    %% Bridge to Dynamic Node connections
    AudioPlayerBridge <-.->|"DoraNode::init_from_node_id"| DN_Audio
    ParticipantPanelBridge <-.->|"DoraNode::init_from_node_id"| DN_Panel
    PromptInputBridge <-.->|"DoraNode::init_from_node_id"| DN_Prompt
    SystemLogBridge <-.->|"DoraNode::init_from_node_id"| DN_Log

    %% Controller Flow
    DN_Prompt -->|"control<br/>(start, reset)"| Controller
    Controller -->|"control_llm1"| BridgeToS1
    Controller -->|"control_llm2"| BridgeToS2
    Controller -->|"control_judge"| BridgeToTutor

    %% LLM Flow
    BridgeToS1 -->|"text"| Student1
    BridgeToS2 -->|"text"| Student2
    BridgeToTutor -->|"text"| Tutor

    Student1 -->|"text"| TextSegmenter
    Student2 -->|"text"| TextSegmenter
    Tutor -->|"text"| TextSegmenter

    %% Text to Controller (completion tracking)
    Student1 -->|"text"| Controller
    Student2 -->|"text"| Controller
    Tutor -->|"text"| Controller

    %% TTS Flow
    TextSegmenter -->|"text_segment_student1"| TTS_S1
    TextSegmenter -->|"text_segment_student2"| TTS_S2
    TextSegmenter -->|"text_segment_tutor"| TTS_Tutor

    %% Audio to Dynamic Nodes
    TTS_S1 -->|"audio"| DN_Audio
    TTS_S2 -->|"audio"| DN_Audio
    TTS_Tutor -->|"audio"| DN_Audio

    TTS_S1 -->|"audio"| DN_Panel
    TTS_S2 -->|"audio"| DN_Panel
    TTS_Tutor -->|"audio"| DN_Panel

    %% Critical Control Signals
    DN_Audio -->|"session_start<br/>(question_id)"| Controller
    DN_Audio -->|"audio_complete<br/>(participant)"| TextSegmenter
    DN_Audio -->|"buffer_status"| Controller
    DN_Audio -->|"buffer_status"| TextSegmenter

    %% Log aggregation
    Student1 -->|"log"| DN_Log
    Student2 -->|"log"| DN_Log
    Tutor -->|"log"| DN_Log
    Controller -->|"log"| DN_Log
    TextSegmenter -->|"log"| DN_Log
```

## Component Layers

### 1. UI Layer (Makepad Widgets)

| Widget | Purpose |
|--------|---------|
| `MofaHero` | Main hero display with audio buffer gauge, connection status, mic/AEC toggle |
| `ParticipantPanel` | LED visualization bars showing audio levels and active speaker |
| `PromptInput` | Text input for user prompts and control buttons |
| `SystemLog` | Aggregated log display with level filtering |

### 2. Bridge Layer (mofa-dora-bridge)

Bridges connect Makepad UI widgets to Dora dynamic nodes. Each bridge:
- Runs a background thread with Dora event loop
- Uses `DoraNode::init_from_node_id()` to attach to the dataflow
- Translates between Dora Arrow data and Rust types
- Handles metadata extraction (String, Integer, Float, Bool, Lists)

| Bridge | Node ID | Inputs | Outputs |
|--------|---------|--------|---------|
| `AudioPlayerBridge` | mofa-audio-player | audio_student1, audio_student2, audio_tutor | session_start, audio_complete, buffer_status |
| `ParticipantPanelBridge` | mofa-participant-panel | audio_student1, audio_student2, audio_tutor | - |
| `PromptInputBridge` | mofa-prompt-input | llm*_text, llm*_status | control |
| `SystemLogBridge` | mofa-system-log | *_log, *_status | - |

### 3. Dora Dataflow Layer

The dataflow consists of:
- **LLM Participants**: 3 `dora-maas-client` instances (student1, student2, tutor)
- **Conference Bridges**: Route text between participants based on controller signals
- **Controller**: Manages turn-taking with configurable policy
- **Text Segmenter**: FIFO queue with sentence segmentation and backpressure
- **TTS Nodes**: PrimeSpeech instances with different voices

## Signal Flow Sequence

```mermaid
sequenceDiagram
    participant User
    participant UI as MoFA Studio UI
    participant Bridge as AudioPlayerBridge
    participant DN as mofa-audio-player<br/>(Dynamic Node)
    participant Controller as conference-controller
    participant Segmenter as text-segmenter
    participant TTS as primespeech
    participant LLM as maas-client

    User->>UI: Click "Start"
    UI->>Bridge: send_control("start")
    Bridge->>DN: control output
    DN->>Controller: control input

    Controller->>LLM: control_judge (question_id=32)
    LLM->>Segmenter: text stream
    LLM->>Controller: text (completion tracking)

    Segmenter->>TTS: text_segment
    TTS->>DN: audio (with question_id, session_status)

    Note over Bridge,DN: First audio chunk for question_id
    DN->>Bridge: audio event
    Bridge->>Controller: session_start (question_id=32)
    Bridge->>Segmenter: audio_complete (participant)

    Controller->>Controller: Ready for next speaker

    Note over Segmenter: Releases next segment
    Segmenter->>TTS: next text_segment

    loop For each audio chunk
        TTS->>DN: audio
        DN->>Bridge: audio event
        Bridge->>UI: Update buffer gauge
        Bridge->>Segmenter: audio_complete
    end
```

## Critical Metadata Flow

```mermaid
flowchart LR
    subgraph Metadata["Metadata Fields"]
        QID["question_id<br/>(Integer: 32, 288, 546...)"]
        SS["session_status<br/>(started/streaming/complete)"]
        PART["participant<br/>(student1/student2/tutor)"]
    end

    Controller -->|"question_id"| Bridge
    Bridge -->|"question_id"| LLM
    LLM -->|"question_id<br/>session_status"| Segmenter
    Segmenter -->|"question_id<br/>session_status"| TTS
    TTS -->|"question_id<br/>session_status"| AudioPlayer

    AudioPlayer -->|"question_id"| session_start
    AudioPlayer -->|"participant<br/>question_id"| audio_complete

    session_start --> Controller
    audio_complete --> Segmenter
```

### Metadata Parameter Types

The metadata extraction must handle all Dora parameter types:

```rust
let string_value = match value {
    Parameter::String(s) => s.clone(),
    Parameter::Integer(i) => i.to_string(),  // question_id is Integer!
    Parameter::Float(f) => f.to_string(),
    Parameter::Bool(b) => b.to_string(),
    Parameter::ListInt(l) => format!("{:?}", l),
    Parameter::ListFloat(l) => format!("{:?}", l),
    Parameter::ListString(l) => format!("{:?}", l),
};
```

## Critical Signals

### 1. `session_start`
- **From**: mofa-audio-player
- **To**: conference-controller
- **Purpose**: Signals that audio playback has begun for a question_id
- **Trigger**: First audio chunk received for a new question_id
- **Required Metadata**: `question_id`, `participant`

### 2. `audio_complete`
- **From**: mofa-audio-player
- **To**: multi-text-segmenter
- **Purpose**: Flow control - releases next segment from FIFO queue
- **Trigger**: Every audio chunk received
- **Required Metadata**: `participant`, `question_id`, `session_status`

### 3. `buffer_status`
- **From**: mofa-audio-player
- **To**: conference-controller, multi-text-segmenter
- **Purpose**: Backpressure control based on audio buffer fill level
- **Values**: 0-100 (percentage)

## Audio Pipeline Details

### Circular Buffer
- **Duration**: 30 seconds
- **Sample Rate**: 32,000 Hz
- **Format**: Mono f32 samples
- **Behavior**: Overwrites oldest samples when full

### Channel Buffers
- **Audio Channel**: 500 items (non-blocking with `try_send()`)
- **Event Channel**: 100 items
- **Buffer Status Channel**: 10 items

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Conversation stops after N rounds | `session_start` not sent for new question_id | Check metadata extraction handles Integer parameters |
| All LED panels active | Active speaker not tracked per question_id | Use HashSet to track active switches per question_id |
| Audio buffer gauge empty | `set_buffer_level()` not called | Poll audio_player.buffer_fill_percentage() in screen update |
| Pipeline stalls | Channel blocking on full buffer | Use `try_send()` instead of `send()` |
| Missing question_id | Only extracting String parameters | Extract Integer parameters too |

## File Structure

```
mofa-studio/
├── apps/mofa-fm/
│   ├── src/
│   │   ├── screen.rs          # Main screen with dora event polling
│   │   ├── audio_player.rs    # Circular buffer audio playback
│   │   ├── dora_integration.rs # DoraIntegration coordinator
│   │   └── mofa_hero.rs       # Hero widget with buffer gauge
│   └── dataflow/
│       └── voice-chat.yml     # Dora dataflow definition
├── mofa-dora-bridge/
│   └── src/
│       ├── bridge.rs          # DoraBridge trait
│       ├── data.rs            # DoraData, EventMetadata types
│       └── widgets/
│           ├── audio_player.rs      # AudioPlayerBridge
│           ├── participant_panel.rs # ParticipantPanelBridge
│           ├── prompt_input.rs      # PromptInputBridge
│           └── system_log.rs        # SystemLogBridge
└── mofa-widgets/              # Shared UI components
```

## References

- Conference Dashboard: `examples/conference-dashboard/` - Reference implementation
- Dora Node Hub: `node-hub/` - Dora node implementations
- Dataflow: `apps/mofa-fm/dataflow/voice-chat.yml` - Full dataflow definition
