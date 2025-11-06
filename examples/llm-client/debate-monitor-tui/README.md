# Debate Monitor TUI (Rust Implementation)

This is a Rust-based terminal user interface for monitoring LLM debates in the Dora framework, using the Ratatui library. It provides a real-time visualization of the debate between multiple LLM participants.

## Features

- Real-time display of messages from all debate participants (LLM1, Judge, LLM2)
- Color-coded panels for each participant
- Status bar showing the current state of each participant
- Interactive input for sending prompts to the judge
- Bridge reset functionality
- Similar layout and functionality to the Python Textual implementation

## Implementation Details

The Rust implementation mirrors the functionality of the Python `debate_monitor.py` but uses:
- Rust for performance and memory safety
- Ratatui for terminal UI rendering
- Dora Node API for integration with the Dora framework
- Crossterm for terminal event handling

## Structure

The application is structured as follows:

1. **Main Application (`App`)**: Manages the overall state and UI rendering
2. **State Management (`AppState`)**: Tracks messages, current streaming content, and participant statuses
3. **Dora Integration**: Handles receiving events from the Dora framework
4. **TUI Rendering**: Uses Ratatui to render the interface with proper layout and styling

## Usage

The debate monitor is designed to run as part of the Dora dataflow as a dynamic node. To use it:

1. First, make sure the Dora debate dataflow is configured to use the Rust-based monitor:

```yaml
# In your dataflow-debate.yml, ensure the debate-monitor node is set to dynamic:
- id: debate-monitor
  path: dynamic
  # ... other configuration
```

2. Start the dataflow in one terminal:

```bash
dora start dataflow-debate.yml
```

3. Then start the Rust-based debate monitor as a dynamic node in another terminal:

```bash
cd debate-monitor-tui
# Build the project first
cargo build

# Then run as a dynamic node
dora run --name debate-monitor target/debug/debate-monitor-tui
```

Alternatively, use the provided script:
```bash
cd debate-monitor-tui
./run.sh
```

The application will connect to the Dora dataflow as a dynamic node and display the debate in real-time.

Note:
- The monitor must be started after the dataflow is running
- The `--name debate-monitor` must match the node ID in your dataflow YAML file
- The binary path `target/debug/debate-monitor-tui` points to the compiled Rust application

## Controls

- Type in the input field at the bottom to send prompts to the judge
- Press Enter to submit a prompt
- Press Esc to clear the input buffer
- Press 'q' to quit the application

## Future Improvements

- Full control message sending (currently just prints to console)
- Enhanced status bar with colored status indicators
- Better error handling and logging
- Configuration options for customization