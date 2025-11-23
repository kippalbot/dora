# PrimeSpeech TTS Prompt Leakage Detection Test

This test suite is designed to identify and investigate prompt leakage issues in PrimeSpeech TTS when processing mixed language segments.

## Problem Description

Prompt leakage occurs when text from previous segments appears in the audio output of current segments, or when there's contamination between different language inputs. This is particularly problematic in mixed Chinese-English scenarios.

## Test Files

### Core Test Scripts
- `test_prompt_leakage_detection.py` - Comprehensive test with multiple scenarios
- `test_mixed_language_segments.py` - Simple mixed language test
- `run_leakage_test.sh` - Automated test runner

### Configuration
- `dataflow-leakage-test.yml` - Dataflow configured for leakage detection
- Enhanced logging and monitoring for debugging

## Test Scenarios

### 1. Short Alternating Segments
Very short segments alternating between English and Chinese:
```
Hi → 嗨 → OK → 好 → Bye → 拜
```

### 2. Similar Sounding Words
Words that sound similar in both languages:
```
hello → 哈喽 → coffee → 咖啡 → sofa → 沙发
```

### 3. Repeated Patterns
Repeated patterns to test state clearing:
```
test → 测试 → test → 测试 → test → 测试
```

### 4. Mixed Single Segments
Segments containing both languages:
```
Hello 世界 → 你好 world → AI 人工智能 → 科技 technology
```

### 5. Rapid Succession
Segments sent in quick succession:
```
quick → 快速 → fast → 迅速 → speed → 速度
```

## Running the Tests

### Quick Start
```bash
cd examples/setup-new-chatbot/primespeech-validation
./run_leakage_test.sh
```

### Manual Testing
```bash
# Start the dataflow manually
dora start dataflow-leakage-test.yml --detach

# Run specific test
python3 test_prompt_leakage_detection.py

# Stop dataflow
dora stop dataflow-leakage-test.yml
```

## Analyzing Results

### Audio Analysis
1. **Check each audio file** for content from previous segments
2. **Listen for language mixing** within single segments
3. **Verify segment boundaries** are clean
4. **Look for truncated or incomplete audio**

### Metadata Analysis
1. **Review `tts_output/prompt_leakage_test_results.json`** for test configuration
2. **Check `tts_output/leakage_monitor_log.json`** for runtime monitoring
3. **Analyze timing data** for processing delays

### Key Indicators of Leakage

#### 🚨 Critical Issues
- Previous segment text appearing in current audio
- Mixed languages where only single language expected
- Incomplete audio generation

#### ⚠️ Warning Signs
- Delayed audio output
- Overlapping segment processing
- Inconsistent segment timing

#### ✅ Normal Behavior
- Clean segment boundaries
- Correct language per segment
- Proper timing and ordering

## Expected Test Results

### Clean Output (No Leakage)
Each audio file should contain only the text for that specific segment:
- `segment_1.wav` → "Hi"
- `segment_2.wav` → "嗨"  
- `segment_3.wav` → "OK"
- etc.

### Leakage Evidence
- `segment_2.wav` contains "Hi 嗨" (previous segment leaked)
- `segment_3.wav` contains "嗨 OK" (contamination)
- Mixed languages in single-language segments

## Debugging Tips

### 1. Check TTS State Management
```python
# In PrimeSpeech TTS, ensure state is cleared between segments
def clear_state(self):
    self.internal_buffer = ""
    self.language_context = None
    self.previous_segment = None
```

### 2. Verify Text Segmentation
```python
# Ensure text segmenter properly isolates segments
def segment_text(self, text, language):
    # Clear previous context
    self.reset_context()
    # Process new segment
    return self.process_segment(text, language)
```

### 3. Monitor Audio Generation
```python
# Log audio generation details
def generate_audio(self, text, metadata):
    print(f"Generating audio for: '{text}' (lang: {metadata.get('language')})")
    print(f"Previous context: {self.get_context()}")
    # Generate audio...
    self.clear_context()
```

## Root Cause Investigation

### Common Causes
1. **State not cleared between segments**
2. **Buffer contamination in TTS engine**
3. **Language context persistence**
4. **Text segmenter not isolating inputs**
5. **Audio pipeline mixing segments**

### Investigation Steps
1. **Check segment metadata** - Verify proper language tagging
2. **Review TTS logs** - Look for state persistence
3. **Analyze timing** - Check for overlapping processing
4. **Test isolation** - Run single segments to verify baseline

## Fix Verification

After implementing fixes:
1. **Re-run the test suite**
2. **Verify clean audio output**
3. **Check no cross-contamination**
4. **Validate performance** (no significant slowdown)

## Files Generated

### Test Results
- `tts_output/prompt_leakage_test_results.json` - Complete test configuration and results
- `tts_output/leakage_monitor_log.json` - Runtime monitoring data
- `tts_output/mixed_language_test_config.json` - Simple test configuration

### Audio Files
- `tts_output/segment_*.wav` - Individual segment audio files
- Named with scenario and segment ID for easy analysis

## Troubleshooting

### No Audio Files Generated
1. Check PrimeSpeech TTS is running
2. Verify model loading
3. Check dataflow connections

### Test Doesn't Start
1. Ensure all dependencies are installed
2. Check Dora runtime is available
3. Verify working directory

### Inconsistent Results
1. Check system load (affects timing)
2. Verify model state consistency
3. Run test multiple times

## Contributing

When adding new test scenarios:
1. Follow the existing pattern in `create_leakage_test_scenarios()`
2. Add descriptive names and documentation
3. Update this README with new scenarios
4. Test with both clean and leaky configurations