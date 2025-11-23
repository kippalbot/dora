#!/usr/bin/env python3
"""
Prompt leakage detection test for PrimeSpeech TTS.
Tests specific scenarios that might cause prompt leakage between segments.
"""

import time
import json
import pyarrow as pa
from dora import Node
from pathlib import Path

def create_leakage_test_scenarios():
    """Create test scenarios designed to trigger prompt leakage"""
    
    scenarios = [
        # Scenario 1: Very short alternating segments
        {
            "name": "short_alternating",
            "description": "Very short alternating segments",
            "segments": [
                {"id": 1, "text": "Hi", "language": "en"},
                {"id": 2, "text": "嗨", "language": "zh"},
                {"id": 3, "text": "OK", "language": "en"},
                {"id": 4, "text": "好", "language": "zh"},
                {"id": 5, "text": "Bye", "language": "en"},
                {"id": 6, "text": "拜", "language": "zh"}
            ]
        },
        
        # Scenario 2: Similar sounding words
        {
            "name": "similar_sounds",
            "description": "Words with similar sounds in both languages",
            "segments": [
                {"id": 1, "text": "hello", "language": "en"},
                {"id": 2, "text": "哈喽", "language": "zh"},
                {"id": 3, "text": "coffee", "language": "en"},
                {"id": 4, "text": "咖啡", "language": "zh"},
                {"id": 5, "text": "sofa", "language": "en"},
                {"id": 6, "text": "沙发", "language": "zh"}
            ]
        },
        
        # Scenario 3: Repeated patterns
        {
            "name": "repeated_patterns",
            "description": "Repeated patterns to test state clearing",
            "segments": [
                {"id": 1, "text": "test", "language": "en"},
                {"id": 2, "text": "测试", "language": "zh"},
                {"id": 3, "text": "test", "language": "en"},
                {"id": 4, "text": "测试", "language": "zh"},
                {"id": 5, "text": "test", "language": "en"},
                {"id": 6, "text": "测试", "language": "zh"}
            ]
        },
        
        # Scenario 4: Mixed within single segment
        {
            "name": "mixed_single_segment",
            "description": "Mixed Chinese and English in single segments",
            "segments": [
                {"id": 1, "text": "Hello 世界", "language": "mixed"},
                {"id": 2, "text": "你好 world", "language": "mixed"},
                {"id": 3, "text": "AI 人工智能", "language": "mixed"},
                {"id": 4, "text": "科技 technology", "language": "mixed"}
            ]
        },
        
        # Scenario 5: Rapid succession
        {
            "name": "rapid_succession",
            "description": "Segments sent in rapid succession",
            "segments": [
                {"id": 1, "text": "quick", "language": "en"},
                {"id": 2, "text": "快速", "language": "zh"},
                {"id": 3, "text": "fast", "language": "en"},
                {"id": 4, "text": "迅速", "language": "zh"},
                {"id": 5, "text": "speed", "language": "en"},
                {"id": 6, "text": "速度", "language": "zh"}
            ]
        }
    ]
    
    return scenarios

def run_scenario(node, scenario, scenario_index):
    """Run a single test scenario"""
    
    print(f"\n{'='*60}")
    print(f"Scenario {scenario_index + 1}: {scenario['name']}")
    print(f"Description: {scenario['description']}")
    print(f"{'='*60}")
    
    segments = scenario['segments']
    results = []
    
    scenario_start = time.time()
    
    for i, segment in enumerate(segments):
        segment_start = time.time()
        
        print(f"Segment {segment['id']}: '{segment['text']}' ({segment['language']})")
        
        # Send segment with detailed metadata
        node.send_output(
            "text_output",
            pa.array([segment["text"]]),
            metadata={
                "session_id": f"leakage_test_{scenario['name']}",
                "scenario": scenario['name'],
                "scenario_index": scenario_index,
                "segment_id": segment["id"],
                "text": segment["text"],
                "language": segment["language"],
                "segment_index": i,
                "total_segments": len(segments),
                "segment_start_time": segment_start,
                "test_type": "prompt_leakage_detection",
                "expected_leakage": True  # Flag for analysis
            }
        )
        
        # Variable delay based on scenario
        if scenario['name'] == 'rapid_succession':
            time.sleep(0.1)  # Very fast
        elif scenario['name'] == 'short_alternating':
            time.sleep(0.3)  # Fast
        else:
            time.sleep(0.5)  # Normal
        
        segment_time = time.time() - segment_start
        results.append({
            "segment_id": segment["id"],
            "text": segment["text"],
            "language": segment["language"],
            "send_time": segment_time
        })
        
        print(f"  ✓ Sent in {segment_time:.3f}s")
    
    scenario_time = time.time() - scenario_start
    print(f"\nScenario completed in {scenario_time:.3f}s")
    
    return results

def main():
    """Run prompt leakage detection tests"""
    
    print("=" * 80)
    print("Prompt Leakage Detection Test - PrimeSpeech TTS")
    print("Testing scenarios that might cause prompt leakage")
    print("=" * 80)
    
    node = Node()
    scenarios = create_leakage_test_scenarios()
    
    print(f"\nTest Configuration:")
    print(f"  Total scenarios: {len(scenarios)}")
    print(f"  Total segments: {sum(len(s['segments']) for s in scenarios)}")
    
    scenario_names = [s['name'] for s in scenarios]
    print(f"  Scenarios: {', '.join(scenario_names)}")
    
    # Wait for system initialization
    print("\nWaiting 5 seconds for all nodes to initialize...")
    time.sleep(5)
    
    # Run all scenarios
    all_results = {}
    test_start = time.time()
    
    for i, scenario in enumerate(scenarios):
        results = run_scenario(node, scenario, i)
        all_results[scenario['name']] = {
            "scenario": scenario,
            "results": results,
            "scenario_index": i
        }
        
        # Brief pause between scenarios
        if i < len(scenarios) - 1:
            print("\nPausing 3 seconds before next scenario...")
            time.sleep(3)
    
    total_test_time = time.time() - test_start
    
    # Save comprehensive test results
    test_results = {
        "test_type": "prompt_leakage_detection_comprehensive",
        "timestamp": time.strftime('%Y-%m-%d %H:%M:%S'),
        "total_test_time": total_test_time,
        "total_scenarios": len(scenarios),
        "total_segments": sum(len(s['segments']) for s in scenarios),
        "scenarios_tested": scenario_names,
        "detailed_results": all_results,
        "analysis_instructions": {
            "check_for": [
                "Previous segment text appearing in current audio",
                "Language mixing between segments", 
                "Incomplete audio generation",
                "Delayed or overlapping audio output",
                "State persistence issues"
            ],
            "audio_files_to_check": "tts_output/*.wav",
            "metadata_to_review": "Segment metadata in audio files"
        }
    }
    
    results_path = Path("tts_output/prompt_leakage_test_results.json")
    results_path.parent.mkdir(exist_ok=True)
    
    with open(results_path, 'w', encoding='utf-8') as f:
        json.dump(test_results, f, indent=2, ensure_ascii=False)
    
    print(f"\n{'='*80}")
    print("All scenarios completed!")
    print(f"Total test time: {total_test_time:.3f}s")
    print(f"Results saved to: {results_path}")
    print("\nNext steps:")
    print("1. Check audio files in tts_output/ for leakage evidence")
    print("2. Review segment timing and ordering")
    print("3. Analyze metadata in generated audio files")
    print("4. Compare expected vs actual audio content")
    print("="*80)
    
    # Monitor for final processing
    print("\nMonitoring final TTS processing...")
    timeout = 120  # 2 minutes
    start = time.time()
    
    for event in node:
        if event["type"] == "STOP":
            break
        if time.time() - start > timeout:
            print("Timeout reached, exiting")
            break

if __name__ == "__main__":
    main()