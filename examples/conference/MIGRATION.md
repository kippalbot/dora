# Migration Guide: Old Debate → New Conference Architecture

This document helps migrate from the old debate system (multiple bridges with cold-start) to the new conference architecture (controller + bridge with policy).

## Key Differences

### Architectural Changes

| Aspect | Old (dataflow-debate.yml) | New (conference/) |
|--------|---------------------------|-------------------|
| **Bridge count** | 3 separate bridges | 1 conference bridge |
| **Control logic** | In debate-monitor | In conference-controller |
| **Policy** | Hardcoded (cold-start configs) | Configurable via env var |
| **State management** | Manual | Auto-pause/resume built-in |
| **Extensibility** | Hard to add participants | Easy (just edit pattern) |

## Side-by-Side Comparison

### Old Approach: Multiple Bridges

```yaml
# Old: dataflow-debate.yml

# Bridge 1: Waits for both LLMs → forwards to judge
- id: bridge-to-judge
  inputs:
    llm1: llm1/text
    llm2: llm2/text
    control: debate-monitor/bridge_control
  env:
    COLD_START: "false"  # Wait for both inputs
    INC_QUESTION_ID: "true"

# Bridge 2: LLM1 + judge → LLM2
- id: bridge-to-llm2
  inputs:
    llm1: llm1/text
    judge: judge/text
    control: debate-monitor/bridge_control
  env:
    COLD_START: "false"  # Forward judge immediately
    INC_QUESTION_ID: "false"

# Bridge 3: LLM2 + judge → LLM1
- id: bridge-to-llm1
  inputs:
    llm2: llm2/text
    judge: judge/text
    control: debate-monitor/bridge_control
  env:
    COLD_START: "true"   # Forward judge immediately
    INC_QUESTION_ID: "false"

# Monitor manually controls bridges
- id: debate-monitor
  inputs:
    # ... many inputs
  outputs:
    - control
    - bridge_control  # ← Controls bridges manually
```

**Problems:**
- ❌ Each bridge has different cold-start settings
- ❌ Hard to understand flow
- ❌ Adding a 4th participant requires modifying 3 bridges
- ❌ Control logic scattered in monitor

### New Approach: Conference Architecture

```yaml
# New: conference/dataflow-debate-sequential.yml

# Single controller manages policy
- id: conference-controller
  env:
    DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
  inputs:
    llm1: llm1/text      # ← Tracks when llm1 speaks
    llm2: llm2/text      # ← Tracks when llm2 speaks
    judge: judge/text    # ← Tracks when judge speaks
  outputs:
    - control            # → Sends "resume" to bridge

# Single bridge executes commands
- id: conference-bridge
  inputs:
    llm1: llm1/text           # ← Receives all messages
    llm2: llm2/text           # ← Queues them
    judge: judge/text         # ← Forwards when told
    control: conference-controller/control  # ← "resume"
  # No env vars needed for policy!

# Monitor just displays (no control logic)
- id: debate-monitor
  inputs:
    # ... display only, no control outputs
```

**Benefits:**
- ✅ One bridge - simple to understand
- ✅ Policy in one place (env var)
- ✅ Easy to modify: just change pattern
- ✅ Auto-pause/resume built-in
- ✅ 85% fewer lines of config!

## Migration Steps

### Step 1: Replace Bridges with Conference Architecture

**Remove:**
```yaml
# DELETE these nodes
- bridge-to-judge
- bridge-to-llm2
- bridge-to-llm1
```

**Add:**
```yaml
# ADD these nodes

- id: conference-controller
  operator:
    rust: dora-conference-controller
  env:
    DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"  # Configurable!
  inputs:
    llm1: llm1/text
    llm2: llm2/text
    judge: judge/text
  outputs:
    - control
    - status

- id: conference-bridge
  operator:
    rust: dora-conference-bridge
  inputs:
    llm1: llm1/text
    llm2: llm2/text
    judge: judge/text
    control: conference-controller/control
  outputs:
      - text
      - status
      - log
```

### Step 2: Update LLM Inputs

**Change LLM inputs from multiple bridges to single bridge:**

```yaml
# OLD
- id: llm1
  inputs:
    text: bridge-to-llm1/text  # From old bridge

# NEW
- id: llm1
  inputs:
    text: conference-bridge/text  # From new bridge
```

### Step 3: Update Monitor (Optional)

The debate-monitor can remain mostly the same, but remove bridge control outputs:

```yaml
# OLD
- id: debate-monitor
  outputs:
    - control
    - bridge_control  # ← Remove this

# NEW
- id: debate-monitor
  outputs:
    - control  # Only used for judge control
```

### Step 4: Update Connections

Change all bridge inputs to use the single conference bridge:

```yaml
# OLD
debate-monitor:
  inputs:
    bundle_text: bridge-to-judge/text
    llm1_prompt: bridge-to-llm1/text
    llm2_prompt: bridge-to-llm2/text

# NEW
debate-monitor:
  inputs:
    bundle_text: conference-bridge/text  # One source!
    llm1_prompt: conference-bridge/text
    llm2_prompt: conference-bridge/text
```

## Policy Translation Guide

### Old: Cold Start Configs → New: Policy Patterns

| Old Logic | New Policy | Meaning |
|-----------|------------|---------|
| `COLD_START: false` (wait for both) | `[llm1 → llm2]` | Sequential |
| `COLD_START: true` (forward immediately) | `[(judge, *), (llm1, 1)]` | Judge priority |
| Manual control | Same pattern | Controller does it automatically |

### Examples

**Courtroom debate**:
```yaml
# Old: Hardcoded in 3 bridges
# Bridge 1: Defense + Prosecution → Judge
# Bridge 2: Defense + Judge → Prosecution
# Bridge 3: Prosecution + Judge → Defense

# New: Single pattern
DORA_POLICY_PATTERN: "[defense → prosecution → judge]"
```

**Interview show**:
```yaml
# Old: Complex cold-start timing
# Bridge 1: Host speaks first
# Bridge 2: Host + Guest1 → Guest2
# etc.

# New: Priority pattern
DORA_POLICY_PATTERN: "[(host, *), (guest1, 1), (guest2, 1)]"
```

**Round-robin**:
```yaml
# Old: Manual rotation in monitor
# Cycle through all 5 participants

# New: Simple pattern
DORA_POLICY_PATTERN: "[alice → bob → charlie → david → eve]"
```

## Migration Checklist

- [ ] Remove old bridge nodes (bridge-to-*, etc.)
- [ ] Add conference-controller node
- [ ] Add conference-bridge node
- [ ] Update all LLM inputs to use conference-bridge
- [ ] Update debate-monitor inputs
- [ ] Copy debate config files (if needed)
- [ ] Test speaking order
- [ ] Verify auto-pause/resume works
- [ ] Update documentation

## Testing

### Verify Policy Parsing

```bash
cd /Users/yuechen/home/fresh/dora
export DORA_POLICY_PATTERN="[llm1 → llm2 → judge]"
cargo run -p dora-conference-controller

# Should see:
# ✅ Policy configured with participants: ["llm1", "llm2", "judge"]
# 📊 Policy configuration:
# {
#   "mode": "sequential",
#   "sequence": ["llm1", "llm2", "judge"],
#   ...
# }
```

### Verify Bridge Control

```bash
# Start dataflow
dora start dataflow-debate-sequential.yml

# Check logs:
tail -f out/*.log | grep -E "(resume|auto-pause|forward)"

# Should see:
# [conference-controller] ⏯️ Sending resume command
# [conference-bridge] ⏸️ Bridge auto-paused after forwarding one cycle
# [conference-bridge] → Forwarded bundle (queued: llm1:0, llm2:0, judge:0)
```

## Benefits Summary

| Metric | Old | New |
|--------|-----|-----|
| Bridge count | 3 | 1 |
| Lines of config | ~150 | ~70 |
| Policy location | Hardcoded | Env var |
| Add participant | 3 bridges + monitor | 1 line (pattern) |
| Understandability | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| Testability | Hard | Easy (unit tests) |

## When NOT to Migrate

Stay with old architecture if:
- You need different forwarding modes per bridge
- You have complex multi-stage pipelines
- You need manual control over every bridge
- You're happy with current setup

Migrate to new architecture if:
- You want automatic turn management
- You need configurable policies
- You plan to add more participants
- You want simpler, more maintainable code

## Full Example

See complete working examples:
- `conference/dataflow-debate-sequential.yml` - With monitors
- `conference/dataflow-debate-minimal.yml` - Controller + bridge only
- `conference/run_debate.sh` - Launcher script

## References

- [Conference Controller API](../../node-hub/dora-conference-controller/)
- [Conference Bridge API](../../node-hub/dora-conference-bridge/)
- [Policy Naming Convention](./NAMING_CONVENTION.md)
- [Architecture Overview](./ARCHITECTURE.md)
