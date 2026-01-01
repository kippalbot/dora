# Conference Dashboard Improvement Plan

## Code Review Summary

### Critical Issues

1. **Broken Dora Bridge Data Parsing** (`dora_bridge.rs:110-119`)
   - `extract_f64()` and `extract_string()` always return `None`
   - Arrow data is never actually parsed - bridge can't receive real dataflow events

2. **Thread-Local Storage Bug** (`app.rs:8-20`)
   - `set_shared_state()` is called from main thread
   - Makepad runs UI in its own thread - `get_shared_state()` returns `None`
   - UI updates silently fail because state is unreachable

3. **Widgets Don't Self-Update**
   - All widgets (`ParticipantPanel`, `BufferGauge`, `LogPanel`) have minimal implementations
   - Update logic is centralized in `app.rs::update_from_shared_state()` but doesn't work due to thread bug
   - Log panel UI exists but never renders actual log messages

### Architecture Issues

4. **Demo Mode Blocks Forever** (`dora_bridge.rs:156-204`)
   - Runs in infinite loop with no shutdown mechanism
   - No way to stop the demo thread gracefully
   - Thread never joins on application exit

5. **Missing Real Waveform Visualization** (`waveform_view.rs`)
   - Uses hardcoded sine wave animation
   - Ignores `SharedState.waveform_data` which should contain real audio samples

6. **No Error Handling**
   - Missing error types
   - Demo thread panics on errors instead of logging
   - No recovery from bridge failures

### Code Quality Issues

7. **Magic Numbers**
   - Buffer duration hardcoded as 360.0 seconds (`dora_bridge.rs:44`)
   - Log max size (100) scattered in code
   - No constants for configuration values

8. **Missing Tests**
   - No unit tests
   - No integration tests
   - No demo mode validation

---

## Implementation Plan

### Phase 1: Core Fixes

#### 1.1 Fix Thread-Local Storage
```
File: app.rs
- Replace thread_local! with Arc<Mutex<...>> passed through Makepad's Scope
- Or use Cx::user_data() to store shared state in Makepad context
```

#### 1.2 Implement Arrow Data Parsing
```
File: dora_bridge.rs
- Add proper Arrow array parsing using dora_node_api methods
- Extract f64 from FloatArray, string from StringArray
- Test with actual Dora dataflow events
```

#### 1.3 Add Shutdown Mechanism
```
File: dora_bridge.rs
- Add AtomicBool flag for shutdown signal
- Check flag in demo loop and dora loop
- Handle Event::Stop properly
```

### Phase 2: Widget Improvements

#### 2.1 Real Waveform Visualization
```
File: waveform_view.rs
- Add waveform_data property to WaveformView struct
- Bind to SharedState.waveform_data
- Render actual audio samples instead of sine wave
```

#### 2.2 Functional Log Panel
```
File: log_panel.rs
- Add log_entries: Vec<LogMessage> to LogPanel struct
- Update from shared state in handle_event
- Implement proper log rendering with scroll
```

#### 2.3 Speaking Indicator Animation
```
File: participant_panel.rs
- Animate the speaking indicator (pulsing effect)
- Bind to participant.is_speaking state
```

### Phase 3: Architecture Improvements

#### 3.1 Add Constants and Config
```
File: lib.rs
- Add const MAX_LOG_MESSAGES = 100
- Add const DEFAULT_BUFFER_SECONDS = 360.0
- Add const DEMO_UPDATE_INTERVAL_MS = 100
```

#### 3.2 Error Handling
```
File: lib.rs
- Define AppError enum
- Use Result types throughout
- Add proper error logging
```

#### 3.3 Thread-Safe Demo Mode
```
File: dora_bridge.rs
- Use Arc<AtomicBool> for shutdown flag
- Spawn demo thread returning JoinHandle
- Join thread on application exit
```

### Phase 4: Testing

#### 4.1 Unit Tests
```
- Test extract_participant_index()
- Test get_timestamp()
- Test state serialization
```

#### 4.2 Integration Tests
```
- Test demo mode data generation
- Test shared state thread safety
- Test widget updates
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/lib.rs` | Add constants, error types |
| `src/app.rs` | Fix thread storage, improve update logic |
| `src/dora_bridge.rs` | Fix Arrow parsing, add shutdown, constants |
| `src/widgets/waveform_view.rs` | Real audio data rendering |
| `src/widgets/log_panel.rs` | Implement log display |
| `src/widgets/participant_panel.rs` | Add speaking animation |
| `Cargo.toml` | Add test dependencies |

---

## Backward Compatibility

All changes are internal improvements. The public API remains:
- Same CLI invocation: `cargo run`
- Same demo mode fallback
- Same Makepad UI layout
