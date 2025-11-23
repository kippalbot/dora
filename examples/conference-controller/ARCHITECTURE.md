# Conference Controller & Bridge Architecture

## Two Separate Nodes

The system consists of **two separate nodes** with different responsibilities:

```
┌─────────────────────────────────────────────────────────────┐
│                    CONFERENCE CONTROLLER                     │
│                                                              │
│  • Reads DORA_POLICY_PATTERN env variable                  │
│  • Runs policy logic (sequential, priority, ratio)         │
│  • Tracks word counts                                      │
│  • Decides who speaks next                                 │
│  • Sends "resume" commands to bridge                       │
│                                                              │
│  Configuration: YES (pattern in env)                       │
│  Input ports: Participant + control ports                  │
│  Output: control commands to bridge                        │
└───────────────────────┬──────────────────────────────────────┘
                        │ "resume" command
                        ↓
┌─────────────────────────────────────────────────────────────┐
│                    CONFERENCE BRIDGE                         │
│                                                              │
│  • Receives "resume" command from controller               │
│  • Forwards ONE message cycle                              │
│  • Auto-pauses after forwarding                            │
│  • NO policy logic - just executes commands                │
│                                                              │
│  Configuration: NONE (no policy needed)                    │
│  Input ports: Any participant + control                    │
│  Output: Combined text stream                              │
└─────────────────────────────────────────────────────────────┘
```

## Question 1: What names should be used in the policy?

**Answer: Use the exact input port names from your YAML configuration.**

### Example

```yaml
nodes:
  - id: conference-controller
    operator:
      rust: dora-conference-controller
    env:
      # ✅ Use these names - they match input ports below
      DORA_POLICY_PATTERN: "[judge → defense → prosecution]"
    inputs:
      judge: llm-judge/text        # Port name: "judge"
      defense: llm-defense/text    # Port name: "defense"
      prosecution: llm-prosecution/text  # Port name: "prosecution"
```

**Why?** When a message arrives:
1. Controller receives: `Event { id: "judge", data: "..." }`
2. Controller calls: `policy.update_word_count("judge", 45)`
3. Policy must recognize "judge" as a participant
4. Otherwise: `"judge" not found in participants` → Error

**Rule of thumb**: The pattern uses the **keys** from the `inputs:` section.

---

## Question 2: Can conference bridge node read from YAML env variable?

**Answer: No, and it doesn't need to!**

### Bridge Configuration

```yaml
nodes:
  - id: conference-bridge
    operator:
      rust: dora-conference-bridge
    # ❌ NO POLICY CONFIGURATION NEEDED
    # env:
    #   DORA_POLICY_PATTERN: "..."  <- NOT READ BY BRIDGE
    inputs:
      judge: llm-judge/text
      defense: llm-defense/text
      prosecution: llm-prosecution/text
      control: conference-controller/control  # ← Controller sends commands here
    outputs:
      - text
```

### What the Bridge Actually Does

```rust
// Bridge main loop (simplified):
loop {
    match event {
        Event::Input { id: "control", data } => {
            // Only reads control commands
            if data == "resume" {
                // Forward ONE message bundle
                forward_bundle();
                // Auto-pause
                state = Paused;
            }
        }
        Event::Input { id, data } if id != "control" => {
            // Store participant messages in queues
            queues.get_mut(id).push(data);
        }
    }
}
```

**The bridge is intentionally "dumb":**
- ❌ Does NOT read policy patterns
- ❌ Does NOT track word counts
- ❌ Does NOT decide who speaks
- ✅ DOES execute controller commands
- ✅ DOES forward messages when told
- ✅ DOES auto-pause after one cycle

---

## Separation of Concerns

| Aspect | Controller | Bridge |
|--------|------------|--------|
| **Configuration** | Reads `DORA_POLICY_PATTERN` | No policy config needed |
| **Logic** | Smart: runs policy algorithm | Dumb: executes commands |
| **State** | Word counts, turn history | Message queues, bridge state |
| **Decision** | Who speaks next | What to forward |
| **Control** | Sends commands | Receives commands |
| **Inputs** | Participant text + control | Any ports + control |
| **Outputs** | control commands + stats | Combined text stream |

---

## Complete Dataflow

```yaml
nodes:
  # ============ Participants (LLMs) ============
  - id: llm-judge
    outputs:
      - text

  - id: llm-defense
    outputs:
      - text

  - id: llm-prosecution
    outputs:
      - text

  # ============ Controller (brain) ============
  - id: conference-controller
    operator:
      rust: dora-conference-controller
    env:
      # 🧠 Controller reads policy pattern
      DORA_POLICY_PATTERN: "[judge → defense → prosecution]"
    inputs:
      # When these receive data, controller updates policy
      judge: llm-judge/text           # ← "judge" messages
      defense: llm-defense/text       # ← "defense" messages
      prosecution: llm-prosecution/text  # ← "prosecution" messages
      control: reset-button/status    # ← "reset" commands
    outputs:
      - control     # → Sends "resume" to bridge
      - status      # → Statistics

  # ============ Bridge (switch) ============
  - id: conference-bridge
    operator:
      rust: dora-conference-bridge
    # No env vars - bridge doesn't need policy!
    inputs:
      # Bridge receives and queues all messages
      judge: llm-judge/text
      defense: llm-defense/text
      prosecution: llm-prosecution/text
      # Controller tells bridge when to forward
      control: conference-controller/control  # ← "resume" commands
    outputs:
      - text  # → Forwarded messages to downstream nodes

  # ============ Output ============
  - id: terminal
    operator:
      rust: terminal-print
    inputs:
      data: conference-bridge/text
```

**Message Flow:**
```
LLMs → Controller (updates policy) → Controller sends "resume" → Bridge forwards → Terminal
        ↑                                                 ↑
        └─────────────── Controls who speaks ───────────┘
                                                          └─ Forwards one message
```

---

## Key Insights

### 1. Single Source of Truth

The YAML `inputs:` section defines the participant names once:
```yaml
inputs:
  judge: ...    # ← "judge" is the canonical name
  defense: ...  # ← "defense" is the canonical name
```

Both the pattern AND the runtime behavior use these exact names.

### 2. Controller is Configurable

The controller **does** read environment variables:
```yaml
env:
  DORA_POLICY_PATTERN: "[judge → defense → prosecution]"
```

But ONLY the controller reads this. The bridge doesn't care about the pattern.

### 3. Bridge is Stateless (Regarding Policy)

The bridge doesn't need to know:
- What the policy is
- Who should speak next
- What the word counts are

It just waits for "resume" and forwards one cycle.

---

## Summary

✅ **Policy names = Input port names** (case-sensitive, exact match)

✅ **Controller reads policy pattern** (env: `DORA_POLICY_PATTERN`)

❌ **Bridge does NOT read policy pattern** (no env vars needed)

This separation makes the system:
- **Modular**: Controller can change policy without affecting bridge
- **Simple**: Each node has one clear responsibility
- **Flexible**: Any policy can control the same bridge
- **Testable**: Controller logic can be tested independently
