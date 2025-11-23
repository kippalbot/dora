#!/usr/bin/env python3
"""
Simple direct test for PrimeSpeech TTS prompt leakage
Tests multiple short segments in sequence
"""

import sys
import time
import os
from pathlib import Path

# Add PrimeSpeech to path
primespeech_path = Path(__file__).parent.parent.parent.parent / "node-hub" / "dora-primespeech"
sys.path.insert(0, str(primespeech_path))

def test_segment_leakage():
    """Test multiple segments for potential leakage"""
    
    print("=" * 80)
    print("PrimeSpeech TTS Direct Leakage Test")
    print("=" * 80)
    
    # Test segments - mixed short Chinese and English
    test_segments = [
        {"text": "Hello", "lang": "en", "expected": "Only 'Hello'"},
        {"text": "你好", "lang": "zh", "expected": "Only '你好'"},
        {"text": "world", "lang": "en", "expected": "Only 'world'"},
        {"text": "世界", "lang": "zh", "expected": "Only '世界'"},
        {"text": "test", "lang": "en", "expected": "Only 'test'"},
        {"text": "测试", "lang": "zh", "expected": "Only '测试'"}
    ]
    
    try:
        from dora_primespeech import PrimeSpeechTTS
        
        # Initialize TTS
        print("\nInitializing PrimeSpeech TTS...")
        tts = PrimeSpeechTTS()
        
        output_dir = Path("tts_output")
        output_dir.mkdir(exist_ok=True)
        
        results = []
        
        for i, segment in enumerate(test_segments):
            print(f"\n--- Segment {i+1}/{len(test_segments)} ---")
            print(f"Text: '{segment['text']}' ({segment['lang']})")
            print(f"Expected: {segment['expected']}")
            
            # Generate audio
            start_time = time.time()
            
            try:
                audio_data = tts.synthesize(segment["text"])
                synthesis_time = time.time() - start_time
                
                # Save audio
                output_file = output_dir / f"segment_{i+1:02d}_{segment['lang']}_{segment['text']}.wav"
                
                import soundfile as sf
                sf.write(str(output_file), audio_data, 16000)
                
                file_size = output_file.stat().st_size
                
                result = {
                    "segment_id": i+1,
                    "text": segment["text"],
                    "language": segment["lang"],
                    "expected": segment["expected"],
                    "output_file": str(output_file),
                    "synthesis_time": synthesis_time,
                    "file_size": file_size,
                    "success": True
                }
                
                print(f"✓ Generated: {output_file.name}")
                print(f"  Time: {synthesis_time:.3f}s")
                print(f"  Size: {file_size} bytes")
                
                results.append(result)
                
                # Brief pause between segments
                time.sleep(0.5)
                
            except Exception as e:
                print(f"❌ Failed: {e}")
                result = {
                    "segment_id": i+1,
                    "text": segment["text"],
                    "language": segment["lang"],
                    "error": str(e),
                    "success": False
                }
                results.append(result)
        
        # Save results
        import json
        results_file = output_dir / "leakage_test_results.json"
        
        test_summary = {
            "test_type": "direct_leakage_test",
            "timestamp": time.strftime('%Y-%m-%d %H:%M:%S'),
            "total_segments": len(test_segments),
            "successful_segments": sum(1 for r in results if r["success"]),
            "results": results,
            "analysis_instructions": [
                "Listen to each audio file individually",
                "Check for any text from previous segments",
                "Verify language isolation between segments",
                "Look for incomplete or truncated audio"
            ]
        }
        
        with open(results_file, 'w', encoding='utf-8') as f:
            json.dump(test_summary, f, indent=2, ensure_ascii=False)
        
        print(f"\n{'='*80}")
        print("Test Summary:")
        print(f"  Total segments: {len(test_segments)}")
        print(f"  Successful: {test_summary['successful_segments']}")
        print(f"  Failed: {len(test_segments) - test_summary['successful_segments']}")
        print(f"  Results saved: {results_file}")
        print("\nGenerated files:")
        for result in results:
            if result["success"]:
                print(f"  - {result['output_file']}")
        print("\nAnalysis Instructions:")
        print("  1. Listen to each audio file separately")
        print("  2. Check if segment_2 contains any of 'Hello'")
        print("  3. Check if segment_3 contains any of '你好' or 'Hello'")
        print("  4. Verify clean separation between all segments")
        print("="*80)
        
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(test_segment_leakage())