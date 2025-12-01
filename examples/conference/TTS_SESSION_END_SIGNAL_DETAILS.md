# TTS Session End Signal: When & What

## 🎯 When TTS Sends Session End Signals

TTS sends session end signals in **4 distinct scenarios**:

### 1. ✅ **Normal Session Completion**
**Trigger:** `session_status` in `["completed", "finished", "ended", "final"]`

```python
# After successful audio synthesis
session_status = metadata.get("session_status", "unknown")
if session_status in ["completed", "finished", "ended", "final"]:
    send_session_end_signal()
```

**When this happens:**
- LLM indicates this is the final message of their turn
- Text segmenter marks content as complete
- All audio fragments have been successfully synthesized and sent

### 2. 🚫 **Skipped Text (No Audio Needed)**
**Trigger:** Text contains only punctuation/whitespace OR `session_status` is "cancelled"

```python
# Text is only punctuation or empty
if not text_stripped or all(c in '。！？.!?,，、；：""''（）【】《》\n\r\t ' for c in text_stripped):
    # Send session_end if this is a terminating status
    if session_status in ["completed", "finished", "ended", "final", "cancelled"]:
        send_session_end_signal()
```

**When this happens:**
- Text contains only punctuation (like "。" or "！")
- Participant's turn ends without speaking content
- Session is cancelled before meaningful content

### 3. ❌ **Initialization Error**
**Trigger:** TTS engine fails to initialize

```python
try:
    # Initialize TTS models
    load_tts_models()
except Exception as init_err:
    # Send session_end with error status
    send_session_end_signal(session_status="error", error=init_err)
```

**When this happens:**
- Model files are missing or corrupted
- GPU/CPU resources unavailable
- Configuration errors prevent TTS startup

### 4. 💥 **Synthesis Error**
**Trigger:** Audio synthesis fails during processing

```python
try:
    # Synthesize speech
    audio_array = tts_engine.synthesize(text)
except Exception as e:
    # Send session_end with error status
    send_session_end_signal(session_status="error", error=e)
```

**When this happens:**
- Text contains unsupported characters
- Audio synthesis encounters processing errors
- Memory or resource limits exceeded

## 📋 Session End Signal Contents

### **Standard Metadata (Always Included)**
```python
metadata = {
    "question_id": metadata.get("question_id", "default"),  # Enhanced 16-bit question_id
    "session_status": session_status,                        # Completion status
    "session_id": session_id,                               # Unique session identifier
    "request_id": request_id,                               # Request tracking ID
}
```

### **Error-Specific Metadata (Error Cases Only)**
```python
# Added for initialization errors
metadata.update({
    "error": str(init_err),      # Error message
    "error_stage": "init"       # Where error occurred
})

# Added for synthesis errors
metadata.update({
    "error": str(e),             # Error message
    "error_stage": "synthesis"   # Where error occurred
})
```

## 🎯 Signal Output Details

### **Output Structure**
```python
node.send_output(
    "session_end",                    # Output channel name
    pa.array(["session_ended"]),       # Signal value (array with single string)
    metadata                          # Metadata dictionary (see above)
)
```

### **Signal Value**
- **Always**: `["session_ended"]` (Array with single string "session_ended")
- **Purpose**: Indicates this is a session end event (not data)

### **Example Session End Signals**

#### **1. Normal Completion**
```python
# Example: Round 2, Participant 3/3 (last participant)
metadata = {
    "question_id": 290,              # 0x0122 (R2P3/3[LAST])
    "session_status": "completed",
    "session_id": "session_12345",
    "request_id": "req_abcde"
}
```

#### **2. Skipped Text**
```python
# Example: Judge's turn with only punctuation
metadata = {
    "question_id": 0x0111,           # R2P2/2[LAST]
    "session_status": "cancelled",
    "session_id": "session_12346",
    "request_id": "req_bcdef"
}
```

#### **3. Initialization Error**
```python
# Example: TTS model loading failed
metadata = {
    "question_id": 0x0020,           # R1P1/3
    "session_status": "error",
    "session_id": "session_12347",
    "request_id": "req_cdefg",
    "error": "Failed to load model: model.pt not found",
    "error_stage": "init"
}
```

#### **4. Synthesis Error**
```python
# Example: Audio processing failed
metadata = {
    "question_id": 0x0021,           # R1P2/3
    "session_status": "error",
    "session_id": "session_12348",
    "request_id": "req_defgh",
    "error": "Unsupported character in text: '🦄'",
    "error_stage": "synthesis"
}
```

## 🔍 Session Status Values

| Status | Meaning | Bridge Action | Use Case |
|--------|---------|---------------|----------|
| `"completed"` | Normal session finished | ✅ Resume if last participant | Standard completion |
| `"finished"` | Session finished successfully | ✅ Resume if last participant | Alternative completion |
| `"ended"` | Session terminated | ✅ Resume if last participant | Another completion variant |
| `"final"` | Final message of session | ✅ Resume if last participant | Explicit final status |
| `"cancelled"` | Session cancelled | ❌ No resume (doesn't count) | User cancellation |
| `"error"` | Session failed | ✅ Resume if last participant | Error recovery |
| `"unknown"` | Status unclear | ❌ No action | Debugging |

## 📊 Session End Signal Flow

```
Text Segmenter → TTS Engine → [Process Text] → Check session_status
                                                    ↓
                                            Normal Completion?
                                                    ↓ YES
                                        Send Audio Fragments ──→ Send session_end
                                                    ↓ NO
                                            Error/Skipped?
                                                    ↓ YES
                                        Send session_end (with error)
                                                    ↓ NO
                                            Continue Processing
```

## 🎯 Key Points

1. **One Signal Per Session**: Unlike fragment completion, session end is sent **once per participant session**
2. **Status-Based Logic**: The `session_status` from upstream determines when to send session end
3. **Error Resilience**: Even errors generate session end signals to avoid hanging conversations
4. **Rich Metadata**: Enhanced question_id provides all context needed for bridge control
5. **Consistent Format**: All session end signals follow the same structure regardless of trigger

This ensures the controller receives **definitive session completion signals** with all necessary context for intelligent conversation flow management!