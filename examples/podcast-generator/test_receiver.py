#!/usr/bin/env python3
"""
Simple audio receiver for testing MiniMax T2A
"""
import json
from dora import Node

def main():
    node = Node("audio-receiver")

    print("Audio receiver started, waiting for events...")

    audio_count = 0
    segment_complete = False

    for event in node:
        if event["type"] == "INPUT":
            event_id = event["id"]

            if event_id == "audio":
                audio_count += 1
                metadata = event.get("metadata", {})
                print(f"✓ Received audio #{audio_count}: metadata={metadata}")

            elif event_id == "segment_complete":
                status = event["value"][0].as_py()
                print(f"✓ Segment complete: status={status}")
                segment_complete = True

            elif event_id == "log":
                log_data = json.loads(event["value"][0].as_py())
                level = log_data.get("level", "INFO")
                message = log_data.get("message", "")
                print(f"[{level}] {message}")

        elif event["type"] == "STOP":
            break

    print(f"\nAudio receiver stopped")
    print(f"Total audio chunks received: {audio_count}")
    print(f"Segment complete: {segment_complete}")

if __name__ == "__main__":
    main()
