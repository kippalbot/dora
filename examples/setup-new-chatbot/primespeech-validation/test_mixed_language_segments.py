#!/usr/bin/env python3
"""
Mixed language segment test for PrimeSpeech TTS to investigate prompt leakage.
Tests short segments with mixed Chinese and English content.
"""

import time
import json
import pyarrow as pa
from dora import Node
from pathlib import Path

def create_mixed_language_segments():
    """Create test segments with mixed Chinese and English content"""
    
    segments = [
        {
            "id": 1,
            "text": "Hello world",
            "language": "en",
            "expected_chars": 11
        },
        {
            "id": 2, 
            "text": "你好世界",
            "language": "zh",
            "expected_chars": 4
        },
        {
            "id": 3,
            "text": "This is a test",
            "language": "en", 
            "expected_chars": 14
        },
        {
            "id": 4,
            "text": "这是一个测试",
            "language": "zh",
            "expected_chars": 6
        },
        {
            "id": 5,
            "text": "Welcome to Beijing",
            "language": "en",
            "expected_chars": 17
        },
        {
            "id": 6,
            "text": "欢迎来到北京",
            "language": "zh", 
            "expected_chars": 6
        },
        {
            "id": 7,
            "text": "AI technology",
            "language": "en",
            "expected_chars": 14
        },
        {
            "id": 8,
            "text": "人工智能技术",
            "language": "zh",
            "expected_chars": 6
        },
        {
            "id": 9,
            "text": "Thank you very much",
            "language": "en",
            "expected_chars": 20
        },
        {
            "id": 10,
            "text": "非常感谢",
            "language": "zh",
            "expected_chars": 4
        }
    ]
    
    return segments

def main():
    """Test mixed language segments for prompt leakage detection"""
    
    print("=" * 80)
    print("Mixed Language Segment Test - PrimeSpeech TTS")
    print("Investigating prompt leakage in short segments")
    print("=" * 80)
    
    node = Node()
    segments = create_mixed_language_segments()
    
    print(f"\nTest Configuration:")
    print(f"  Total segments: {len(segments)}")
    print(f"  Languages: English (en), Chinese (zh)")
    print(f"  Segment pattern: Alternating en/zh")
    
    total_chars = sum(seg["expected_chars"] for seg in segments)
    print(f"  Total characters: {total_chars}")
    
    # Wait for system initialization
    print("\nWaiting 5 seconds for all nodes to initialize...")
    time.sleep(5)
    
    # Send segments with timing
    results = []
    start_time = time.time()
    
    print(f"\nSending segments at {time.strftime('%H:%M:%S')}...")
    print("-" * 60)
    
    for i, segment in enumerate(segments):
        segment_start = time.time()
        
        print(f"Segment {segment['id']}: {segment['text']} ({segment['language']})")
        
        # Send segment with metadata
        node.send_output(
            "text_output",
            pa.array([segment["text"]]),
            metadata={
                "session_id": "mixed_language_test",
                "segment_id": segment["id"],
                "language": segment["language"],
                "expected_chars": segment["expected_chars"],
                "segment_index": i,
                "total_segments": len(segments),
                "segment_start_time": segment_start,
                "test_type": "prompt_leakage_detection"
            }
        )
        
        # Small delay between segments to observe processing
        time.sleep(0.5)
        
        segment_time = time.time() - segment_start
        results.append({
            "segment_id": segment["id"],
            "text": segment["text"],
            "language": segment["language"],
            "send_time": segment_time
        })
        
        print(f"  ✓ Sent in {segment_time:.3f}s")
    
    total_send_time = time.time() - start_time
    print(f"\nAll segments sent in {total_send_time:.3f}s")
    
    # Save test configuration
    test_config = {
        "test_type": "mixed_language_prompt_leakage",
        "timestamp": time.strftime('%Y-%m-%d %H:%M:%S'),
        "total_segments": len(segments),
        "total_characters": total_chars,
        "total_send_time": total_send_time,
        "segments": segments,
        "send_results": results
    }
    
    config_path = Path("tts_output/mixed_language_test_config.json")
    config_path.parent.mkdir(exist_ok=True)
    
    with open(config_path, 'w', encoding='utf-8') as f:
        json.dump(test_config, f, indent=2, ensure_ascii=False)
    
    print(f"\nTest configuration saved to: {config_path}")
    
    # Monitor for TTS output and potential prompt leakage
    print("\nMonitoring TTS output for prompt leakage...")
    print("Watch for:")
    print("  - Text from previous segments appearing in current audio")
    print("  - Mixed language contamination")
    print("  - Incomplete or truncated segments")
    print("  - Delayed audio output")
    
    # Wait for processing
    timeout = 180  # 3 minutes
    start = time.time()
    
    for event in node:
        if event["type"] == "STOP":
            break
        if time.time() - start > timeout:
            print("Timeout reached, exiting")
            break
    
    print("\n" + "=" * 80)
    print("Mixed language test completed")
    print("Check audio files in tts_output/ for prompt leakage evidence")
    print("=" * 80)

if __name__ == "__main__":
    main()