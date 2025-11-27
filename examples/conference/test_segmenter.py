#!/usr/bin/env python3
"""Test the multi-participant segmenter logic"""

import sys
sys.path.insert(0, '../../node-hub/dora-text-segmenter')

from dora_text_segmenter.multi_participant_segmenter import detect_session_event, segment_by_punctuation

# Test session detection
print("=== Testing Session Detection ===")
print(f"started: {detect_session_event({'session_status': 'started'})}")
print(f"ended: {detect_session_event({'session_status': 'ended'})}")
print(f"chunk: {detect_session_event({})}")

# Test segmentation
print("\n=== Testing Segmentation ===")
text = "你好，这是一个测试。这是第二句话！"
segments, incomplete, keep = segment_by_punctuation(text, min_length=5, max_length=15, punctuation="。！？.!?")
print(f"Input: {text}")
print(f"Segments: {segments}")
print(f"Incomplete: {incomplete}")
print(f"Keep: {keep}")

# Test with chunks
print("\n=== Testing Chunk Accumulation ===")
chunks = ["你好", "，这是", "一个测试", "。"]
buffer = ""
all_segments = []
for chunk in chunks:
    combined = buffer + chunk
    segs, incomplete, keep = segment_by_punctuation(combined, min_length=5, max_length=15, punctuation="。！？.!?")
    buffer = incomplete if keep else ""
    all_segments.extend(segs)
    print(f"Chunk: '{chunk}' -> Combined: '{combined}' -> Segments: {segs}, Buffer: '{buffer}'")

print(f"\nAll segments: {all_segments}")
