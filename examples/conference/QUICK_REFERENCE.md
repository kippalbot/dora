# Conference Debate Quick Reference

## Overview

This example reimplements the debate scenario using the new conference architecture with **sequential policy**.

## Architecture

```
Old (llm-client/debate)              New (conference/debate)

3 separate bridges               ┌─────────────────────────┐
Complex cold-start configs       │  Conference Controller  │
Hardcoded logic in monitor       │  Policy: [sequential]   │
                                 └──────┬──────────────────┘
                                        │ "resume"
                                        ↓
                                 ┌─────────────────────────┐
                                 │  Conference Bridge      │
                                 │  Auto-pause/resume      │
                                 └──────┬──────────────────┘
                                        │
                                        ↓
                                 ┌─────────────────────────┐
                                 │  Debate Monitor         │
                                 │  (3-panel TUI)          │
                                 └─────────────────────────┘

✗ Hard to maintain                ✓ Simple, declarative
✗ 150 lines config                ✓ 70 lines config
✗ Different configs per bridge    ✓ One policy pattern
```

## File Structure

```
conference/
├── README.md                           # Main documentation
├── MIGRATION.md                        # Migration guide from old debate
├── ARCHITECTURE.md                     # Detailed architecture
├── NAMING_CONVENTION.md                # Policy naming guide
├── QUICK_REFERENCE.md                  # This file
├── run_debate.sh                       # Launch script
├── dataflow-debate-sequential.yml      # Full example with monitors
├── dataflow-debate-minimal.yml         # Minimal example (no monitors)
├── debate_config_maas_llm1.toml       # LLM1: 正方, supports restoration
├── debate_config_maas_llm2.toml       # LLM2: 反方, opposes restoration
└── debate_config_maas_judge.toml      # Judge: moderator, fair assessment
```

## Quick Start

```bash
cd /Users/yuechen/home/fresh/dora/examples/conference

# Set API key
export ALIBABA_CLOUD_API_KEY="your-key"

# Run (builds if needed)
./run_debate.sh
```

## Speaking Order

Policy: `[llm1 → llm2 → judge]` (sequential)

1. **llm1** (正方) - Speaks first
2. **llm2** (反方) - Speaks second
3. **judge** (主持人) - Comments/summarizes
4. Loop back to llm1

### Change Order

```bash
# Judge speaks first
export DORA_POLICY_PATTERN="[judge → llm1 → llm2]"

# Priority: judge always first
export DORA_POLICY_PATTERN="[(judge, *), (llm1, 1), (llm2, 1)]"

# Equal ratio (no → arrows)
export DORA_POLICY_PATTERN="[llm1, llm2, judge]"

./run_debate.sh
```

## Key Differences

| Feature | Old (llm-client) | New (conference) |
|---------|------------------|------------------|
| Bridge count | 3 | 1 |
| Controller | In monitor | Separate node |
| Policy | Hardcoded | Env var |
| Auto-pause | Manual | Built-in |
| Add participant | Hard | Easy |
| Lines of YAML | 150 | 70 |

## Configuration Files

### Policy (Environment Variable)

```yaml
# In dataflow-debate-sequential.yml
env:
  DORA_POLICY_PATTERN: "[llm1 → llm2 → judge]"
```

### LLM Prompts (TOML Files)

```toml
# debate_config_maas_llm1.toml
system_prompt = "你叫大牛，是中学历史教师...支持完全恢复..."
model = "qwen-max"
temperature = 0.8
```

Change prompts to modify debate topic.

## Monitoring

### Debate Monitor (TUI)

Shows 3 panels in real-time:
```
┌───────────┬───────────┐
│  LLM1     │  LLM2     │
│  [正方]   │  [反方]   │
│           │           │
└───────────┴───────────┘
┌─────────────────────────┐
│  Judge                  │
│  [主持人]               │
└─────────────────────────┘
```

**Controls:**
- `q` or `Ctrl-C`: Quit
- `r`: Reset debate

### Viewer (Logs)

```bash
# View all logs
./viewer.py

# Save to file
./viewer.py > logs.txt 2>&1
```

## Policy Patterns

### Sequential
```yaml
DORA_POLICY_PATTERN: "[a → b → c]"  # Fixed order
```

### Priority
```yaml
DORA_POLICY_PATTERN: "[(a, *), (b, 1), (c, 1)]"  # a always first
```

### Ratio
```yaml
DORA_POLICY_PATTERN: "[(a, 2), (b, 1), (c, 1)]"  # a gets 2x time
```

### Simple
```yaml
DORA_POLICY_PATTERN: "[a, b, c]"  # Equal ratio (1:1:1)
```

## Troubleshooting

### Debate doesn't start

```bash
# Check API key
echo $ALIBABA_CLOUD_API_KEY

# Check configs
ls -la debate_config_maas_*.toml
```

### Speaking order wrong

```bash
# Verify pattern
grep DORA_POLICY_PATTERN dataflow-debate-sequential.yml

# View controller logs
dora logs conference-controller
```

### Bridge not forwarding

```bash
# Check for "resume" commands
dora logs conference-bridge | grep resume

# Verify controller outputs
# Should see: "Sending resume command"
```

## Examples Comparison

### Sequential (default)
```yaml
# [llm1 → llm2 → judge]
# Strict rotation: A → B → C → A → B → C
```

### Judge First (priority)
```yaml
# [(judge, *), (llm1, 1), (llm2, 1)]
# Judge always speaks first (unless they just spoke)
```

### Weighted (ratio)
```yaml
# [(moderator, 2), (panelist, 1)]
# Moderator gets twice the speaking time
```

## Documentation

- **README.md** - Full documentation
- **MIGRATION.md** - How to migrate from old debate
- **ARCHITECTURE.md** - Detailed architecture
- **NAMING_CONVENTION.md** - Policy naming guide

## References

- [Conference Controller API](../../node-hub/dora-conference-controller/)
- [Conference Bridge API](../../node-hub/dora-conference-bridge/)
- [MaaS Client API](../../node-hub/dora-maas-client/)
- [Original Debate](../llm-client/README.md)

## License

Apache 2.0 (same as parent project)
