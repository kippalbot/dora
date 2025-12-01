# YAML Configuration Analysis: Issues & Updates Required

## 🚨 Current Issues Found

### **1. Missing Session End Signal Wiring**

**Problem:** The YAML configurations don't include the new `session_end` signal connections from TTS to controller.

**Current wiring:**
```yaml
# primespeech-tutor outputs
outputs:
  - audio
  - status
  - segment_complete
  - log
```

**Missing:**
```yaml
# Should include session_end for round completion
outputs:
  - audio
  - status
  - segment_complete
  - session_end  # ← MISSING!
  - log
```

### **2. Obsolete Buffer Status Connections**

**Problem:** Still includes buffer_status inputs that are no longer used after removal of audio buffer control.

**Current wiring:**
```yaml
# conference-controller inputs (OBSOLETE)
inputs:
  buffer_status: audio-player/buffer_status  # ← SHOULD BE REMOVED

# debate-monitor inputs (OBSOLETE)
inputs:
  buffer_status: audio-player/buffer_status  # ← SHOULD BE REMOVED
```

### **3. Obsolete Audio Buffer Environment Variables**

**Problem:** Controller still has buffer threshold environment variables that are no longer used.

**Current environment:**
```yaml
env:
  AUDIO_BUFFER_THRESHOLD: 30      # ← OBSOLETE
  AUDIO_BUFFER_RESUME_THRESHOLD: 10  # ← OBSOLETE
```

### **4. Missing Bridge Control Output**

**Problem:** Controller should send `bridge_control` output for round completion, but it's not in the YAML outputs.

**Missing:**
```yaml
outputs:
  - bridge_control  # ← MISSING for session end based round advancement
```

## 🔧 Required Updates

### **For All TTS Nodes (primespeech-tutor, primespeech-student1, etc.)**

Add session_end output:
```yaml
- id: primespeech-tutor
  inputs:
    text: tutor-text-segmenter/text_segment
  outputs:
    - audio
    - status
    - segment_complete
    - session_end        # ← ADD THIS
    - log
```

### **For Conference Controller**

Remove buffer status input and add session_end input and bridge_control output:
```yaml
- id: conference-controller
  inputs:
    student1:
      source: student1/text
      queue_size: 1000
    student2:
      source: student2/text
      queue_size: 1000
    tutor:
      source: tutor/text
      queue_size: 1000
    control: debate-monitor/control
    # REMOVE: buffer_status: audio-player/buffer_status
    # ADD: session_end: primespeech-tutor/session_end
    session_end: primespeech-tutor/session_end
  outputs:
    - control_judge
    - control_llm2
    - control_llm1
    - llm_control
    - judge_prompt
    - bridge_control     # ← ADD THIS
    - status
    - log
  env:
    DORA_POLICY_PATTERN: "[(tutor, *), (student2, 2), (student1, 1)]"
    # REMOVE: AUDIO_BUFFER_THRESHOLD: 30
    # REMOVE: AUDIO_BUFFER_RESUME_THRESHOLD: 10
```

### **For Debate Monitor**

Remove buffer_status input:
```yaml
- id: debate-monitor
  inputs:
    # ... existing inputs ...
    # REMOVE: buffer_status: audio-player/buffer_status
  outputs:
    # ... existing outputs ...
```

## 📋 Updated Configuration Files

### **1. dataflow-study-audio.yml**
- ✅ Remove buffer_status connections
- ✅ Add session_end wiring
- ✅ Add bridge_control output
- ✅ Remove audio buffer environment variables

### **2. dataflow-study-sequential.yml**
- ✅ Review for similar issues (no audio components, but check controller)

### **3. dataflow-study-audio-multi.yml**
- ✅ Apply same fixes as dataflow-study-audio.yml

### **4. dataflow-debate-sequential.yml**
- ✅ Review and apply controller updates

## 🎯 Benefits of Updates

### **After Updates:**
- ✅ **Proper session end signaling** for round completion
- ✅ **Clean bridge control** without obsolete buffer logic
- ✅ **Simplified environment configuration**
- ✅ **Correct controller wiring** for latest design
- ✅ **Eliminate obsolete connections** that confuse debugging

### **Signal Flow After Updates:**
```
LLM → Text Segmenter → TTS → Audio Player
                          ↓
                    session_end (NEW)
                          ↓
                 Controller (enhanced question_id)
                          ↓
                Round completion detection
                          ↓
                   bridge_control (NEW)
                          ↓
                    Next round activation
```

## 🔍 Files Requiring Updates

1. `dataflow-study-audio.yml` - **High Priority** (major changes needed)
2. `dataflow-study-audio-multi.yml` - **High Priority**
3. `dataflow-study-sequential.yml` - **Medium Priority** (controller updates)
4. `dataflow-debate-sequential.yml` - **Medium Priority**
5. Any other YAML files with conference-controller or TTS components

The YAML configurations need to be updated to reflect the **session end-based bridge control** architecture!