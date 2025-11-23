#!/usr/bin/env python3
"""
Direct test for PrimeSpeech TTS prompt leakage using the existing test framework
"""

import sys
import time
import os
import json
import numpy as np
import soundfile as sf
from pathlib import Path

# Add PrimeSpeech to path
primespeech_path = Path(__file__).parent.parent.parent.parent / "node-hub" / "dora-primespeech"
sys.path.insert(0, str(primespeech_path))

def test_leakage_direct():
    """Test multiple short segments for prompt leakage"""
    
    print("=" * 80)
    print("PrimeSpeech TTS Direct Leakage Test")
    print("=" * 80)
    
    try:
        from dora_primespeech.moyoyo_tts_wrapper_streaming_fix import StreamingMoYoYoTTSWrapper
        
        # Test segments - mixed short Chinese and English
        test_segments = [
            {"text": "Hello", "lang": "en", "description": "English segment 1"},
            {"text": "你好", "lang": "zh", "description": "Chinese segment 1"},
            {"text": "world", "lang": "en", "description": "English segment 2"},
            {"text": "世界", "lang": "zh", "description": "Chinese segment 2"},
            {"text": "test", "lang": "en", "description": "English segment 3"},
            {"text": "测试", "lang": "zh", "description": "Chinese segment 3"},
            {"text": "AI", "lang": "en", "description": "English segment 4"},
            {"text": "人工智能", "lang": "zh", "description": "Chinese segment 4"}
        ]
        
        output_dir = Path("tts_output")
        output_dir.mkdir(exist_ok=True)
        
        # Initialize TTS once
        print("\nInitializing MoYoYo TTS...")
        init_start = time.time()
        
        wrapper = StreamingMoYoYoTTSWrapper(
            voice='doubao',
            device='cpu',
            enable_streaming=False  # Use batch mode
        )
        
        init_time = time.time() - init_start
        print(f"Initialization time: {init_time:.2f}s")
        
        results = []
        
        print(f"\nTesting {len(test_segments)} segments for prompt leakage...")
        print("-" * 60)
        
        for i, segment in enumerate(test_segments):
            print(f"\nSegment {i+1}: '{segment['text']}' ({segment['description']})")
            
            try:
                # Generate audio for this segment
                synthesis_start = time.time()
                
                # Use the wrapper to synthesize
                sample_rate, audio_data = wrapper.synthesize(
                    segment["text"], 
                    language=segment["lang"], 
                    speed=1.0
                )
                
                synthesis_time = time.time() - synthesis_start
                audio_duration = len(audio_data) / sample_rate
                
                # Save audio with specific naming
                output_file = output_dir / f"segment_{i+1:02d}_{segment['text']}.wav"
                sf.write(str(output_file), audio_data, sample_rate)
                
                file_size = output_file.stat().st_size
                
                result = {
                    "segment_id": i+1,
                    "text": segment["text"],
                    "language": segment["lang"],
                    "description": segment["description"],
                    "expected_content": segment["text"],  # Should only contain this
                    "audio_file": str(output_file),
                    "synthesis_time": synthesis_time,
                    "audio_duration": audio_duration,
                    "file_size": file_size,
                    "success": True
                }
                
                print(f"  ✓ Generated: {output_file.name}")
                print(f"    Duration: {audio_duration:.2f}s")
                print(f"    Size: {file_size} bytes")
                print(f"    Synthesis time: {synthesis_time:.3f}s")
                
                # Check for potential leakage indicators
                if audio_duration > 5.0:  # Single word shouldn't be this long
                    print(f"  ⚠️  Warning: Audio duration seems long for single word")
                
                results.append(result)
                
                # Small delay between segments to observe any state persistence
                time.sleep(2)
                
            except Exception as e:
                print(f"  ❌ Failed: {e}")
                result = {
                    "segment_id": i+1,
                    "text": segment["text"],
                    "language": segment["lang"],
                    "description": segment["description"],
                    "error": str(e),
                    "success": False
                }
                results.append(result)
        
        # Save comprehensive results
        results_file = output_dir / "leakage_test_results.json"
        
        test_summary = {
            "test_type": "prompt_leakage_direct_test",
            "timestamp": time.strftime('%Y-%m-%d %H:%M:%S'),
            "total_segments": len(test_segments),
            "successful_segments": sum(1 for r in results if r.get("success", False)),
            "segments_tested": test_segments,
            "results": results,
            "leakage_analysis": {
                "what_to_check": [
                    "Listen to segment_2 (你好) - should NOT contain 'Hello'",
                    "Listen to segment_3 (world) - should NOT contain 'Hello' or '你好'",
                    "Listen to segment_4 (世界) - should NOT contain 'world'",
                    "Each segment should only contain its specified text",
                    "Check for any audio artifacts or incomplete synthesis",
                    "Verify audio duration is appropriate for single words"
                ],
                "evidence_of_leakage": [
                    "Previous segment text audible in current segment",
                    "Mixed language contamination within single segment",
                    "Incomplete or truncated audio",
                    "Audio longer than expected for single word/phrase",
                    "Audio artifacts at beginning or end of segments"
                ],
                "expected_durations": {
                    "single_word_en": "0.5-2.0 seconds",
                    "single_word_zh": "0.5-2.0 seconds",
                    "warning_threshold": ">5.0 seconds"
                }
            }
        }
        
        with open(results_file, 'w', encoding='utf-8') as f:
            json.dump(test_summary, f, indent=2, ensure_ascii=False)
        
        # Print summary
        print(f"\n{'='*80}")
        print("LEAKAGE TEST SUMMARY")
        print("="*80)
        print(f"Total segments tested: {len(test_segments)}")
        print(f"Successful generations: {test_summary['successful_segments']}")
        print(f"Results saved: {results_file}")
        
        print(f"\nGenerated audio files:")
        for result in results:
            if result.get("success", False) and "audio_file" in result:
                duration = result.get("audio_duration", 0)
                size = result.get("file_size", 0)
                print(f"  {result['audio_file']} ({duration:.2f}s, {size} bytes)")
        
        print(f"\n🔍 LEAKAGE ANALYSIS INSTRUCTIONS:")
        print("="*50)
        for i, instruction in enumerate(test_summary["leakage_analysis"]["what_to_check"], 1):
            print(f"{i}. {instruction}")
        
        print(f"\n🚨 EVIDENCE OF LEAKAGE:")
        for i, evidence in enumerate(test_summary["leakage_analysis"]["evidence_of_leakage"], 1):
            print(f"{i}. {evidence}")
        
        print(f"\n⏱️  EXPECTED DURATIONS:")
        for key, value in test_summary["leakage_analysis"]["expected_durations"].items():
            print(f"  {key}: {value}")
        
        print("="*80)
        print("Test completed! Listen to the audio files to check for leakage.")
        print("="*80)
        
        return 0
        
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        return 1

if __name__ == "__main__":
    sys.exit(test_leakage_direct())