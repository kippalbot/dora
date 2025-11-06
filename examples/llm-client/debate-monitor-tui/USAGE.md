# Rust Debate Monitor - Usage Instructions

## Prerequisites
Make sure you have:
1. The Dora dataflow running
2. A proper terminal environment

## How to Use

### 1. Start the Dataflow
In one terminal, start the debate dataflow:
```bash
cd /Users/yuechen/home/fresh/dora/examples/llm-client
dora start dataflow-debate.yml
```

### 2. Run the Rust Monitor
In another terminal, run the Rust debate monitor:
```bash
cd /Users/yuechen/home/fresh/dora/examples/llm-client/debate-monitor-tui
cargo run -r -p debate-monitor-tui
```

### 3. Interact with the Application
Once running, you can:
- Type prompts in the input field at the bottom
- Press Enter to send prompts to the judge
- Press Esc to clear the input buffer
- Type "reset" and press Enter to reset the debate bridges
- Press 'q' to quit the application

## Troubleshooting

### "Device not configured" Error
This usually means:
1. The dataflow isn't running - make sure you started it first
2. You're not in a proper terminal - make sure you're in a real terminal, not an IDE terminal
3. Terminal permissions - make sure your terminal has proper access

### Not Seeing Messages
If you don't see messages appearing:
1. Make sure the debate has started (send a prompt first)
2. Check that the dataflow is properly configured
3. The application now has debug output to show what events it's receiving

## Features
- Real-time display of messages from all debate participants
- Three-panel layout (Judge, LLM1, LLM2)
- Status bar showing participant states
- Interactive input for sending prompts to the judge
- Bridge reset functionality
- Clean exit with 'q' key

## Debug Mode
The application now includes debug output that shows:
- What input events are being received
- What text is being processed
- When control messages are sent
- Participant identification

This helps troubleshoot issues with message flow.