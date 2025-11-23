# MaaS Client Cancellation Test

This directory contains a complete test setup for the MaaS client cancellation feature, demonstrating how to cancel ongoing HTTP requests through the control input.

## 🚀 Quick Start

### Prerequisites

Make sure you have:
- Dora CLI installed
- Python 3.8+
- OpenAI API key set in environment: `export OPENAI_API_KEY="your-key-here"`

### Run the Test

```bash
# Start the dataflow (terminal 1)
cd /Users/yuechen/home/fresh/dora/examples/maas-client-cancellation-test
dora start dataflow.yml --name maas-cancellation-test

# In another terminal (terminal 2), watch the receiver:
# (it will show all events including cancellations)

# In another terminal (terminal 3), run the test:
python test_sender.py
```

## 🎯 What the Test Demonstrates

The test sends three cancellation scenarios:

### 1. **Immediate Cancellation** (test-session-1)
   - Sends a long prompt
   - Cancels after 0.5 seconds
   - Expects: Few or no text chunks, cancellation event emitted

### 2. **Mid-Stream Cancellation** (test-session-2)
   - Sends a detailed prompt
   - Waits 3 seconds to receive some content
   - Then cancels
   - Expects: Several text chunks, then cancellation event

### 3. **Request-Specific Cancellation** (test-session-3)
   - Demonstrates cancelling by specific request ID
   - Shows advanced targeting capabilities

## 📊 Expected Output

### Receiver Output

```
🎯 MaaS Client Cancellation Test - Receiver
======================================================================
Listening for events...

[10:23:45.123] 📨 text
         Status: started
         Data: Once upon a time in...
         Metadata: {'session_id': 'test-session-1', 'segment_index': '0'}

[10:23:45.623] 📨 cancellation
         🚨 CANCELLATION EVENT 🚨
         Session ID: test-session-1
         Request ID: req-abc123
         Reason: user_requested
         Source: user
         Timestamp: 1234567890

[10:23:45.624] 📨 status
         Status: cancelled
```

### Key Observations

1. **Cancellation Events**: You should see `cancellation` events with reason details
2. **No Text After Cancel**: Text chunks should stop after cancellation
3. **Status Updates**: Status changes to "cancelled"
4. **Session Tracking**: Each session is tracked separately

## 🔧 Configuration

The dataflow uses these cancellation settings:

```yaml
env:
  OPENAI_API_KEY: "${OPENAI_API_KEY}"
  REQUEST_TIMEOUT_SECS: "30"        # Timeout for regular requests
  STREAM_TIMEOUT_SECS: "120"        # Timeout for streaming requests
  ENABLE_CANCELLATION: "true"       # Enable cancellation feature
  EMIT_CANCELLATION_EVENTS: "true"  # Emit events to downstream nodes
```

## 🎮 Control Input API

### Cancel Current Session

```python
import json
from dora import Node

node = Node()
cancel_command = json.dumps({
    "command": "cancel",
    "target": "session"  # Cancels all requests for this session
})

node.send_output(
    "control",
    {"session_id": "my-session"},
    cancel_command
)
```

### Cancel Specific Request

```python
cancel_command = json.dumps({
    "command": "cancel",
    "target": "request",  # Cancels specific request
    "id": "request-id-123"
})

node.send_output(
    "control",
    {"session_id": "my-session"},
    cancel_command
)
```

### Cancel All Requests for a Different Session

```python
cancel_command = json.dumps({
    "command": "cancel",
    "target": "session",
    "id": "other-session-id"  # Target specific session
})

node.send_output(
    "control",
    {},  # Can be empty or have session_id
    cancel_command
)
```

## 📤 Cancellation Event Format

When a request is cancelled, a `cancellation` output is emitted:

```json
{
  "session_id": "test-session-1",
  "request_id": "req-abc123",
  "reason": {
    "type": "user_requested",
    "source": "user"
  },
  "timestamp": 1234567890
}
```

**Reason Types:**
- `user_requested`: Cancelled via control command
- `timeout`: Cancelled due to timeout
- `node_shutdown`: Cancelled because node is shutting down
- `error`: Cancelled due to error

**Event Metadata:**
```python
{
    'session_status': 'cancelled',
    'cancellation_reason': 'user_requested',
    'session_id': 'test-session-1',
    # ... plus all original metadata from the prompt
}
```

## 🏗️ Architecture

```
┌─────────────────┐
│  test-sender    │  Sends prompts and cancel commands
│                 │  via 'prompt' and 'control' outputs
└────────┬────────┘
         │
         ▼
┌────────────────────────┐
│   maas-client          │  Main MaaS client with cancellation
│                        │  - Accepts control commands
│                        │  - Emits cancellation events
│                        │  - Stops HTTP requests when cancelled
└──────┬─────────────────┘
       │
       ├───────text─────────────────────┐
       ├───────status───────────────────┤
       └───────cancellation─────────────┘
                                       ▼
                            ┌──────────────────┐
                            │  test-receiver   │  Logs and verifies
                            │                  │  cancellation events
                            └──────────────────┘
```

## 💡 Usage in Production

### Dataflow Configuration

```yaml
nodes:
  - id: maas-client
    operator:
      rust: ../../node-hub/dora-maas-client
    inputs:
      prompt: ui/prompt
      control: ui/control  # Add control input for cancellation
    outputs:
      - text
      - status
      - cancellation  # Add cancellation output
    env:
      ENABLE_CANCELLATION: "true"
      EMIT_CANCELLATION_EVENTS: "true"

  - id: tts-node
    operator:
      python: ../../node-hub/dora-primespeech
    inputs:
      text: maas-client/text
      cancellation: maas-client/cancellation  # Handle cancellation
    outputs:
      - audio
```

### Downstream Node (e.g., TTS) Handling

```python
from dora import Node
import json

node = Node()

cancelled_sessions = set()

while True:
    event = node.next()
    if event is None:
        break

    event_type = event["type"]
    data = event["data"][0] if event["data"] else ""
    metadata = event["metadata"]
    session_id = metadata.get("session_id")

    # Handle cancellation events
    if event_type == "cancellation":
        cancel_data = json.loads(data)
        cancelled_sessions.add(session_id)

        # Stop current TTS synthesis
        stop_tts_synthesis()

        # Clean up resources
        clear_pending_segments()

        print(f"Session {session_id} cancelled")

    # Handle text (but ignore if cancelled)
    elif event_type == "text":
        if session_id in cancelled_sessions:
            # Skip text from cancelled sessions
            continue

        # Process text normally
        synthesize_speech(data)
```

## 🔍 Troubleshooting

### No cancellation events received
- Check: `ENABLE_CANCELLATION=true` in environment
- Check: Control input is connected in dataflow
- Check: Using OpenAI client (Gemini doesn't support streaming yet)

### Text continues after cancellation
- Bug: Cancellation token not properly passed to streaming function
- Fix: Ensure `complete_streaming_with_cancellation()` is called

### Cancellation not stopping HTTP request
- Check: `tokio-util` dependency is added to Cargo.toml
- Check: Timeout is properly configured
- Check: Firewall not blocking connection close

## 🧪 Running Specific Tests

```bash
# Test only immediate cancellation
python -c "
from dora import Node
import time
node = Node()
node.send_output('prompt', {'session_id': 'test'}, 'Tell me a story...')
time.sleep(0.5)
node.send_output('control', {'session_id': 'test'}, '{\"command\":\"cancel\",\"target\":\"session\"}')
"

# Test timeout (set short timeout)
MAAS_STREAM_TIMEOUT_SECS=5 dora start dataflow.yml
# Send a prompt that would take longer than 5 seconds
```

## 📚 Related Files

- Main implementation: `node-hub/dora-maas-client/src/main.rs`
- Streaming module: `node-hub/dora-maas-client/src/streaming.rs`
- Client module: `node-hub/dora-maas-client/src/client.rs`
- Configuration: `node-hub/dora-maas-client/src/config.rs`

## 🤝 Contributing

To extend the cancellation feature:

1. Add new cancellation reasons in `streaming.rs`:
   ```rust
   pub enum CancellationReason {
       UserRequested { source: String },
       Timeout { duration_secs: u64 },
       NodeShutdown,
       Error { details: String },
       YourNewReason { details: String },  // Add here
   }
   ```

2. Update event emission in `main.rs`:
   ```rust
   match reason {
       // ... existing matches
       CancellationReason::YourNewReason { details } => "your_reason",
   }
   ```

3. Update downstream handlers to recognize new reason type

## 📄 License

Same as Dora project (Apache 2.0)
