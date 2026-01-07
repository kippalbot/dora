# MoFA Studio - Dora Integration Checklist

> Consolidated from: roadmap-claude.md, roadmap-m2.md, roadmap-glm.md, mofa-studio-roadmap.m2, mofa-studio-roadmap.claude

---

## P0: Critical (Do First) - Blocking Production

### P0.1 - Buffer Status Measurement ✅ COMPLETE

**Problem:** Buffer status must be measured from actual circular buffer, not estimated.

**Solution Implemented:**
```rust
// apps/mofa-fm/src/screen.rs:1089-1096
// Send actual buffer fill percentage to dora for backpressure control
// This replaces the bridge's estimation with the real value from AudioPlayer
if let Some(ref player) = self.audio_player {
    let fill_percentage = player.buffer_fill_percentage();
    if let Some(ref dora) = self.dora_integration {
        dora.send_command(DoraCommand::UpdateBufferStatus { fill_percentage });
    }
}
```

**Data Flow:**
1. Audio timer (50ms) triggers in screen.rs
2. Gets real buffer status: `audio_player.buffer_fill_percentage()`
3. Sends to DoraIntegration via `UpdateBufferStatus` command
4. DoraIntegration worker routes to bridge (dora_integration.rs:315-327)
5. Bridge sends to Dora via `send_buffer_status_to_dora()` (audio_player.rs:429-434)
6. Dora outputs `buffer_status` for backpressure control

**Verification:**
- [x] `AudioPlayer::buffer_fill_percentage()` returns real circular buffer fill (audio_player.rs:200)
- [x] Screen sends buffer status every 50ms via audio_timer (screen.rs:1089-1096)
- [x] DoraIntegration forwards to bridge when dataflow running (dora_integration.rs:318)
- [x] Bridge outputs to Dora: `buffer_status` (audio_player.rs:431)
- [x] NO estimation code in bridge (removed, now uses real values)

**Acceptance Criteria:**
- [x] `buffer_status` output reflects actual circular buffer fill (0-100%)
- [x] Bridge receives real values via `buffer_status_receiver` channel
- [x] Dispatcher check ensures status only sent when dataflow running

**Testing Verification:**
```bash
# Run dataflow and check logs
cargo run
# Should see: "Buffer status: XX.X%" in debug logs
# No estimation messages
```

---

### P0.2 - Session Start Deduplication ✅ DONE

**Problem:** `session_start` must be sent exactly ONCE per `question_id` on first audio chunk.

**Solution Implemented:**
```rust
// mofa-dora-bridge/src/widgets/audio_player.rs:222-242
let mut session_start_sent_for: HashSet<String> = HashSet::new();

if let Some(qid) = question_id {
    if !session_start_sent_for.contains(qid) {
        Self::send_session_start(node, input_id, &event_meta)?;
        session_start_sent_for.insert(qid.to_string());

        // Bound set size to last 100 question_ids
        if session_start_sent_for.len() > 100 {
            let to_remove: Vec<_> = session_start_sent_for.iter().take(50).cloned().collect();
            for key in to_remove {
                session_start_sent_for.remove(&key);
            }
        }
    }
}
```

**Verification:**
- [x] HashSet tracks sent question_ids
- [x] Set bounded to prevent memory growth
- [ ] Test 10+ conversation rounds without stopping
- [ ] Verify single `session_start` per question_id in controller logs

---

### P0.3 - Metadata Integer Extraction ✅ DONE

**Problem:** `question_id` is `Parameter::Integer`, but code only extracted `Parameter::String`.

**Solution Implemented:**
```rust
// mofa-dora-bridge/src/widgets/audio_player.rs:189-201
for (key, value) in metadata.parameters.iter() {
    let string_value = match value {
        Parameter::String(s) => s.clone(),
        Parameter::Integer(i) => i.to_string(),  // question_id is Integer!
        Parameter::Float(f) => f.to_string(),
        Parameter::Bool(b) => b.to_string(),
        Parameter::ListInt(l) => format!("{:?}", l),
        Parameter::ListFloat(l) => format!("{:?}", l),
        Parameter::ListString(l) => format!("{:?}", l),
    };
    event_meta.values.insert(key.clone(), string_value);
}
```

**Files Fixed:**
- [x] `mofa-dora-bridge/src/widgets/audio_player.rs`
- [x] `mofa-dora-bridge/src/widgets/participant_panel.rs`
- [x] `mofa-dora-bridge/src/widgets/prompt_input.rs`
- [x] `mofa-dora-bridge/src/widgets/system_log.rs`

---

### P0.4 - Channel Non-Blocking ✅ DONE

**Problem:** `send()` blocks when channel full, stalling the event loop.

**Solution Implemented:**
```rust
// mofa-dora-bridge/src/widgets/audio_player.rs:246-253
// Use try_send() to avoid blocking
if let Err(e) = audio_sender.try_send(audio_data.clone()) {
    warn!("Audio channel full, dropping audio chunk: {}", e);
}
let _ = event_sender.try_send(BridgeEvent::DataReceived { ... });
```

**Changes:**
- [x] Changed `send()` to `try_send()` for audio channel
- [x] Changed `send()` to `try_send()` for event channel
- [x] Increased audio channel buffer from 50 to 500 items

---

### P0.5 - Sample Count Tracking ✅ DONE

**Problem:** `data.0.len()` returns 1 for ListArray, not actual sample count.

**Solution Implemented:**
```rust
// mofa-dora-bridge/src/widgets/audio_player.rs:177-279
fn handle_dora_event(...) -> usize {  // Now returns sample count
    // ...
    if let Some(audio_data) = Self::extract_audio(&data, &event_meta) {
        let sample_count = audio_data.samples.len();
        // ... process audio ...
        return sample_count;  // Return actual samples extracted from ListArray
    }
    0  // Return 0 for non-audio events
}

// In event loop:
let sample_count = Self::handle_dora_event(...);
if sample_count > 0 {
    samples_in_buffer = (samples_in_buffer + sample_count).min(buffer_capacity);
}
```

**Verification:**
- [x] `handle_dora_event` returns `usize` sample count
- [x] Sample count extracted from `audio_data.samples.len()`
- [x] Build verified with `cargo check`

---

### P0.6 - Smart Reset (question_id Filtering)

**Problem:** After reset, may play stale audio from previous question.

**Current:** `reset()` clears entire buffer.

**Target:** Smart reset keeps only segments with active question_ids.

```rust
// apps/mofa-fm/src/audio_player.rs - ADD THIS
impl CircularAudioBuffer {
    /// Smart reset - only clear segments NOT in active question set
    pub fn smart_reset(&mut self, active_question_ids: &HashSet<String>) {
        // Keep segments that belong to active questions
        let mut new_segments = VecDeque::new();
        let mut samples_to_keep = 0;

        for segment in &self.segments {
            if let Some(ref qid) = segment.question_id {
                if active_question_ids.contains(qid) {
                    new_segments.push_back(segment.clone());
                    samples_to_keep += segment.samples_remaining;
                }
            }
        }

        self.segments = new_segments;
        self.available_samples = samples_to_keep;
        // Note: actual audio data stays in circular buffer, just tracking changes
    }
}
```

**Files to Modify:**
- [ ] `apps/mofa-fm/src/audio_player.rs` - Add `smart_reset()` method to CircularAudioBuffer
- [ ] `apps/mofa-fm/src/audio_player.rs` - Add `question_id: Option<String>` to AudioSegment struct
- [ ] `mofa-dora-bridge/src/widgets/audio_player.rs` - Pass question_id to AudioPlayer

**Acceptance Criteria:**
- [ ] Reset clears only stale segments (old question_ids)
- [ ] Active segments preserved during reset
- [ ] No stale audio playback after question change

---

### P0.7 - Streaming Timeout (Auto-Complete)

**Problem:** Incomplete LLM responses can hang UI indefinitely.

**Target:** Auto-complete streaming after 2s of silence.

```rust
// mofa-dora-bridge/src/widgets/audio_player.rs - ADD THIS
const STREAMING_TIMEOUT: Duration = Duration::from_secs(2);

struct StreamingState {
    participant: String,
    question_id: String,
    last_update: Instant,
}

// In event loop:
let mut streaming_states: HashMap<String, StreamingState> = HashMap::new();

// When receiving audio:
streaming_states.insert(participant.clone(), StreamingState {
    participant: participant.clone(),
    question_id: qid.clone(),
    last_update: Instant::now(),
});

// Check for timeouts periodically:
for (participant, state) in streaming_states.iter() {
    if state.last_update.elapsed() > STREAMING_TIMEOUT {
        info!("Streaming timeout for {} (qid={}), auto-completing", participant, state.question_id);
        Self::send_session_complete(node, participant, &state.question_id)?;
    }
}
streaming_states.retain(|_, state| state.last_update.elapsed() <= STREAMING_TIMEOUT);
```

**Files to Modify:**
- [ ] `mofa-dora-bridge/src/widgets/audio_player.rs` - Add streaming timeout tracking
- [ ] `mofa-dora-bridge/src/widgets/prompt_input.rs` - Add streaming timeout for text responses

**Acceptance Criteria:**
- [ ] Streaming auto-completes after 2s of no audio
- [ ] UI shows complete state, not stuck streaming
- [ ] Timeout configurable via environment variable (optional)

---

### P0.8 - Consolidate Participant Panel into Audio Player Bridge

**Problem:** mofa-fm has TWO separate bridges receiving the same audio:
- `mofa-audio-player` - handles playback, buffer_status, session_start, audio_complete
- `mofa-participant-panel` - handles LED level visualization

This causes:
1. **Duplicate audio processing** (same TTS audio sent to 2 nodes)
2. **Active speaker mismatch** - mofa-participant-panel uses `question_id` tracking, but should use `current_participant` from AudioPlayer (what's actually playing)
3. **More dataflow complexity** (extra dynamic node definition)

**Conference-dashboard approach:** Single `dashboard` node handles BOTH audio playback AND LED visualization.

**Current (mofa-fm):**
```yaml
# voice-chat.yml - TWO nodes receive same audio
mofa-audio-player:
  inputs:
    audio_student1: primespeech-student1/audio
    audio_student2: primespeech-student2/audio
    audio_tutor: primespeech-tutor/audio

mofa-participant-panel:  # DUPLICATE - remove this
  inputs:
    audio_student1: primespeech-student1/audio
    audio_student2: primespeech-student2/audio
    audio_tutor: primespeech-tutor/audio
```

**Target (like conference-dashboard):**
```yaml
# Only ONE node receives audio
mofa-audio-player:
  inputs:
    audio_student1: primespeech-student1/audio
    audio_student2: primespeech-student2/audio
    audio_tutor: primespeech-tutor/audio
  # Audio level/bands computed internally, sent to UI via events
```

**Implementation Plan:**

1. **Move audio level calculation into AudioPlayerBridge:**
```rust
// mofa-dora-bridge/src/widgets/audio_player.rs - ADD
fn calculate_audio_level(samples: &[f32]) -> f32 {
    // RMS with peak normalization (same as conference-dashboard)
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, |a, b| a.max(b));
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    let norm_factor = if peak > 0.01 { 1.0 / peak } else { 1.0 };
    (rms * norm_factor * 1.5).clamp(0.0, 1.0)
}

fn calculate_bands(samples: &[f32]) -> [f32; 8] {
    // 8-band visualization (same as ParticipantPanelBridge)
}
```

2. **Use AudioPlayer's current_participant for active speaker:**
```rust
// In screen.rs audio_timer handler - get active from AudioPlayer
if let Some(ref player) = self.audio_player {
    let active_participant = player.current_participant(); // What's ACTUALLY playing
    // Update participant panels based on this
}
```

3. **Send ParticipantAudioData from AudioPlayerBridge:**
```rust
// mofa-dora-bridge/src/widgets/audio_player.rs
// After processing audio, emit participant audio data
let audio_data = ParticipantAudioData {
    participant_id: participant.clone(),
    audio_level: Self::calculate_audio_level(&samples),
    bands: Self::calculate_bands(&samples),
    is_active: true, // Active because we just received audio
};
let _ = event_sender.send(BridgeEvent::ParticipantAudio(audio_data));
```

4. **Update dora_integration.rs to handle new event:**
```rust
// dora_integration.rs - handle ParticipantAudio from audio player bridge
BridgeEvent::ParticipantAudio(data) => {
    let _ = event_tx.send(DoraEvent::ParticipantAudioReceived { data });
}
```

5. **Remove mofa-participant-panel from dataflow:**
```yaml
# voice-chat.yml - DELETE this node
# - id: mofa-participant-panel
#   path: dynamic
#   inputs: ...
```

6. **Delete ParticipantPanelBridge (no longer needed):**
- Delete `mofa-dora-bridge/src/widgets/participant_panel.rs`
- Remove from `mofa-dora-bridge/src/widgets/mod.rs`
- Remove from dispatcher bridge creation

**Files to Modify:**
- [ ] `mofa-dora-bridge/src/widgets/audio_player.rs` - Add audio level/bands calculation, emit ParticipantAudio event
- [ ] `mofa-dora-bridge/src/bridge.rs` - Add `BridgeEvent::ParticipantAudio` variant
- [ ] `mofa-dora-bridge/src/widgets/mod.rs` - Remove participant_panel export
- [ ] `mofa-dora-bridge/src/dispatcher.rs` - Remove ParticipantPanelBridge creation
- [ ] `apps/mofa-fm/src/dora_integration.rs` - Handle ParticipantAudio event
- [ ] `apps/mofa-fm/src/screen.rs` - Use AudioPlayer.current_participant() for active speaker
- [ ] `apps/mofa-fm/dataflow/voice-chat.yml` - Remove mofa-participant-panel node
- [ ] Delete `mofa-dora-bridge/src/widgets/participant_panel.rs`

**Acceptance Criteria:**
- [ ] Only ONE dynamic node receives audio (mofa-audio-player)
- [ ] Active speaker based on `AudioPlayer.current_participant()` (actual playback)
- [ ] LED bars show audio levels correctly
- [ ] Band visualization works
- [ ] No duplicate audio processing
- [ ] Build passes without participant_panel bridge

---

### P0.9 - Conference Dashboard Chat Window Format

**Problem:** mofa-fm chat format differs from conference-dashboard.

**Current (mofa-fm):**
```
**Sender** ⌛: content
```
- No timestamp
- No message separators
- No filtering of "Context" messages
- Streaming indicator: ⌛

**Target (conference-dashboard):**
```
**Sender** (HH:MM:SS):
content

---

**Sender2** (HH:MM:SS):
content2
```
- Timestamp in parentheses
- `---` separator between messages
- Filters out "Context" sender
- Newline after sender line

**Implementation:**

```rust
// apps/mofa-fm/src/screen.rs - update_chat_display()

fn update_chat_display(&mut self, cx: &mut Cx) {
    // Filter out "Context" messages (like conference-dashboard)
    let filtered_messages: Vec<_> = self.chat_messages.iter()
        .filter(|msg| msg.sender != "Context")
        .collect();

    let chat_text = if filtered_messages.is_empty() {
        "Waiting for conversation...".to_string()
    } else {
        filtered_messages.iter()
            .map(|msg| {
                let streaming_indicator = if msg.is_streaming { " ⌛" } else { "" };
                // Format: **Sender** (timestamp) indicator:  \ncontent
                format!("**{}** ({}){}: \n{}",
                    msg.sender,
                    msg.timestamp,  // Need to add timestamp field
                    streaming_indicator,
                    msg.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")  // Add --- separator
    };

    self.view.markdown(ids!(...)).set_text(cx, &chat_text);
}
```

**Add timestamp to ChatMessageEntry:**
```rust
// apps/mofa-fm/src/screen.rs

struct ChatMessageEntry {
    sender: String,
    content: String,
    is_streaming: bool,
    timestamp: String,  // ADD THIS
}

impl ChatMessageEntry {
    fn new(sender: &str, content: String) -> Self {
        Self {
            sender: sender.to_string(),
            content,
            is_streaming: false,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        }
    }
}
```

**Files to Modify:**
- [ ] `apps/mofa-fm/src/screen.rs` - Add timestamp field to ChatMessageEntry
- [ ] `apps/mofa-fm/src/screen.rs` - Update `update_chat_display()` format
- [ ] `apps/mofa-fm/src/screen.rs` - Filter "Context" messages
- [ ] `apps/mofa-fm/Cargo.toml` - Add chrono dependency (if not present)

**Acceptance Criteria:**
- [ ] Chat shows timestamp in (HH:MM:SS) format
- [ ] Messages separated by `---`
- [ ] "Context" messages filtered out
- [ ] Streaming indicator still works (⌛)
- [ ] Format matches conference-dashboard

---

## P0 Summary

**Status:** 5/9 items complete

| Task | Status | Impact | Verification |
|------|--------|--------|--------------|
| P0.1 Buffer Status Measurement | ✅ COMPLETE | Accurate backpressure | ✅ Real values from AudioPlayer |
| P0.2 Session Start Deduplication | ✅ DONE | No duplicate signals | ✅ HashSet tracking implemented |
| P0.3 Metadata Integer Extraction | ✅ DONE | question_id works | ✅ All parameter types handled |
| P0.4 Channel Non-Blocking | ✅ DONE | No pipeline stalls | ✅ try_send() with buffer 500 |
| P0.5 Sample Count Tracking | ✅ DONE | Accurate buffer tracking | ✅ Returns actual sample count |
| P0.6 Smart Reset | ❌ MISSING | No stale audio | ❌ Not implemented |
| P0.7 Streaming Timeout | ❌ MISSING | No hung UI | ❌ Not implemented |
| P0.8 Consolidate Participant Panel | ❌ MISSING | No duplicate processing | ❌ Not implemented |
| P0.9 Chat Window Format | ❌ MISSING | Consistent UX | ❌ Not implemented |

**Blocking Issues Remaining:**
1. **P0.6**: Smart reset not implemented (stale audio after reset) - **CRITICAL**
2. **P0.7**: Streaming timeout not implemented (hung UI on incomplete LLM) - **HIGH**
3. **P0.8**: Duplicate audio processing (2 bridges receive same audio) - **HIGH**
4. **P0.9**: Chat format mismatch with conference-dashboard - **MEDIUM**

**Cross-Reference:** See [MOFA_FM_COMPARISON_ANALYSIS.md](./MOFA_FM_COMPARISON_ANALYSIS.md) for comparison with conference-dashboard which has P0.6 and P0.7 implemented.

---

## P1: High Priority (Do Second)

### P1.1 - Code Organization: Break Up Large Files

**Problem:** Monolithic files violate single responsibility principle.

| File | Current | Target |
|------|---------|--------|
| `apps/mofa-fm/src/screen.rs` | 2065 lines | < 500 lines |
| `mofa-studio-shell/src/app.rs` | 1120 lines | (Makepad constraint) |
| `mofa-dora-bridge/src/widgets/audio_player.rs` | ~600 lines | < 400 lines |

**screen.rs Extraction Plan:**

```
apps/mofa-fm/src/
├── screen/
│   ├── mod.rs              # Main screen composition (~200 lines)
│   ├── audio_controls.rs   # Audio panel, device selection (~300 lines)
│   ├── chat_panel.rs       # Chat display, prompt input (~400 lines)
│   ├── log_panel.rs        # Log display (~200 lines)
│   └── connection_status.rs # Connection state UI (~150 lines)
├── lib.rs
├── audio_player.rs
├── audio.rs
├── dora_integration.rs
└── mofa_hero.rs
```

**Files to Modify:**
- [ ] Create `apps/mofa-fm/src/screen/` directory
- [ ] Extract audio controls to `screen/audio_controls.rs`
- [ ] Extract chat panel to `screen/chat_panel.rs`
- [ ] Extract log panel to `screen/log_panel.rs`
- [ ] Extract connection status to `screen/connection_status.rs`
- [ ] Update `screen/mod.rs` to compose widgets
- [ ] Update imports in `lib.rs`
- [ ] Verify build and functionality

---

### P1.2 - Widget Duplication Removal

**Problem:** 988 duplicated lines (12% of codebase)

| Component | Location 1 | Location 2 | Lines |
|-----------|-----------|-----------|-------|
| ParticipantPanel | shell/widgets/ | mofa-widgets/ | 492 |
| LogPanel | shell/widgets/ | mofa-widgets/ | 134 |
| AudioPlayer | mofa-fm/ | conference-dashboard/ | 724 |

**Phase 1: Shell Widget Cleanup**
- [ ] Delete `mofa-studio-shell/src/widgets/participant_panel.rs`
- [ ] Delete `mofa-studio-shell/src/widgets/log_panel.rs`
- [ ] Update `mofa-studio-shell/src/widgets/mod.rs` to remove exports
- [ ] Update imports to use `mofa_widgets::` versions
- [ ] Verify build

**Phase 2: Audio Player Unification**
- [ ] Create `mofa-audio/` shared crate in workspace
- [ ] Move `apps/mofa-fm/src/audio_player.rs` to `mofa-audio/src/audio_player.rs`
- [ ] Add smart_reset from conference-dashboard
- [ ] Add streaming timeout from conference-dashboard
- [ ] Update `mofa-fm` and `conference-dashboard` to use shared crate

**Recommended Structure:**
```
mofa-audio/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── audio_player.rs      # Unified circular buffer
    ├── device_manager.rs    # Device enumeration
    ├── mic_monitor.rs       # Level monitoring
    └── smart_reset.rs       # question_id filtering
```

---

### P1.3 - Waveform Visualization

**Problem:** mofa-fm lacks real-time audio visualization that conference-dashboard has.

**Source:** `conference-dashboard/src/widgets/waveform_view.rs`

```rust
// 512-sample rolling buffer for visualization
struct WaveformView {
    samples: VecDeque<f32>,
    // Real-time visualization
}
```

**Files to Modify:**
- [ ] Port `waveform_view.rs` from conference-dashboard to `mofa-widgets/src/`
- [ ] Export from `mofa-widgets/src/lib.rs`
- [ ] Integrate into mofa-fm screen

---

### P1.4 - Font Definition Cleanup

**Problem:** Same fonts defined in multiple files.

**Audit Command:**
```bash
rg "FONT_REGULAR|FONT_BOLD|FONT_FAMILY" --type rust
```

**Files to Check:**
- [ ] `mofa-studio-shell/src/app.rs` - Remove local fonts, import from theme
- [ ] `mofa-studio-shell/src/widgets/sidebar.rs` - Remove local fonts
- [ ] `mofa-studio-shell/src/widgets/mofa_hero.rs` - Remove local fonts
- [ ] Keep only `mofa-widgets/src/theme.rs` as source of truth

---

## P1 Summary

| Task | Status | Impact |
|------|--------|--------|
| P1.1 Break Up Large Files | 📋 TODO | Maintainability |
| P1.2 Widget Duplication | 📋 TODO | -988 lines |
| P1.3 Waveform Visualization | 📋 TODO | UX improvement |
| P1.4 Font Cleanup | 📋 TODO | Single source of truth |

---

## P2: Medium Priority (Do Third)

### P2.1 - Shared State Pattern

**Problem:** UI and bridge thread tightly coupled via events.

**Target:** Shell coordinator pattern.

```rust
// Add to mofa-studio-shell/src/app.rs
pub struct DoraState {
    pub buffer_fill: f64,
    pub participants: Vec<ParticipantState>,
    pub chat_messages: Vec<ChatMessage>,
    pub connection_status: ConnectionStatus,
    pub pending_commands: Vec<DoraCommand>,
}

impl App {
    #[rust]
    dora_state: DoraState,

    fn notify_buffer_change(&mut self, cx: &mut Cx) {
        // Update UI widgets with new buffer state
        self.ui.buffer_gauge(ids!(...)).set_fill(cx, self.dora_state.buffer_fill);
    }
}
```

**Files to Modify:**
- [ ] Create `DoraState` struct in shell or shared location
- [ ] Add state field to `App` struct
- [ ] Add notification methods for state changes
- [ ] Update UI to read from state

---

### P2.2 - Debug Logging Cleanup

**Problem:** 15+ `println!` statements in production code.

**Files to Clean:**
- [ ] `apps/mofa-fm/src/screen.rs`
- [ ] `mofa-dora-bridge/src/widgets/*.rs`

**Solution:**
```rust
#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($arg:tt)*) => { };
}
```

---

### P2.3 - System Monitoring Integration

**Problem:** mofa-fm CPU/memory stats update may lag during heavy operations.

**Target:** Background thread updates like conference-dashboard.

```rust
// conference-dashboard pattern
fn start_system_monitor(shared_state: SharedStateRef) {
    thread::spawn(move || {
        let mut sys = System::new();
        loop {
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            if let Ok(mut state) = shared_state.lock() {
                state.cpu_usage = sys.global_cpu_usage();
                state.memory_usage = sys.used_memory() as f32 / sys.total_memory() as f32;
            }

            thread::sleep(Duration::from_secs(1));
        }
    });
}
```

**Files to Modify:**
- [ ] Add background system monitor thread to mofa-fm
- [ ] Update `mofa_hero.rs` to read from shared state

---

### P2.4 - Settings Persistence

**Problem:** Verify all settings persist correctly.

**Settings to Check:**
- [ ] Dark mode preference saves/loads
- [ ] Audio device preference saves/loads
- [ ] API keys save/load (already implemented)
- [ ] Last-used dataflow path (optional)

**Files:**
- [ ] `apps/mofa-settings/src/data/preferences.rs` - Verify persistence
- [ ] `apps/mofa-fm/src/screen.rs` - Load preferences on startup

---

## P2 Summary

| Task | Status | Impact |
|------|--------|--------|
| P2.1 Shared State Pattern | 📋 TODO | Cleaner architecture |
| P2.2 Debug Logging | 📋 TODO | Clean console |
| P2.3 System Monitoring | 📋 TODO | Responsive stats |
| P2.4 Settings Persistence | 📋 TODO | User preferences |

---

## P3: Low Priority (Do Later)

### P3.1 - CLI Interface

**Problem:** mofa-fm lacks CLI arguments.

**Target:**
```bash
mofa-studio --dataflow voice-chat.yml --node mofa-audio-player --sample-rate 32000
```

**Files to Modify:**
- [ ] Add clap/structopt to `apps/mofa-fm/Cargo.toml`
- [ ] Parse args in startup
- [ ] Pass to DoraIntegration

---

### P3.2 - Track mofa-dora-bridge in Git

**Problem:** `mofa-dora-bridge/` shows as untracked.

```bash
git add mofa-dora-bridge/
git commit -m "Track mofa-dora-bridge crate"
```

---

### P3.3 - Testing Infrastructure

**Target:** 70%+ coverage on testable components.

**Testable:**
- [ ] `CircularAudioBuffer` - fill percentage, smart reset
- [ ] `EventMetadata` - parameter extraction
- [ ] Session start deduplication logic

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_fill_percentage() {
        let mut buffer = CircularAudioBuffer::new(30.0, 32000);
        buffer.write_samples(&[0.5; 16000], None);
        assert!((buffer.fill_percentage() - 1.67).abs() < 0.1);
    }

    #[test]
    fn test_session_start_deduplication() {
        let mut sent = HashSet::new();
        assert!(should_send("100", &mut sent));
        assert!(!should_send("100", &mut sent)); // Duplicate
        assert!(should_send("200", &mut sent));  // New
    }

    #[test]
    fn test_smart_reset() {
        let mut buffer = CircularAudioBuffer::new(30.0, 32000);
        buffer.write_samples(&[0.5; 16000], Some("100".to_string()));
        buffer.write_samples(&[0.5; 16000], Some("200".to_string()));

        let active = HashSet::from(["200".to_string()]);
        buffer.smart_reset(&active);

        // Only question_id 200 segments remain
        assert_eq!(buffer.segments.len(), 1);
    }
}
```

---

### P3.4 - API Documentation

**Files to Document:**
- [ ] `mofa-dora-bridge/src/lib.rs` - Crate overview
- [ ] `mofa-dora-bridge/src/bridge.rs` - Bridge trait
- [ ] `mofa-dora-bridge/src/widgets/*.rs` - Each bridge widget
- [ ] Signal flow diagram in `MOFA_DORA_ARCHITECTURE.md`

---

## P3 Summary

| Task | Status | Impact |
|------|--------|--------|
| P3.1 CLI Interface | 📋 TODO | Flexibility |
| P3.2 Git Tracking | 📋 TODO | Version control |
| P3.3 Testing | 📋 TODO | Reliability |
| P3.4 Documentation | 📋 TODO | Maintainability |

---

## Success Criteria

### After P0
- [ ] Conversation runs 10+ rounds without stopping
- [ ] Buffer status reflects actual fill (measured, not estimated)
- [ ] No buffer overrun warnings in logs
- [ ] No duplicate `session_start` signals
- [ ] Smart reset clears only stale audio
- [ ] Streaming auto-completes after 2s timeout
- [ ] Only ONE bridge receives audio (no duplicate processing)
- [ ] Active speaker based on actual playback (AudioPlayer.current_participant)
- [ ] Chat format matches conference-dashboard (timestamps, separators, filtering)

### After P1
- [ ] No file > 500 lines (except app.rs - Makepad constraint)
- [ ] 0 duplicate widget files
- [ ] Waveform visualization working
- [ ] Single source of truth for fonts

### After P2
- [ ] Shared state pattern implemented
- [ ] 0 debug println statements
- [ ] System stats update in background
- [ ] All settings persist correctly

### After P3
- [ ] CLI arguments working
- [ ] mofa-dora-bridge tracked in git
- [ ] 70%+ test coverage on buffer/signal logic
- [ ] Complete API documentation

---

## Quick Reference: Key Files

### Dora Bridge Layer
| File | Purpose | Lines |
|------|---------|-------|
| `mofa-dora-bridge/src/widgets/audio_player.rs` | Audio bridge, signals | ~600 |
| `mofa-dora-bridge/src/widgets/participant_panel.rs` | LED bars, active speaker | ~300 |
| `mofa-dora-bridge/src/widgets/prompt_input.rs` | Chat, control commands | ~430 |
| `mofa-dora-bridge/src/widgets/system_log.rs` | Log aggregation | ~360 |

### Audio Layer
| File | Purpose | Lines |
|------|---------|-------|
| `apps/mofa-fm/src/audio_player.rs` | Circular buffer, CPAL | ~360 |
| `apps/mofa-fm/src/audio.rs` | Device enum, mic monitor | ~230 |

### UI Layer
| File | Purpose | Lines |
|------|---------|-------|
| `apps/mofa-fm/src/screen.rs` | Main screen | ~2065 |
| `apps/mofa-fm/src/mofa_hero.rs` | Status bar | ~740 |

### Configuration
| File | Purpose |
|------|---------|
| `apps/mofa-fm/dataflow/voice-chat.yml` | Dataflow definition |
| `MOFA_DORA_ARCHITECTURE.md` | Architecture diagram |

---

## Related Documents

| Document | Description |
|----------|-------------|
| [MOFA_DORA_ARCHITECTURE.md](./MOFA_DORA_ARCHITECTURE.md) | Signal flow diagrams |
| [CHECKLIST.md](./CHECKLIST.md) | UI refactoring checklist |
| [roadmap-claude.md](./roadmap-claude.md) | Architectural analysis |
| [roadmap-glm.md](./roadmap-glm.md) | Strategic planning with grades |
| [mofa-studio-roadmap.m2](./mofa-studio-roadmap.m2) | MoFA FM vs Conference Dashboard |

---

*Last Updated: 2026-01-05*
*P0 Progress: 5/9 complete (P0.1 verified complete, P0.6/P0.7/P0.8/P0.9 missing)*
*P1 Progress: 0/4 complete*
*P2 Progress: 0/4 complete*
*P3 Progress: 0/4 complete*

**P0.1 Buffer Status Measurement - VERIFIED COMPLETE ✅**
- Screen.rs sends real buffer status every 50ms (line 1089-1096)
- DoraIntegration routes to bridge (dora_integration.rs:315-327)
- Bridge outputs to Dora (audio_player.rs:429-434)
- No estimation code remaining

**Critical Implementation Gap:**
- **MoFA Studio** is missing:
  - **P0.6 Smart Reset** - stale audio after reset
  - **P0.7 Streaming Timeout** - hung UI on incomplete LLM
  - **P0.8 Consolidate Participant Panel** - duplicate audio processing
  - **P0.9 Chat Window Format** - format mismatch with conference-dashboard
- **Conference Dashboard** has P0.6, P0.7, P0.8, P0.9 implemented

**Port from conference-dashboard:**
- P0.6 Smart reset: `dora_bridge.rs` lines 100-102, 312-328
- P0.7 Streaming timeout: `dora_bridge.rs` lines 447-467
- P0.8 Consolidated audio: Single `dashboard` node approach
- P0.9 Chat format: `app.rs` lines 1865-1876

**Next Action:**
1. ✅ P0.1 Complete - No action needed
2. P0.8 Consolidate participant panel into audio player bridge
3. P0.9 Implement conference-dashboard chat format
4. P0.6 Port smart reset from conference-dashboard
5. P0.7 Port streaming timeout from conference-dashboard
