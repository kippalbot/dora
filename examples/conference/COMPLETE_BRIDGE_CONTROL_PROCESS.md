# Complete Bridge Control Process

## 🌉 Bridge Control Architecture Overview

The bridge control system manages conversation flow between participants using **pause/resume signals** and **session end triggers**. Here's the complete process:

## 🔄 Bridge Control Flow Diagram

```
🎬 CONVERSATION START
        ↓
🏛️ Controller: process_next_speaker()
        ↓
🌉 Send Resume Signal → Conference Bridge
        ↓
📤 Bridge: Forward participant text to next speaker
        ↓
🤖 LLM: Process text and generate response
        ↓
📝 LLM → Text Segmenter → TTS (synthesize audio)
        ↓
🔊 TTS: Stream audio fragments + session_end signal
        ↓
🏁 Controller: handle_session_end()
        ↓
🎯 Check: Is this last participant?
        ↓
    YES 👇                    NO 👇
🌉 Send Resume → Bridge     ⏳ Wait for more participants
        ↓
🔄 Next Round Starts
```

## 📋 Detailed Step-by-Step Process

### 1. **Conversation Initiation**

```rust
// Controller starts conversation
fn process_next_speaker(&mut self, node: &mut DoraNode) -> Result<()> {
    if let Some(next_speaker) = self.policy.determine_next_speaker() {
        // Map participant to control output
        let control_output = match next_speaker.as_str() {
            "judge" => "control_judge",
            "llm2" => "control_llm2",
            "llm1" => "control_llm1",
        };

        // Generate enhanced question_id for this participant
        let participant_id = self.map_control_output_to_participant(&control_output);
        self.current_question_id = self.generate_enhanced_question_id(node, &participant_id);

        // 🌉 SEND RESUME TO BRIDGE
        node.send_output(
            DataId::from(control_output.to_string()),
            metadata_with_question_id,
            StringArray::from(vec!["resume"]),
        )?;
    }
}
```

### 2. **Bridge Processing**

**Dataflow Configuration:**
```yaml
# Example: Bridge to Tutor
- id: bridge-to-tutor
  path: ../../target/release/dora-conference-bridge
  inputs:
    # Participant outputs
    student1:
      source: student1/text
      queue_size: 1000
    student2:
      source: student2/text
      queue_size: 1000
    # Controller tells bridge when to forward
    control:
      source: conference-controller/control_judge  # ← Resume signal
      queue_size: 10
  outputs:
    - text    # → Next participant (tutor)
    - status  # → Forwarding status
    - log     # → Debug logs
```

**Bridge Logic:**
1. **Receives `resume` signal** from controller
2. **Forwards queued participant text** to next speaker
3. **Sends status updates** about forwarding progress

### 3. **LLM Processing**

```python
# MaaS Client (Student/Tutor)
class MaaSClient:
    def on_resume_signal(self):
        # Bridge forwards previous participant text
        context_text = bridge.get_forwarded_text()

        # Generate response using LLM
        response = llm.generate_response(context_text)

        # Send response to text segmenter
        self.send_output("text", response)
```

### 4. **TTS Processing with Session End**

```python
# TTS Engine (PrimeSpeech)
def synthesize_and_stream(text, metadata):
    # Process text through TTS
    for audio_fragment in synthesize_streaming(text):
        # Send audio fragments
        node.send_output("audio", audio_fragment, metadata)

    # 🏁 SEND SESSION END SIGNAL
    session_status = metadata.get("session_status", "unknown")
    if session_status in ["completed", "finished", "ended", "final"]:
        node.send_output(
            "session_end",
            pa.array(["session_ended"]),
            metadata={
                "question_id": metadata.get("question_id", "default"),
                "session_status": session_status,
                "session_id": session_id,
                "request_id": request_id,
            }
        )
```

### 5. **Controller Session End Handling**

```rust
// Controller receives session_end signal
else if id.as_str() == "session_end" {
    let question_id = extract_question_id_from_metadata();
    let session_status = extract_session_status_from_metadata();

    controller.handle_session_end(question_id, &session_status, &mut node, log_level)?;
}

fn handle_session_end(&mut self, question_id: u16, session_status: &str,
                     node: &mut DoraNode, log_level: LogLevel) -> Result<()> {
    let (_, _, _, is_last) = decode_enhanced_question_id(question_id);

    match session_status {
        "completed" | "finished" | "ended" | "final" => {
            if is_last {
                // 🌉 ROUND COMPLETED - RESUME BRIDGE FOR NEXT ROUND
                self.round_completed = true;

                node.send_output(
                    DataId::from("bridge_control".to_string()),
                    Default::default(),
                    StringArray::from(vec!["resume"]),
                )?;
            }
        }
        "error" => {
            if is_last {
                // Error still resumes to avoid hanging
                node.send_output("bridge_control", vec!["resume"])?;
            }
        }
        "cancelled" => {
            // Don't resume - cancelled doesn't count
        }
    }
}
```

### 6. **Round Advancement**

```rust
// Next time process_next_speaker is called
fn process_next_speaker(&mut self, node: &mut DoraNode) -> Result<()> {
    // Check if starting new round
    if self.round_completed {
        self.current_question_id = self.advance_to_new_round(node);
        self.round_completed = false;
        self.policy.reset_round_tracking();
    }

    // Continue with next participant selection...
}
```

## 🎯 Key Control Points

### **Bridge Resume Triggers:**

1. **Normal Conversation Progress:**
   ```
   Participant speaks → TTS completes → Session end → Last participant? → Resume bridge
   ```

2. **Error Recovery:**
   ```
   TTS error → Session end with error status → Last participant? → Resume bridge (continue)
   ```

3. **Round Completion:**
   ```
   All participants complete → Last participant session end → Controller marks round_complete → Next process_next_speaker advances round
   ```

### **Audio Buffer Backpressure:**

```rust
// Special case: Tutor audio buffer management
if is_tutor && self.should_pause_tutor_output() {
    // Defer tutor activation until audio buffer drains
    self.pending_tutor_activation = Some(control_output.to_string());
    return Ok(); // Don't send resume yet
}
```

## 📊 Signal Types and Purposes

| Signal | Source | Destination | Purpose |
|--------|--------|-------------|---------|
| `resume` (to specific bridge) | Controller | Conference Bridge | Start forwarding text to next speaker |
| `text` | Participant | Bridge | Store participant response |
| `text` | Bridge | Next LLM | Forward previous responses |
| `session_end` | TTS | Controller | Signal session completion |
| `resume` (bridge_control) | Controller | System | Trigger next round after completion |

## 🔍 Enhanced Question ID Role

The **enhanced question_id** is crucial throughout this process:

```rust
// 16-bit format: 8-4-4 layout
// Bits 15-8: Round number (0-255)
// Bits 7-4: Total participants in round (1-16)
// Bits 3-0: Participant index (0-15)

fn decode_enhanced_question_id(question_id: u16) -> (u8, u8, u8, bool) {
    let round = (question_id >> 8) as u8;
    let total_participants = ((question_id >> 4) & 0xF) + 1;
    let participant = (question_id & 0xF) as u8;
    let is_last_participant = participant + 1 == total_participants;

    (round, participant, total_participants, is_last_participant)
}
```

This embedded information allows the controller to:
- **Track which round** the session belongs to
- **Know how many participants** are in this specific round
- **Determine if this is the last participant** for round completion
- **Make precise bridge control decisions**

## 🎉 Complete Flow Summary

1. **Controller** selects next speaker → sends `resume` to specific bridge
2. **Bridge** forwards queued participant text to selected LLM
3. **LLM** processes text → generates response → sends to TTS
4. **TTS** synthesizes audio → sends `session_end` signal when complete
5. **Controller** receives `session_end` → checks if last participant
6. **If last participant**: sends `bridge_control.resume` → triggers next round
7. **Process repeats** for next round with new participants

This creates a **clean, reliable conversation flow** where each step is clearly triggered by specific signals, and the enhanced question_id provides all necessary context for intelligent bridge control.