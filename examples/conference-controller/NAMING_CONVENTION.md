# Policy Naming Strategy: Input Port Names

## Key Principle

**Use the exact input port names from your YAML configuration in the policy pattern.**

## Why This Works

1. **Direct mapping**: When an input arrives on port "judge", the controller receives `id = "judge"`
2. **Simple lookup**: The policy tracks word counts using these same names
3. **Clear intent**: Pattern self-documents which ports are participants

## Example Flow

### YAML Configuration

```yaml
nodes:
  - id: conference-controller
    operator:
      rust: dora-conference-controller
    env:
      # Pattern uses the input port names
      DORA_POLICY_PATTERN: "[judge → defense → prosecution]"
    inputs:
      judge: llm-judge/text        # Port name: "judge"
      defense: llm-defense/text    # Port name: "defense"
      prosecution: llm-prosecution/text  # Port name: "prosecution"
```

### Runtime Flow

```
1. LLM generates text on "llm-judge/text"
   ↓
2. Controller receives event: { id: "judge", data: "The evidence shows..." }
   ↓
3. Controller updates policy
   policy.update_word_count("judge", 45)  # 45 words
   ↓
4. Controller determines next speaker
   next_speaker = policy.determine_next_speaker()
   → Returns "defense" (next in sequence)
   ↓
5. Controller sends resume command
   node.send_output("control", "resume", ...)
   ↓
6. Bridge forwards message from "defense" port
```

## Comparison: Right vs Wrong

### ✅ CORRECT - Names match

```yaml
inputs:
  host: host-llm/text
  guest1: guest1-llm/text

env:
  DORA_POLICY_PATTERN: "[(host, *), (guest1, 1)]"  # ✅ Matches input ports
```

**Result**: Controller correctly tracks and selects speakers

### ❌ WRONG - Names don't match

```yaml
inputs:
  host: host-llm/text        # Port name: "host"
  guest1: guest1-llm/text    # Port name: "guest1"

env:
  DORA_POLICY_PATTERN: "[(Host, *), (Guest1, 1)]"  # ❌ Different names!
```

**Result**: Controller receives inputs with IDs "host" and "guest1", but policy looks for "Host" and "Guest1" → No matches, policy fails

## Benefits of This Approach

1. **No mapping layer needed**: Direct 1:1 correspondence
2. **Simple to understand**: Pattern shows actual port names
3. **Self-documenting**: Anyone reading the YAML can see the connection
4. **Case-sensitive**: Prevents subtle bugs from typos
5. **Flexible**: Works with any port naming scheme

## Valid Naming Patterns

### Sequential
```yaml
DORA_POLICY_PATTERN: "[participant-1 → participant-2 → participant-3]"
```

### Priority
```yaml
DORA_POLICY_PATTERN: "[(moderator, *), (speaker-a, 1), (speaker-b, 1)]"
```

### Ratio
```yaml
DORA_POLICY_PATTERN: "[(team-captain, 2), (team-member-1, 1), (team-member-2, 1)]"
```

### Simple List
```yaml
DORA_POLICY_PATTERN: "[alice, bob, charlie]"
```

## All Examples Updated

All examples in `examples/conference-controller/` now use this convention:

- `sequential-simple.yml` - `[judge → defense → prosecution]`
- `priority-interview.yml` - `[(host, *), (guest1, 1), (guest2, 1)]`
- `ratio-debate.yml` - `[(moderator, 2), (team-a, 1), (team-b, 1)]`
- `dataflow-complete.yml` - Demonstrates full flow

## Key Takeaway

**The policy pattern is not abstract - it's literal input port names.**

When you see:
```yaml
DORA_POLICY_PATTERN: "[alice → bob]"
```

It means: "When input arrives on port 'alice', then next allow port 'bob' to speak."

No indirection. No mapping. Direct and simple.
