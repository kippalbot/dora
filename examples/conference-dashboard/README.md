# Conference Dashboard

A Makepad-based real-time dashboard for multi-participant AI conference sessions with TTS audio playback, visualization, and integrated dataflow management.

## Features

- **Multi-Participant Display**: Shows 3 participants with real-time status indicators and LED audio meters
- **Audio Visualization**: LED-style frequency meter with 8 bands and 4 segments each per participant
- **System Monitoring**: Real-time CPU and Memory usage gauges
- **Buffer Status**: Audio buffer gauge showing playback queue status with color-coded LED segments
- **Chat History**: Scrollable markdown-rendered conversation history (filters out context messages)
- **Log Panel**: Collapsible log viewer with filtering by level, node, and keyword search
- **Clipboard Export**: Copy filtered logs to clipboard with one click
- **Session Control**: Send prompt and Reset buttons for conference management
- **Dataflow Management**: Automatically start/stop dataflows via command line

## Quick Start

### One-Command Launch

```bash
# Start dataflow and dashboard together (release mode for best performance)
cargo run -r -- --dataflow dataflow-study-audio-multi-dashboard.yml

# Short form
cargo run -r -- -d dataflow-study-audio-multi-dashboard.yml
```

### Manual Launch (Two Terminals)

**Terminal 1** - Start the Dora dataflow:
```bash
dora start dataflow-study-audio-multi-dashboard.yml
```

**Terminal 2** - Start the dashboard:
```bash
DORA_NODE_ID=dashboard cargo run -r
```

## Command Line Options

```
conference-dashboard [OPTIONS]

OPTIONS:
    -d, --dataflow <PATH>     Path to dataflow YAML file to start
    -n, --name <NAME>         Node name to connect as (default: dashboard)
    -s, --sample-rate <RATE>  Audio sample rate in Hz (default: 32000)
    -h, --help                Print help information

ENVIRONMENT VARIABLES:
    DORA_NODE_ID              Node name (if --name not provided)
    SAMPLE_RATE               Audio sample rate (if --sample-rate not provided)
    DATAFLOW_PATH             Dataflow file (if --dataflow not provided)
    PARTICIPANT1_NAME         Name for first participant (default: Daniu)
    PARTICIPANT2_NAME         Name for second participant (default: Yifei)
    PARTICIPANT3_NAME         Name for third participant (default: Laoshi)

EXAMPLES:
    cargo run -r -- --dataflow dataflow-study.yml
    cargo run -r -- -d dataflow.yml -n my-dashboard -s 24000
    DORA_NODE_ID=dashboard cargo run -r
```

## UI Layout

```
┌───────────────────────────────────────────────────────────┬─────────────────┐
│ [Logo] MoFA FM                               ● Connected  │ [>] System Logs │
├───────────────────────────────────────────────────────────┤─────────────────│
│ ● Buffer [████░░░░] │ ● CPU [██░░░░] │ ● Memory [████░░]  │ [ALL ▼]         │
├───────────────────────────────────────────────────────────│ [Nodes ▼]       │
│ ┌───────────────┐ ┌───────────────┐ ┌───────────────┐     │ [Search... ] [C]│
│ │    Daniu      │ │    Yifei      │ │    Laoshi     │     │─────────────────│
│ │   [LED Bar]   │ │   [LED Bar]   │ │   [LED Bar]   │     │ [INFO] ctrl:... │
│ └───────────────┘ └───────────────┘ └───────────────┘     │ [DEBUG] s1:...  │
├───────────────────────────────────────────────────────────│ [INFO] tts:...  │
│                                                           │                 │
│                    Chat History                           │                 │
│                                                           │                 │
│  **Daniu**: Hello, let's discuss...                       │                 │
│  **Yifei**: I agree with that point...                    │                 │
│  **Laoshi**: Good observations, let me add...             │                 │
│                                                           │                 │
├───────────────────────────────────────────────────────────│                 │
│ [Enter prompt to send to tutor...     ] [Send] [Reset]    │                 │
└───────────────────────────────────────────────────────────┴─────────────────┘

Layout Structure:
┌─────────────────────────────────────────────────┐  ┌──────────────────┐
│                  MAIN CONTENT                   │  │    LOG PANEL     │
│  ┌───────────────────────────────────────────┐  │  │ (collapsible)    │
│  │ Header: Logo + Title + Status             │  │  │                  │
│  ├───────────────────────────────────────────┤  │  │ - Level filter   │
│  │ Control Bar: Buffer | CPU | Memory        │  │  │ - Node filter    │
│  ├───────────────────────────────────────────┤  │  │ - Search box     │
│  │ Participants: [Daniu] [Yifei] [Laoshi]    │  │  │ - Copy button    │
│  ├───────────────────────────────────────────┤  │  │                  │
│  │ Chat History (scrollable markdown)        │  │  │ Log entries...   │
│  ├───────────────────────────────────────────┤  │  │                  │
│  │ Prompt: [Input] [Send] [Reset]            │  │  │                  │
│  └───────────────────────────────────────────┘  │  │                  │
└─────────────────────────────────────────────────┘  └──────────────────┘
                      ↕ Draggable Splitter
```

## Log Panel Features

### Filtering
- **Level Filter**: ALL, DEBUG, INFO, WARN, ERROR
- **Node Filter**: All Nodes, Daniu, Yifei, Laoshi, Bridge, Controller, Segmenter, TTS, Dashboard
- **Keyword Search**: Free-text search in log messages and source names

### Actions
- **Copy to Clipboard**: Click the clipboard icon to copy all filtered logs
- **Toggle Panel**: Click ">" to collapse, "<" to expand
- **Resize**: Drag the splitter to adjust panel width

## System Monitoring

The dashboard displays real-time system metrics:
- **Buffer**: Audio playback buffer fill level (0-100%)
- **CPU**: Average CPU usage across all cores
- **Memory**: System memory usage with GB values

Each gauge uses color-coded LED segments:
- Blue (0-30%): Low usage
- Cyan (30-50%): Normal
- Green (50-70%): Moderate
- Yellow (70-85%): High
- Orange (85-95%): Very high
- Red (95-100%): Critical

## Prerequisites

1. Build the required binaries:
```bash
cd ../../
cargo build --release -p dora-maas-client
cargo build --release -p dora-conference-bridge
cargo build --release -p dora-conference-controller
```

2. Install Python dependencies:
```bash
pip install -e ../../node-hub/dora-text-segmenter
pip install -e ../../node-hub/dora-primespeech
```

3. Set environment variables:
```bash
export OPENAI_API_KEY="your-key"
export DEEPSEEK_API_KEY="your-key"
export ALIBABA_CLOUD_API_KEY="your-key"
```

## Configuration Files

| File | Description |
|------|-------------|
| `dataflow-study-audio-multi-dashboard.yml` | Main dataflow configuration |
| `study_config_student1.toml` | MaaS config for Student1 |
| `study_config_student2.toml` | MaaS config for Student2 |
| `study_config_tutor.toml` | MaaS config for Tutor |
| `study-context.md` | Context document for study session |

## Starting a Study Session

Once the dashboard is running and connected (green dot visible):

### Option 1: Quick Start
Simply click the **[Send]** button with an empty input field. This sends the default prompt "Let's start" to the tutor, who will begin the study session.

### Option 2: Custom Prompt
1. Type your own prompt in the input field, e.g.:
   - "Let's discuss machine learning basics"
   - "Please explain quantum computing"
   - "Start with a question about history"
2. Click **[Send]** to send your prompt to the tutor

The tutor (Laoshi) will receive your prompt and begin the discussion. Students (Daniu and Yifei) will then participate in the conversation, taking turns based on the conference controller's policy.

## Controls

- **Send**: Send prompt text to the tutor (default: "Let's start" if empty)
- **Reset**: Clear conversation, reset all participants, stop audio playback

## Status Indicators

| Color | Status |
|-------|--------|
| Blue | Idle - Waiting for input |
| Green | Speaking - Currently responding |
| Red | Error - Processing error occurred |

## Participant Configuration

### Default Names
- **Participant 1**: Daniu (大牛)
- **Participant 2**: Yifei (亦菲)
- **Participant 3**: Laoshi (老师)

### Custom Names via Environment
```bash
PARTICIPANT1_NAME="Alice" PARTICIPANT2_NAME="Bob" PARTICIPANT3_NAME="Teacher" cargo run
```

### TTS Voices (PrimeSpeech)
- **Student1**: Luo Xiang voice - Male, rational tone
- **Student2**: Doubao voice - Female, emotional tone
- **Tutor**: Zhao Daniu voice - Male, authoritative tone

## Dataflow Architecture

```
student1 ─────┬────► bridge-to-student1 ───► multi-text-segmenter
student2 ─────┼────► bridge-to-student2 ───────────┬───────────────►
tutor    ─────┴────► bridge-to-tutor    ───────────┤
                                                   │
                               ┌───────────────────┘
                               ▼
                    primespeech-student1 ─► audio_student1 ─┐
                    primespeech-student2 ─► audio_student2 ─┼─► dashboard
                    primespeech-tutor    ─► audio_tutor    ─┘
                               │
conference-controller ◄────────┴──────────────────────────────────►
```

## Development

### Building

```bash
cargo build           # Debug build
cargo build --release # Release build
```

### Code Structure

```
src/
├── main.rs           # Entry point with dataflow management
├── lib.rs            # Shared state and types
├── app.rs            # Main Makepad application and UI
├── audio_player.rs   # Circular buffer audio playback
├── dora_bridge.rs    # Dora dataflow integration
└── widgets/
    └── mod.rs        # Custom widget definitions
```

### Key Components

| Component | Description |
|-----------|-------------|
| `SharedState` | Thread-safe state shared between UI and Dora bridge |
| `AudioPlayer` | Circular buffer with multi-participant audio mixing |
| `DoraBridge` | Handles Dora node connection and message routing |
| `Dashboard` | Main Makepad widget with all UI elements |

## Troubleshooting

### Audio not playing
- Check that PrimeSpeech models are installed at `~/.dora/models/primespeech`
- Verify sample rate matches TTS output (32kHz for PrimeSpeech, 24kHz for Kokoro)
- Use `-s 24000` for Kokoro TTS

### Participants stuck on "Idle"
- Check API keys are set correctly
- View logs in the log panel (filter by node name)
- Verify network connectivity to API endpoints

### Dashboard not receiving messages
- Check dataflow is running: `dora list`
- Verify node name matches YAML: `-n dashboard` or `DORA_NODE_ID=dashboard`
- Check log panel for connection errors

### Dataflow won't start
- Ensure dora daemon is running: `dora up`
- Check YAML syntax: `dora check dataflow.yml`
- Look for port conflicts or missing binaries

### Log panel issues
- Use level filter to reduce noise (INFO or WARN)
- Use search to find specific messages
- Copy logs to clipboard for external analysis
