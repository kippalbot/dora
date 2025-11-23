# Conference Controller Examples

This directory contains example dataflow configurations demonstrating the ergonomic pattern-based policy system.

## Pattern-Based Configuration

**No separate `type` field needed!** The pattern string itself determines the behavior:

- `→` arrows = Sequential mode
- `(Name, weight)` = Ratio/priority mode
- Plain names = Equal ratio mode

## Examples Overview

### 1. Sequential: Courtroom Debate

**File**: `sequential-simple.yml`

**Pattern**:
```yaml
DORA_POLICY_PATTERN: "[Judge → Defense → Prosecution]"
```

**Behavior**:
- Fixed speaking order
- Cycles back to Judge after Prosecution
- Everyone gets equal air time

**Run**:
```bash
dora start sequential-simple.yml
```

**Expected Output Sequence**:
```
Judge speaks → Defense speaks → Prosecution speaks → Judge speaks → ...
```

---

### 2. Priority: Interview Show

**File**: `priority-interview.yml`

**Pattern**:
```yaml
DORA_POLICY_PATTERN: "[(Host, *), (Guest1, 1), (Guest2, 1)]"
```

**Behavior**:
- Host (`*`) always speaks first (unless they just spoke)
- Guests share remaining time equally (1:1 ratio)
- Dynamic selection based on word counts

**Run**:
```bash
dora start priority-interview.yml
```

**Expected Flow**:
```
Initial: Host speaks (priority)
        ↓
Guest1 or Guest2 speaks (ratio-based)
        ↓
Host speaks again (priority)
        ↓
Other guest responds (ratio-based)
        ↓
...and so on
```

---

### 3. Ratio: Balanced Debate

**File**: `ratio-debate.yml`

**Pattern**:
```yaml
DORA_POLICY_PATTERN: "[(Moderator, 2), (TeamA, 1), (TeamB, 1)]"
```

**Behavior**:
- Moderator gets 2x speaking time of each team
- Teams A and B share equally
- Fair distribution based on actual word counts
- Ratio: Moderator:TeamA:TeamB = 2:1:1

**Run**:
```bash
dora start ratio-debate.yml
```

**Distribution Analysis**:
- 10 turns total:
- Moderator: ~40% (4 turns, ~400 words)
- TeamA: ~30% (3 turns, ~300 words)
- TeamB: ~30% (3 turns, ~300 words)

---

## Quick Start

### Prerequisites

```bash
# Set up environment
cd /Users/yuechen/home/fresh/dora

# Build the controller
cargo build -p dora-conference-controller --release

# Build the bridge
cargo build -p dora-conference-bridge --release
```

### Running Examples

```bash
cd examples/conference-controller

# Example 1: Sequential
dora start sequential-simple.yml

# Example 2: Priority
dora start priority-interview.yml

# Example 3: Ratio
dora start ratio-debate.yml
```

---

## Configuration Comparison

### Traditional (Explicit Type)

**What we AVOIDED**:
```yaml
# ❌ More verbose, separate type field
- id: conference-controller
  operator:
    rust: dora-conference-controller
  env:
    DORA_POLICY_TYPE: "sequential"
    DORA_POLICY_SEQUENCE: "Judge,Defense,Prosecution"
```

### Pattern-Based (Ergonomic)

**What we implemented**:
```yaml
# ✅ Pattern determines type automatically
- id: conference-controller
  operator:
    rust: dora-conference-controller
  env:
    DORA_POLICY_PATTERN: "[Judge → Defense → Prosecution]"
```

**Benefits**:
- ✅ **50% less configuration**
- ✅ **Self-documenting**: `→` arrows clearly show sequential mode
- ✅ **Single field**: No separate `type` needed
- ✅ **Error-resistant**: Invalid patterns rejected at startup
- ✅ **Flexible**: Easy to switch modes by changing pattern

---

## Pattern-Specific Behaviors

### Sequential Mode (`→` arrows)

**Pattern**: `[A → B → C]`

**Details**:
- Always cycles in fixed order
- No word count tracking needed
- Predictable sequence
- Best for: Structured debates, round-robin

**Implementation**:
- `conference-controller/src/policies/unified_ratio.rs:Sequential`
- Uses simple position tracking

---

### Priority Mode (`*` marker)

**Pattern**: `[(Host, *), (Guest1, 1), (Guest2, 1)]`

**Details**:
- `*` = priority marker (not a numeric weight)
- Priority speakers always go first (unless they just spoke)
- Non-priority speakers selected by ratio
- Word counts tracked for fair distribution

**Priority Logic**:
```rust
// From unified_ratio.rs
if weight == "*" {
    // Priority speaker
    if last_speaker != this_speaker {
        select_this_speaker(); // Prefer priority
    }
}
```

**Best for**: Interviews, debates with moderator

---

### Ratio Mode (numeric weights)

**Pattern**: `[(A, 2), (B, 1), (C, 1)]`

**Details**:
- Numeric ratios determine speaking time
- Actual word counts tracked
- Dynamic selection based on deviation from ideal ratio
- Formula: `(ideal_words - actual_words) / weight`

**Ratio Calculation**:
```rust
// From unified_ratio.rs
let ideal_words = (weight / total_weight) * total_actual_words;
let ratio_difference = (ideal_words - actual_words) / weight;
// Select speaker with maximum ratio_difference
```

**Best for**: Balanced discussions, weighted participation

---

## Testing

### Run Tests

```bash
cd /Users/yuechen/home/fresh/dora

# Test policy parsing
cargo test -p dora-conference-controller policies::tests

# Test sequential mode
cargo test -p dora-conference-controller test_policy_sequential

# Test priority mode
cargo test -p dora-conference-controller test_policy_ratio_priority_with_priority

# Test ratio mode
cargo test -p dora-conference-controller test_policy_ratio_based
```

### Manual Testing

```python
from dora import Node
import os

# Set pattern
os.environ['DORA_POLICY_PATTERN'] = "[A → B → C]"

# Start dora
dora.start("sequential-simple.yml")

# Or test interactively
node = Node()

# Send participant input
node.send_output("judge", "The following evidence is clear.")

# Should see conference-controller send resume command
# Then conference-bridge forward the message
```

---

## Troubleshooting

### Invalid Pattern Error

**Problem**: Pattern parsing fails

**Solution**: Check pattern syntax
```bash
# ❌ Wrong: Missing brackets
DORA_POLICY_PATTERN: "A → B → C"

# ✅ Correct: With brackets
DORA_POLICY_PATTERN: "[A → B → C]"
```

---

### Priority Not Working

**Problem**: Priority speaker doesn't get preference

**Check**: Is the priority speaker the same as last speaker?
```rust
// From code: Priority only applies if not last_speaker
if self.last_speaker.as_deref() != Some(participant) {
    priority_candidates.push(participant); // Only add if not last
}
```

This prevents the same speaker from going twice in a row.

---

### Ratio Distribution Unbalanced

**Problem**: Speaker ratios don't match expected weights

**Explanation**: Controller uses word counts, not turn counts

**Example**:
- Participant A speaks: 200 words
- Participant B speaks: 50 words
- Next selection: B gets priority (further from ideal ratio)

This is intentional - it ensures fair **time**, not fair **turns**.

---

## Integration with Conference Bridge

### Full Pipeline

```
LLMs → Conference Controller → Conference Bridge → Output
 (pattern-based selection)      (pause/resume)       (combined)
                                                        ↓
                                                status + stats
```

### Control Flow

1. LLMs generate text
2. Controller receives input via participant ports
3. Controller updates word counts
4. Controller determines next speaker using policy pattern
5. Controller sends `resume` command to bridge
6. Bridge forwards message for selected speaker
7. Bridge auto-pauses (waits for next `resume`)
8. Cycle repeats

---

## Performance

- **Policy Decision**: < 1ms (parsing + selection)
- **Word Counting**: O(n) where n = participants
- **Memory**: O(n) for word count hash map
- **Scales to**: 100+ participants easily

---

## Extending

### Add New Policy Mode

Edit `src/policies/unified_ratio.rs`:

```rust
// Add new pattern variant
pub enum PolicyPattern {
    RatioPriority { ... },
    Sequential { ... },
    YourNewMode { ... }, // ← Add here
}

// Update parser
self.parse_your_new_mode(pattern)?;

// Implement logic
determine_your_new_mode_speaker(...)?;
```

### Custom Pattern Syntax

Add to `PatternParser::parse()`:

```rust
if pattern.contains('@') {
    return self.parse_custom_syntax(pattern);
}
```

---

## Related Docs

- **Conference Bridge**: [dora-conference-bridge/README.md](../dora-conference-bridge/README.md)
- **Controller API**: [dora-conference-controller/README.md](../dora-conference-controller/README.md)
- **Policy Module**: [dora-conference-controller/src/policies/](../dora-conference-controller/src/policies/)
- **MaaS Client**: [dora-maas-client/API.md](../dora-maas-client/API.md)

---

## Summary

| Example | Pattern | Mode | Use Case |
|---------|---------|------|----------|
| sequential-simple.yml | `[Judge → Defense → Prosecution]` | Sequential | Courtroom |
| priority-interview.yml | `[(Host, *), (Guest1, 1), (Guest2, 1)]` | Priority | Interview show |
| ratio-debate.yml | `[(Moderator, 2), (TeamA, 1), (TeamB, 1)]` | Ratio | Panel debate |

**Key Feature**: Each pattern is self-describing. No separate `type` field needed!
