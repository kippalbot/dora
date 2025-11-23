#!/usr/bin/env python3
"""
Analyze the leakage test results and provide insights
"""

import json
import numpy as np
import soundfile as sf
from pathlib import Path

def analyze_audio_files():
    """Analyze generated audio files for leakage patterns"""
    
    print("=" * 80)
    print("PRIMESPEECH TTS LEAKAGE ANALYSIS")
    print("=" * 80)
    
    results_file = Path("tts_output/leakage_test_results.json")
    
    if not results_file.exists():
        print("❌ No test results found. Run the leakage test first.")
        return
    
    with open(results_file, 'r') as f:
        test_data = json.load(f)
    
    print(f"\nTest Overview:")
    print(f"  Test Type: {test_data['test_type']}")
    print(f"  Timestamp: {test_data['timestamp']}")
    print(f"  Total Segments: {test_data['total_segments']}")
    print(f"  Successful: {test_data['successful_segments']}")
    
    print(f"\nAudio File Analysis:")
    print("-" * 60)
    
    suspicious_patterns = []
    
    for result in test_data['results']:
        if not result.get('success', False):
            continue
            
        file_path = Path(result['audio_file'])
        if not file_path.exists():
            continue
            
        # Load audio data
        try:
            audio_data, sample_rate = sf.read(str(file_path))
            actual_duration = len(audio_data) / sample_rate
            
            print(f"\nSegment {result['segment_id']}: '{result['text']}'")
            print(f"  File: {file_path.name}")
            print(f"  Expected Duration: {result['audio_duration']:.2f}s")
            print(f"  Actual Duration: {actual_duration:.2f}s")
            print(f"  File Size: {result['file_size']} bytes")
            print(f"  Sample Rate: {sample_rate} Hz")
            
            # Check for suspicious patterns
            issues = []
            
            # Duration consistency check
            if abs(actual_duration - result['audio_duration']) > 0.1:
                issues.append(f"Duration mismatch: expected {result['audio_duration']:.2f}s, got {actual_duration:.2f}s")
            
            # Unusually long duration for single words
            if actual_duration > 3.0 and len(result['text']) <= 5:
                issues.append(f"Suspiciously long duration ({actual_duration:.2f}s) for short text '{result['text']}'")
            
            # File size anomalies
            expected_size_range = {
                1: (20000, 30000),   # Very short
                2: (25000, 50000),   # Short
                3: (30000, 60000),   # Medium
                4: (40000, 80000)    # Longer
            }
            
            text_len = len(result['text'])
            if text_len <= 2:
                min_size, max_size = expected_size_range[1]
            elif text_len <= 5:
                min_size, max_size = expected_size_range[2]
            else:
                min_size, max_size = expected_size_range[3]
                
            if not (min_size <= result['file_size'] <= max_size):
                issues.append(f"File size anomaly: {result['file_size']} bytes (expected {min_size}-{max_size})")
            
            # Audio quality checks
            if np.any(np.abs(audio_data) > 1.0):
                issues.append("Audio clipping detected")
            
            # Check for silence at beginning/end (potential leakage indicators)
            silence_threshold = 0.01
            start_silence_samples = int(0.1 * sample_rate)  # First 100ms
            end_silence_samples = int(0.1 * sample_rate)    # Last 100ms
            
            if len(audio_data) > start_silence_samples:
                start_rms = np.sqrt(np.mean(audio_data[:start_silence_samples]**2))
                if start_rms < silence_threshold:
                    issues.append(f"Silence at beginning (RMS: {start_rms:.4f})")
            
            if len(audio_data) > end_silence_samples:
                end_rms = np.sqrt(np.mean(audio_data[-end_silence_samples:]**2))
                if end_rms < silence_threshold:
                    issues.append(f"Silence at end (RMS: {end_rms:.4f})")
            
            if issues:
                print(f"  ⚠️  Issues found:")
                for issue in issues:
                    print(f"    - {issue}")
                suspicious_patterns.append({
                    'segment': result['segment_id'],
                    'text': result['text'],
                    'issues': issues
                })
            else:
                print(f"  ✅ No obvious issues detected")
                
        except Exception as e:
            print(f"  ❌ Error analyzing audio: {e}")
    
    # Summary analysis
    print(f"\n{'='*80}")
    print("LEAKAGE ANALYSIS SUMMARY")
    print("="*80)
    
    if suspicious_patterns:
        print(f"⚠️  {len(suspicious_patterns)} segments with suspicious patterns:")
        for pattern in suspicious_patterns:
            print(f"  Segment {pattern['segment']} ('{pattern['text']}'):")
            for issue in pattern['issues']:
                print(f"    - {issue}")
    else:
        print("✅ No obvious technical anomalies detected in audio files")
    
    print(f"\n🔍 MANUAL LISTENING TEST REQUIRED:")
    print("="*50)
    print("Technical analysis alone cannot detect prompt leakage.")
    print("Please manually listen to the files in this order:")
    print()
    
    for i, result in enumerate(test_data['results']):
        if result.get('success', False):
            expected = result['expected_content']
            file_name = Path(result['audio_file']).name
            print(f"{i+1}. {file_name}")
            print(f"   Expected content: ONLY '{expected}'")
            if i > 0:
                prev_text = test_data['results'][i-1]['text']
                print(f"   Should NOT contain: '{prev_text}'")
            print()
    
    print("🚨 LEAKAGE INDICATORS TO LISTEN FOR:")
    print("- Previous segment text audible in current segment")
    print("- Mixed language contamination (e.g., English in Chinese segment)")
    print("- Incomplete words or truncated audio")
    print("- Extra audio before or after the expected content")
    print("- Repeated phrases or echoes")
    
    print(f"\n{'='*80}")

if __name__ == "__main__":
    analyze_audio_files()