# Chinese Voice Verification Test

Comprehensive test for all Chinese male and female voices in Kokoro TTS.

## Quick Start

```bash
cd examples/setup-new-chatbot/kokoro-tts-validation

# Test all Chinese voices (recommended)
python test_chinese_voices.py

# Test only male voices
python test_chinese_voices.py --type male

# Test only female voices
python test_chinese_voices.py --type female

# Test with custom text
python test_chinese_voices.py --text "你好世界，这是测试文本。"
```

## What It Tests

### Male Voices (zm_*)
- **zm_yunjian** - Chinese Male - Yunjian
- **zm_yunxi** - Chinese Male - Yunxi
- **zm_yunxia** - Chinese Male - Yunxia
- **zm_yunyang** - Chinese Male - Yunyang

### Female Voices (zf_*)
- **zf_xiaobei** - Chinese Female - Xiaobei
- **zf_xiaoni** - Chinese Female - Xiaoni
- **zf_xiaoxiao** - Chinese Female - Xiaoxiao
- **zf_xiaoyi** - Chinese Female - Xiaoyi

## Test Output

The test will:

1. **Synthesize audio** for each voice with the same Chinese text
2. **Measure performance** (RTF, synthesis time, processing speed)
3. **Save audio files** to `tts_output/chinese_voices/` for manual comparison
4. **Generate metrics** in JSON format for analysis
5. **Print comparison table** showing performance of all voices

### Example Output

```
================================================================================
CHINESE VOICE VERIFICATION TEST - KOKORO TTS
================================================================================

Test text: 168 characters
------------------------------------------------------------
我们说中国式现代化是百年大战略，这又分为三个阶段。第一个阶段，我们先用30年时间建成了独立完整的工业体系...
------------------------------------------------------------

================================================================================
TESTING MALE VOICES
================================================================================

Testing: Chinese Male - Yunxi (zm_yunxi)
------------------------------------------------------------
  Audio duration: 12.45s
  Synthesis time: 0.823s
  RTF: 15.12x
  Speed: 204.1 chars/sec
  File saved: zm_yunxi_test.wav (294.5 KB)
  ✓ Success

[... other voices ...]

================================================================================
SUMMARY
================================================================================

Total voices tested: 8
Successful: 8
Failed: 0

Average RTF: 14.32x
Average synthesis time: 0.867s
Average processing speed: 193.7 chars/sec

Fastest voice: zm_yunxi
Slowest voice: zf_xiaoyi

================================================================================
VOICE COMPARISON
================================================================================
Voice ID        Type     RTF      Time (s)   Speed        Status
--------------------------------------------------------------------------------
zm_yunjian      Male     14.23    0.875      192.0        ✓ Success
zm_yunxi        Male     15.12    0.823      204.1        ✓ Success
zm_yunxia       Male     13.98    0.890      188.8        ✓ Success
zm_yunyang      Male     14.56    0.855      196.5        ✓ Success
zf_xiaobei      Female   14.11    0.882      190.5        ✓ Success
zf_xiaoni       Female   14.45    0.861      195.1        ✓ Success
zf_xiaoxiao     Female   14.78    0.842      199.5        ✓ Success
zf_xiaoyi       Female   13.45    0.925      181.6        ✓ Success
================================================================================

================================================================================
GENERATED AUDIO FILES
================================================================================

All audio files saved to: tts_output/chinese_voices/

Male voices:
  - zm_yunjian_test.wav
  - zm_yunxi_test.wav
  - zm_yunxia_test.wav
  - zm_yunyang_test.wav

Female voices:
  - zf_xiaobei_test.wav
  - zf_xiaoni_test.wav
  - zf_xiaoxiao_test.wav
  - zf_xiaoyi_test.wav

Listen to these files to compare voice quality and characteristics.
================================================================================

✓ Results saved to: tts_output/chinese_voices_results.json
✓ Test completed successfully!
```

## Output Files

### Audio Files
All test audio files are saved to `tts_output/chinese_voices/`:
- `zm_*.wav` - Male voice samples
- `zf_*.wav` - Female voice samples

### Metrics JSON
Performance metrics saved to `tts_output/chinese_voices_results.json`:
```json
{
  "timestamp": "2024-12-04 16:30:00",
  "text_length": 168,
  "male_voices": {
    "zm_yunxi": {
      "status": "success",
      "rtf": 15.12,
      "synthesis_time": 0.823,
      "audio_duration": 12.45,
      "chars_per_second": 204.1,
      "file_path": "tts_output/chinese_voices/zm_yunxi_test.wav"
    }
  },
  "summary": {
    "total_tested": 8,
    "successful": 8,
    "avg_rtf": 14.32,
    "fastest_voice": "zm_yunxi"
  }
}
```

## Use Cases

### 1. Voice Selection
Listen to all generated audio files to choose the best voice for your application:
```bash
# macOS
open tts_output/chinese_voices/zm_yunxi_test.wav

# Linux
xdg-open tts_output/chinese_voices/zm_yunxi_test.wav
```

### 2. Performance Comparison
Compare synthesis speed across all voices:
```bash
python test_chinese_voices.py --type all
# Check the comparison table and JSON output
```

### 3. Quality Testing
Test with your specific content:
```bash
python test_chinese_voices.py --text "$(cat your_chinese_text.txt)"
```

### 4. Integration Testing
Verify voices work before integrating into your Dora dataflow:
```bash
# Test the voice you plan to use
python test_chinese_voices.py --type male
# Listen to zm_yunxi_test.wav
# Update your dataflow with VOICE: zm_yunxi
```

## Performance Expectations

Based on typical results:

| Metric | Expected Value |
|--------|----------------|
| **RTF** | 12-18x (all voices) |
| **Synthesis Time** | 0.8-1.0s for ~170 chars |
| **Processing Speed** | 180-210 chars/sec |
| **Audio Quality** | Good (24kHz sample rate) |
| **File Size** | ~290-300 KB for 12s audio |

## Troubleshooting

### "No module named 'kokoro'"
```bash
pip install kokoro>=0.2.2
pip install "misaki[zh]"
```

### "Voice not found" error
The language code must be `'z'` for Chinese voices (not `'zh'`):
```python
pipeline = KPipeline(lang_code='z')  # Correct
```

### Poor audio quality
- Ensure text is in Chinese (not English)
- Try different voices - quality varies slightly
- Check the text encoding (must be UTF-8)

## Language Code Reference

Kokoro uses single-letter codes for languages:
- **English**: `'a'`
- **Chinese**: `'z'` (not 'zh'!)
- **Japanese**: `'j'`
- **Korean**: `'k'`

Chinese voice prefixes:
- **Male**: `zm_` (uses lang_code='z')
- **Female**: `zf_` (uses lang_code='z')

## Integration Example

After finding your preferred voice, use it in your Dora dataflow:

```yaml
- id: kokoro-tts
  path: dora-kokoro-tts
  inputs:
    text: llm/response
  outputs:
    - audio
  env:
    BACKEND: mlx  # or cpu
    VOICE: zm_yunxi  # Your chosen voice
    LANGUAGE: z  # Important: 'z' not 'zh'
    SPEED_FACTOR: "1.0"
```

## See Also

- [Main Kokoro TTS Validation](README.md)
- [Backend Comparison Tests](README_BACKEND_TESTS.md)
- [Kokoro TTS Documentation](https://github.com/remsky/Kokoro-TTS)
- [Available Voices](https://huggingface.co/hexgrad/Kokoro-82M/blob/main/VOICES.md)
