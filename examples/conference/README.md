# Conference Debate Example

Implements a 3-person debate using the new conference controller and bridge architecture with a **sequential policy**. This replaces the complex multi-bridge setup with a simpler, more maintainable architecture.

## Architecture Overview

### Old Approach (llm-client/debate)
❌ Multiple bridges with complex cold-start configurations
❌ Manual control logic in debate-monitor
❌ Hard to extend or modify speaking order
❌ Each bridge had different settings

### New Approach (conference/debate)
✅ **Single conference controller** - manages speaking order via policy
✅ **Single conference bridge** - executes controller commands
✅ **Sequential policy** - `[llm1 → llm2 → judge]` → easy to understand
✅ **Auto-pause/resume** - Bridge auto-pauses after each turn

## Flow Diagram

```
┌─────────────────────────────────────────────────────────┐
│                CONFERENCE CONTROLLER                     │
│                                                          │
│  Policy: [llm1 → llm2 → judge] (sequential)             │
│                                                          │
│  1. Track word counts for each participant              │
│  2. Determine next speaker based on policy              │
│  3. Send "resume" command to bridge                     │
│                                                          │
└──────────────────┬───────────────────────────────────────┘
                   │ "resume" command
                   ↓
┌─────────────────────────────────────────────────────────┐
│                CONFERENCE BRIDGE                         │
│                                                          │
│  • Receives all participant messages                    │
│  • Waits for "resume" from controller                   │
│  • Forwards ONE speaker's messages                      │
│  • Auto-pauses after forwarding                         │
│                                                          │
└──────────────────┬───────────────────────────────────────┘
                   │ forwarded messages
                   ↓
┌─────────────────────────────────────────────────────────┐
│                    DEBATE MONITOR                        │
│                                                          │
│  Real-time 3-panel TUI showing:                         │
│  - LLM1 (top-left): Debater A (正方)                   │
│  - LLM2 (top-right): Debater B (反方)                  │
│  - Judge (bottom): Moderator (主持人)                  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## File Structure

```
conference/
├── README.md                          # This file
├── dataflow-debate-sequential.yml     # Main dataflow with conference controller
├── debate_config_maas_llm1.toml      # LLM1 configuration
├── debate_config_maas_llm2.toml      # LLM2 configuration
├── debate_config_maas_judge.toml     # Judge configuration
└── dataflow-debate-sequential-controllers.yml  # Optional: no monitors (controller only)
```

## Quick Start

### 1. Set up environment

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Use the same environment as llm-client
source ../../setup-new-chatbot/setup_cloud_env.sh
conda activate dora_cloud

# Build required components
cd /Users/yuechen/home/fresh/dora

# Build conference controller
cargo build -p dora-conference-controller --release

# Build conference bridge
cargo build -p dora-conference-bridge --release

# Build maas client
cargo build -p dora-maas-client --release

# Build terminal print
cargo build -p terminal-print --release
```

### 2. Set API key

```bash
export ALIBABA_CLOUD_API_KEY="your-api-key-here"
```

### 3. Start the debate

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Start the dataflow
dora start dataflow-debate-sequential.yml

# The debate will start automatically with sequential turns:
# llm1 → llm2 → judge → llm1 → llm2 → judge → ...
```

## Speaking Order

The **sequential policy** `[llm1 → llm2 → judge]` means:

1. **llm1 (正方)** speaks first
2. **llm2 (反方)** speaks second
3. **judge (主持人)** speaks third, provides summary/comments
4. Loop back to llm1

**Word counts are tracked**, so the controller can ensure fair speaking time distribution if needed (though sequential gives equal turns).

## Components Reused from llm-client

### 1. dora-maas-client
- **Location**: `node-hub/dora-maas-client/`
- **Use**: Connects to Alibaba Cloud AI for LLM responses
- **Configuration**: `.toml` files with prompts and models

### 2. debate-monitor
- **Location**: `examples/llm-client/debate-monitor/`
- **Use**: Real-time 3-panel terminal UI
- **Features**:
  - Shows streaming responses as they arrive
  - Displays question_id for tracking rounds
  - Shows controller status (new!)

### 3. viewer
- **Location**: `examples/llm-client/viewer.py`
- **Use**: Log monitoring and event tracking
- **Features**:
  - Logs all node outputs
  - Helps debug issues
  - Can save logs to files

## Comparison with Old Debate Example

| Aspect | Old (llm-client/debate) | New (conference/debate) |
|--------|------------------------|------------------------|
| **Bridges** | 3 separate bridges | 1 conference bridge |
| **Controller logic** | In debate-monitor | Separate conference-controller node |
| **Policy** | Hardcoded in monitor | Configurable via env var |
| **Speaking order** | Configured per bridge | Single policy pattern |
| **Auto-pause** | Manual in monitor | Built into bridge |
| **Extension** | Hard to add participants | Easy: just add to pattern |

### Old dataflow-debate.yml (excerpt)

```yaml
# Bridge 1: llm1 + llm2 → judge
- id: bridge-to-judge
  inputs:
    llm1: llm1/text
    llm2: llm2/text
    control: debate-monitor/bridge_control
  env:
    COLD_START: "false"  # Manual configuration

# Bridge 2: llm1 + judge → llm2
- id: bridge-to-llm2
  inputs:
    llm1: llm1/text
    judge: judge/text
    control: debate-monitor/bridge_control
  env:
    COLD_START: "false"  # Different settings!
```

### New dataflow-debate-sequential.yml (excerpt)

```yaml
# Conference controller - brain
- id: conference-controller
  env:
    DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
  inputs:
    llm1: llm1/text
    llm2: llm2/text
    judge: judge/text

# Conference bridge - switch
- id: conference-bridge
  inputs:
    llm1: llm1/text
    llm2: llm2/text
    judge: judge/text
    control: conference-controller/control  # Auto-pause/resume
```

## Configuration

### Policy Pattern

```yaml
env:
  DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
```

**To change speaking order**, just edit the pattern:
- `[judge → llm1 → llm2]` - Judge speaks first
- `[llm1 → judge → llm2]` - Judge in the middle
- `[judge, llm1, llm2]` - Equal ratio mode (no → arrows)

### LLM Configurations

Edit the `.toml` files to change:
- System prompts (角色设定)
- Model parameters (temperature, max_tokens, etc.)
- API endpoints

Example from `debate_config_maas_llm1.toml`:
```toml
system_prompt = "你叫大牛，是中学历史教师，对清代历史很有研究。你是正方，支持完全恢复..."
model = "qwen-max"
temperature = 0.8
```

## Monitoring

### Debate Monitor (TUI)

```bash
# Shows 3 panels:
# ┌──────────────────┬──────────────────┐
# │  LLM1 (正方)     │  LLM2 (反方)     │
# │                  │                  │
# │  [streaming]     │  [streaming]     │
# └──────────────────┴──────────────────┘
# ┌───────────────────────────────────────┐
# │  Judge (主持人)                       │
# │  [comments/summary]                   │
# └───────────────────────────────────────┘
```

Press in debate-monitor:
- `q` or `Ctrl-C` - Quit
- `r` - Reset debate

### Viewer (Logs)

```bash
# Shows all events and logs for debugging
# Can be redirected to file:
./viewer.py > debate_logs.txt
```

## Troubleshooting

### Debate doesn't start

1. Check API key:
   ```bash
   echo $ALIBABA_CLOUD_API_KEY
   ```

2. Check logs:
   ```bash
   # In another terminal
cd /Users/yuechen/home/fresh/dora/examples/conference
cargo run -p dora-conference-controller 2>&1 | grep -i error
   ```

3. Verify configs:
   ```bash
   ls -la debate_config_maas_*.toml
   ```

### Participants don't take turns

1. Check controller status output:
   ```bash
   # Look for conference-controller/status in viewer
   ```

2. Verify pattern syntax:
   ```bash
   # Should be: [llm1 → llm2 → judge]
   grep PATTERN dataflow-debate-sequential.yml
   ```

3. Check bridge receives commands:
   ```bash
   # Look for "resume" commands in bridge logs
   ```

### Reset doesn't work

1. Connect reset button:
   ```bash
   # Ensure dataflow has:
   # inputs:
   #   control: reset-button/status
   ```

## Advanced Usage

### Add a fourth participant

```yaml
# 1. Add participant to dataflow
- id: llm3
  inputs:
    text: conference-bridge/text

# 2. Update policy pattern
env:
  DORA_POLICY_PATTERN: "[llm1 → llm2 → llm3 → judge]"

# 3. Add to controller inputs
inputs:
  llm3: llm3/text

# 4. Add to bridge inputs
inputs:
  llm3: llm3/text
```

### Use priority-based policy

```yaml
# Judge speaks first (unless they just spoke)
env:
  DORA_POLICY_PATTERN: "[(judge, *), (llm1, 1), (llm2, 1)]"
```

### Use ratio-based policy

```yaml
# Judge gets 2x speaking time
env:
  DORA_POLICY_PATTERN: "[(judge, 2), (llm1, 1), (llm2, 1)]"
```

## Differences from Original

### What Changed

1. **Conference controller** - New node that manages policy
2. **Conference bridge** - Single bridge with auto-pause
3. **Policy pattern** - Configurable via env var
4. **Sequential turns** - Fixed order vs complex cold-start logic

### What Stayed the Same

1. **MaaS client** - Same configuration and prompts
2. **Debate-monitor** - Same 3-panel TUI
3. **Viewer** - Same log monitoring
4. **Speaking participants** - Same 3 roles (llm1, llm2, judge)

## References

- [Conference Controller API](../../node-hub/dora-conference-controller/README.md)
- [Conference Bridge API](../../node-hub/dora-conference-bridge/README.md)
- [MaaS Client API](../../node-hub/dora-maas-client/API.md)
- [Original Debate Example](../llm-client/README.md)
- [Policy Naming Convention](./NAMING_CONVENTION.md)
- [Architecture Overview](./ARCHITECTURE.md)

## License

Same as parent project (Apache 2.0)
