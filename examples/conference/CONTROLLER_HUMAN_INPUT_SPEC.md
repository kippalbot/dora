# Controller Human Input Implementation Specification

## Overview

This document specifies the Rust code changes required in `dora-conference-controller` to handle human speaker input with proper interrupt, reset, and restart logic.

## File Location

`/Users/yuechen/home/fresh/dora/node-hub/dora-conference-controller/src/main.rs`

---

## Critical Distinction: Streaming vs Non-Streaming

### AI Participants (LLMs) - STREAMING
```
LLM Output Flow:
  session_status="started"     → Controller: Start accumulating
          ↓
  session_status="streaming"   → Controller: Continue accumulating
          ↓
  session_status="ended"       → Controller: Message complete, process

Controller must ACCUMULATE chunks over time
```

### Human Participant (ASR) - NON-STREAMING
```
Human Output Flow:
  session_status="ended"       → Controller: Complete transcription, process immediately

ASR sends SINGLE complete message
Controller processes IMMEDIATELY (no accumulation needed)
```

**Key Implication**:
- Human input handler does NOT need to check `is_message_complete()`
- Human input is ALWAYS complete when it arrives
- We can immediately trigger interrupt sequence

---

## Change 1: Add system_paused Field to ConferenceController Struct

**Location**: Line 118-133 (struct definition)

**Current Code**:
```rust
struct ConferenceController {
    state: ControllerState,
    policy: UnifiedRatioPolicy,
    participant_inputs: HashMap<String, ParticipantInput>,
    streaming_accumulators: HashMap<String, StreamingAccumulator>,
    pattern: String,
    log_level: LogLevel,
    reset_pending: bool,
    participant_name_map: HashMap<String, String>,
    participant_index_map: HashMap<String, u8>,
    current_question_id: u16,

    // New session-start based resume control
    waiting_for_session_start: Option<u16>,
    pending_next_speaker: bool,
}
```

**Add**:
```rust
struct ConferenceController {
    state: ControllerState,
    policy: UnifiedRatioPolicy,
    participant_inputs: HashMap<String, ParticipantInput>,
    streaming_accumulators: HashMap<String, StreamingAccumulator>,
    pattern: String,
    log_level: LogLevel,
    reset_pending: bool,
    participant_name_map: HashMap<String, String>,
    participant_index_map: HashMap<String, u8>,
    current_question_id: u16,

    // New session-start based resume control
    waiting_for_session_start: Option<u16>,
    pending_next_speaker: bool,

    // NEW: Human interrupt control
    system_paused: bool,  // True when human is speaking or processing human input
}
```

---

## Change 2: Initialize system_paused in Constructor

**Location**: Line 136-200 (ConferenceController::new)

**Current Code** (end of constructor):
```rust
Ok(ConferenceController {
    state: ControllerState::Waiting,
    policy,
    participant_inputs: HashMap::new(),
    streaming_accumulators: HashMap::new(),
    pattern: pattern.clone(),
    log_level,
    reset_pending: false,
    participant_name_map,
    participant_index_map,
    current_question_id: initial_question_id,
    waiting_for_session_start: None,
    pending_next_speaker: false,
})
```

**Add**:
```rust
Ok(ConferenceController {
    state: ControllerState::Waiting,
    policy,
    participant_inputs: HashMap::new(),
    streaming_accumulators: HashMap::new(),
    pattern: pattern.clone(),
    log_level,
    reset_pending: false,
    participant_name_map,
    participant_index_map,
    current_question_id: initial_question_id,
    waiting_for_session_start: None,
    pending_next_speaker: false,
    system_paused: false,  // NEW: Initialize as not paused
})
```

---

## Change 3: Add Human Input Detection in handle_participant_input

**Location**: Line 269-450 (handle_participant_input method)

**Important Context**:
- **Human input is NOT streaming** - ASR sends single complete transcription
- **Human input always has `session_status="ended"`** - Added by ASR modification
- **AI participants ARE streaming** - LLM sends "started" → "streaming" → "ended"

**Current Code** (line 269):
```rust
fn handle_participant_input(
    &mut self,
    participant_id: &str,
    text: String,
    metadata: &dora_node_api::Metadata,
    node: &mut DoraNode,
) -> Result<()> {
    // Existing accumulation logic for streaming AI participants...
}
```

**Add at the BEGINNING of the method** (after line 273):
```rust
fn handle_participant_input(
    &mut self,
    participant_id: &str,
    text: String,
    metadata: &dora_node_api::Metadata,
    node: &mut DoraNode,
) -> Result<()> {
    // NEW: Special handling for human input (non-streaming)
    // Human input always arrives with session_status="ended" (single shot from ASR)
    if participant_id == "human" {
        return self.handle_human_input(participant_id, text, metadata, node);
    }

    // Existing accumulation logic for AI participants (streaming)...
    let session_status = if let Some(Parameter::String(status)) = metadata.parameters.get("session_status") {
        status.as_str()
    } else {
        "unknown"
    };

    // ... rest of existing code for streaming accumulation ...
}
```

---

## Change 4: Implement handle_human_input Method

**Location**: Add new method after handle_participant_input (around line 450)

**Important**: Human input is **non-streaming** and **always complete** (ASR sends full transcription with `session_status="ended"`), so we don't need to check completion status - we can immediately trigger the reset sequence.

**Add Complete Method**:
```rust
/// Handle input from human speaker (via ASR)
/// Human input is non-streaming - always arrives complete with session_status="ended"
/// When human speaks, interrupt all AI participants and reset system to initial state
fn handle_human_input(
    &mut self,
    participant_id: &str,
    text: String,
    metadata: &dora_node_api::Metadata,
    node: &mut DoraNode,
) -> Result<()> {
    send_log(node, LogLevel::Info, self.log_level,
        &format!("👤 Human input received: '{}'",
                 text.chars().take(100).collect::<String>()));

    // Human input is always complete (non-streaming ASR output)
    // ASR modification ensures session_status="ended" is always present
    // So we immediately trigger interrupt sequence

    // 1. Mark system as paused
    self.system_paused = true;

    // 2. Store current question_id for logging
    let old_question_id = self.current_question_id;

    // 3. ENCODE NEW question_id for next round (CRITICAL!)
    // Question ID uses 16-bit encoding (8-4-4 layout):
    //   Bits 15-8: Round number (0-255)
    //   Bits 7-4: Total participants - 1 (0-15)
    //   Bits 3-0: Current participant index (0-15)
    // We increment the ROUND number and reset to first participant (tutor)
    let (current_round, _, _, _) = decode_enhanced_question_id(old_question_id);
    let new_round = current_round.wrapping_add(1);  // Increment round number
    let total_participants = self.policy.get_participants().len() as u8;

    // Encode new question_id: new round, participant 0 (will be tutor after reset)
    self.current_question_id = encode_enhanced_question_id(
        new_round,
        0,  // Start from first participant (tutor speaks first)
        total_participants
    );

    send_log(node, LogLevel::Info, self.log_level,
        &format!("📈 Encoded new question_id: {} → {} ({})",
                 old_question_id,
                 self.current_question_id,
                 enhanced_id_debug_string(self.current_question_id)));

    // 4. Cancel all LLMs with NEW question_id
    // LLMs will abort streaming and propagate question_id to downstream
    self.send_cancel_to_all_llms(node)?;

    // 5. Reset all bridges with NEW question_id
    // Bridges will clear buffered messages
    self.send_reset_to_all_bridges(node)?;

    // 6. Reset audio pipeline (text-segmenter + audio-player) with NEW question_id
    // Text-segmenter: discards segments with old question_id, keeps new
    // Audio-player: discards audio with old question_id, keeps new
    self.send_reset_to_audio_pipeline(node)?;

    // 7. Reset controller state to initial (tutor speaks first, cycle=0)
    self.reset_to_initial_state(node)?;

    // 8. Resume system
    self.system_paused = false;

    send_log(node, LogLevel::Info, self.log_level,
        "✅ System reset complete - ready for new round");

    Ok(())
}
```

---

## Change 5: Implement send_cancel_to_all_llms Method

**Location**: Add new method after handle_human_input

**Add Complete Method**:
```rust
/// Send cancel signal to all LLM participants with NEW question_id
fn send_cancel_to_all_llms(&self, node: &mut DoraNode) -> Result<()> {
    // Create metadata with NEW question_id
    let mut cancel_metadata = std::collections::BTreeMap::new();
    cancel_metadata.insert(
        "command".to_string(),
        Parameter::String("cancel".to_string())
    );
    cancel_metadata.insert(
        "question_id".to_string(),
        Parameter::String(self.current_question_id.to_string())
    );

    // Send to student1 and student2 via llm_control
    node.send_output(
        DataId::from("llm_control".to_string()),
        cancel_metadata.clone(),
        StringArray::from(vec!["cancel"]),
    )?;

    // Send to tutor via judge_prompt
    node.send_output(
        DataId::from("judge_prompt".to_string()),
        cancel_metadata.clone(),
        StringArray::from(vec!["cancel"]),
    )?;

    send_log(node, LogLevel::Debug, self.log_level,
        &format!("🛑 Sent cancel to all LLMs with question_id={}",
                 self.current_question_id));

    Ok(())
}
```

---

## Change 6: Implement send_reset_to_all_bridges Method

**Location**: Add new method after send_cancel_to_all_llms

**Add Complete Method**:
```rust
/// Send reset signal to all bridges with NEW question_id
fn send_reset_to_all_bridges(&self, node: &mut DoraNode) -> Result<()> {
    // Create metadata with NEW question_id
    let mut reset_metadata = std::collections::BTreeMap::new();
    reset_metadata.insert(
        "command".to_string(),
        Parameter::String("reset".to_string())
    );
    reset_metadata.insert(
        "question_id".to_string(),
        Parameter::String(self.current_question_id.to_string())
    );

    // Send reset to all bridge control outputs
    node.send_output(
        DataId::from("control_judge".to_string()),
        reset_metadata.clone(),
        StringArray::from(vec!["reset"]),
    )?;

    node.send_output(
        DataId::from("control_llm1".to_string()),
        reset_metadata.clone(),
        StringArray::from(vec!["reset"]),
    )?;

    node.send_output(
        DataId::from("control_llm2".to_string()),
        reset_metadata.clone(),
        StringArray::from(vec!["reset"]),
    )?;

    send_log(node, LogLevel::Debug, self.log_level,
        &format!("🔄 Sent reset to all bridges with question_id={}",
                 self.current_question_id));

    Ok(())
}
```

---

## Change 7: Implement send_reset_to_audio_pipeline Method

**Location**: Add new method after send_reset_to_all_bridges

**Add Complete Method**:
```rust
/// Send reset signal to audio pipeline (text-segmenter + audio-player) with NEW question_id
fn send_reset_to_audio_pipeline(&self, node: &mut DoraNode) -> Result<()> {
    // Create metadata with NEW question_id
    let mut reset_metadata = std::collections::BTreeMap::new();
    reset_metadata.insert(
        "command".to_string(),
        Parameter::String("reset".to_string())
    );
    reset_metadata.insert(
        "question_id".to_string(),
        Parameter::String(self.current_question_id.to_string())
    );

    // Send reset to llm_control (will reach text-segmenter)
    // Text-segmenter will discard segments with question_id != current_question_id
    // Audio-player will receive reset via its reset input (configured in YAML)
    node.send_output(
        DataId::from("llm_control".to_string()),
        reset_metadata.clone(),
        StringArray::from(vec!["reset"]),
    )?;

    send_log(node, LogLevel::Debug, self.log_level,
        &format!("🔄 Sent reset to audio pipeline with question_id={}",
                 self.current_question_id));

    Ok(())
}
```

---

## Change 8: Implement reset_to_initial_state Method

**Location**: Add new method after send_reset_to_audio_pipeline

**Add Complete Method**:
```rust
/// Reset controller to initial state (tutor speaks first, cycle=0)
fn reset_to_initial_state(&mut self, node: &mut DoraNode) -> Result<()> {
    send_log(node, LogLevel::Info, self.log_level,
        "🔄 Resetting controller to initial state");

    // 1. Clear all accumulated inputs
    self.participant_inputs.clear();

    // 2. Clear streaming accumulators
    self.streaming_accumulators.clear();

    // 3. Reset state
    self.state = ControllerState::Waiting;
    self.reset_pending = false;
    self.waiting_for_session_start = None;
    self.pending_next_speaker = false;

    // 4. Reset policy to initial state
    self.policy.reset();

    send_log(node, LogLevel::Info, self.log_level,
        &format!("✅ Reset complete - ready to start with question_id={} ({})",
                 self.current_question_id,
                 enhanced_id_debug_string(self.current_question_id)));

    // 5. Trigger initial speaker (tutor)
    // Use existing resume logic to start first speaker
    self.resume_next_speaker(node)?;

    Ok(())
}
```

---

## Change 9: Update Main Event Loop (Optional Check)

**Location**: Line 694-706 (main event loop)

**Current Code**:
```rust
} else {
    // Participant input - extract text
    let text_array = data.as_string::<i32>();
    let text = text_array
        .iter()
        .filter_map(|s| s)
        .collect::<Vec<_>>()
        .join(" ");

    send_log(&mut node, LogLevel::Debug, log_level, &format!("📨 Processing input from {}", id));

    if let Err(e) = controller.handle_participant_input(id.as_str(), text, &metadata, &mut node) {
        send_log(&mut node, LogLevel::Error, log_level, &format!("❌ Error handling input: {}", e));
    }
}
```

**No changes needed** - handle_participant_input will route to handle_human_input automatically.

---

## Summary of Changes

### New Fields (1)
- `system_paused: bool` in ConferenceController struct

### New Methods (5)
1. `handle_human_input()` - Main human input handler
2. `send_cancel_to_all_llms()` - Cancel with question_id
3. `send_reset_to_all_bridges()` - Reset with question_id
4. `send_reset_to_audio_pipeline()` - Reset with question_id
5. `reset_to_initial_state()` - Reset controller state

### Modified Methods (1)
- `handle_participant_input()` - Add human detection at start

### Total Lines of Code to Add
- ~200 lines of new Rust code

---

## Testing Checklist

After implementation:

- [ ] Compile without errors: `cargo build --release`
- [ ] Controller detects human input (participant_id == "human")
- [ ] question_id increments when human finishes speaking
- [ ] Cancel signals sent to all LLMs with NEW question_id
- [ ] Reset signals sent to bridges with NEW question_id
- [ ] Reset signals sent to audio pipeline with NEW question_id
- [ ] Text-segmenter discards old segments (question_id filtering)
- [ ] Audio-player discards old audio (question_id filtering)
- [ ] System restarts from initial state (tutor speaks first)
- [ ] Logging shows all steps: increment, cancel, reset, restart

---

## Implementation Steps

1. Add `system_paused` field to struct and constructor
2. Add `handle_human_input()` method
3. Add three helper methods: `send_cancel_to_all_llms()`, `send_reset_to_all_bridges()`, `send_reset_to_audio_pipeline()`
4. Add `reset_to_initial_state()` method
5. Modify `handle_participant_input()` to detect human
6. Compile and test
7. Run with `dataflow-study-multi-aec.yml`

---

## Question ID Encoding Algorithm

### Overview

The controller uses an **enhanced 16-bit question_id** encoding (not a simple counter) with the following bit layout:

```
Bits 15-8: Round number (0-255)
Bits 7-4:  Total participants - 1 (0-15, so 1-16 participants)
Bits 3-0:  Current participant index (0-15)
```

### Encoding Functions (Already in main.rs)

```rust
fn encode_enhanced_question_id(round: u8, participant: u8, total_participants: u8) -> u16 {
    let round_bits = (round as u16) << 8;
    let total_bits = ((total_participants - 1) as u16) << 4;
    let participant_bits = participant as u16;
    round_bits | total_bits | participant_bits
}

fn decode_enhanced_question_id(question_id: u16) -> (u8, u8, u8, bool) {
    let round = (question_id >> 8) as u8;
    let total_participants = ((question_id >> 4) & 0xF) + 1;
    let participant = (question_id & 0xF) as u8;
    let is_last_participant = participant + 1 == total_participants as u8;
    (round, participant, total_participants as u8, is_last_participant)
}
```

### Human Interrupt Algorithm

When human speaks, we need to **increment the ROUND number** (not the whole question_id):

```rust
// DON'T DO THIS (wrong - changes participant index):
self.current_question_id += 1;  // ❌ INCORRECT

// DO THIS (correct - increments round number):
let (current_round, _, _, _) = decode_enhanced_question_id(self.current_question_id);
let new_round = current_round.wrapping_add(1);
let total_participants = self.policy.get_participants().len() as u8;
self.current_question_id = encode_enhanced_question_id(
    new_round,
    0,  // Reset to first participant (tutor)
    total_participants
);  // ✅ CORRECT
```

### Example Calculation

Suppose we have 4 participants (human, tutor, student1, student2) and we're in round 5:

```
Current state: Round 5, Student2 speaking (participant index 3)
Current question_id: encode_enhanced_question_id(5, 3, 4)
                   = 0x0503 | 0x0030 | 0x0003
                   = 0x0533
                   → Decoded: R6P4/4 (round 5+1=6, participant 3+1=4, total 4)

Human interrupts:
1. Decode: (round=5, participant=3, total=4, is_last=true)
2. Increment: new_round = 5 + 1 = 6
3. Encode new: encode_enhanced_question_id(6, 0, 4)
              = 0x0600 | 0x0030 | 0x0000
              = 0x0630
              → Decoded: R7P1/4 (round 6+1=7, participant 0+1=1, total 4)

New question_id: 0x0630 (round 6, tutor speaks first)
```

**Why this matters**:
- Simple `+= 1` would give: 0x0533 + 1 = 0x0534 (changes participant to 4, invalid!)
- Correct encoding gives: 0x0630 (new round, first participant)

---

## Key Design Points

### 1. Why encode NEW question_id BEFORE sending reset/cancel?

The NEW question_id must be in the reset/cancel signals so downstream components know which data to KEEP and which to DISCARD.

**Question ID Encoding (16-bit: 8-4-4 layout)**:
- Bits 15-8: Round number (0-255)
- Bits 7-4: Total participants - 1 (0-15)
- Bits 3-0: Current participant index (0-15)

**Algorithm**:
1. Decode current question_id to extract round number
2. Increment round number using `wrapping_add(1)`
3. Encode new question_id with: new_round, participant=0 (tutor), total_participants

Example:
```
Current round: question_id=0x0523 (round=5, total=3, participant=3)
                           → Decoded: R5P4/4 (round 5, last participant)
Human speaks: question_id=0x0523 (from mac-aec metadata)
Controller decodes: round=5
Controller increments: new_round=6
Controller encodes: encode_enhanced_question_id(6, 0, 4)
                   → question_id=0x0630 (round=6, total=4, participant=0)
                   → R6P1/4 (round 6, tutor speaks first)
Reset signal: question_id=0x0630 (tells downstream: keep only qid=0x0630)
Text-segmenter: Discards segments with qid=0x0523, keeps qid=0x0630
Audio-player: Discards audio with qid=0x0523, keeps qid=0x0630
```

### 2. Why send cancel AND reset?

- **Cancel to LLMs**: Stop generation immediately, keep conversation history
- **Reset to Bridges**: Clear buffered messages
- **Reset to Audio Pipeline**: Discard queued segments/audio

### 3. Why use llm_control for audio pipeline reset?

The YAML wiring connects:
```yaml
multi-text-segmenter:
  inputs:
    control: conference-controller/llm_control  # Receives reset here

audio-player:
  inputs:
    reset: conference-controller/llm_control  # Also receives reset here (in multi-aec dataflow)
```

So sending to `llm_control` reaches both text-segmenter and audio-player.

---

## References

- **Architecture Doc**: `ARCHITECTURE_MULTI_AEC.md`
- **Quick Start**: `QUICKSTART_MULTI_AEC.md`
- **Existing Methods**: See `handle_participant_input()` (line 269) for pattern
- **Metadata Usage**: See `is_message_complete()` (line 211) for session_status checking

---

**Last Updated**: 2025-12-01
**Status**: Specification Complete - Ready for Implementation
