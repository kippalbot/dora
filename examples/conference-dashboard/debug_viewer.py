#!/usr/bin/env python3
"""Debug viewer to verify dataflow wiring."""

from dora import Node
import json
from datetime import datetime

def main():
    node = Node()
    print("=" * 60)
    print("DEBUG VIEWER STARTED - Monitoring dataflow")
    print("=" * 60)

    for event in node:
        if event["type"] == "INPUT":
            input_id = event["id"]
            ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]

            try:
                value = event["value"]

                # Handle different input types
                if "audio" in input_id:
                    # Audio data - just show length
                    if hasattr(value, '__len__'):
                        print(f"[{ts}] {input_id}: AUDIO {len(value)} samples")
                    else:
                        print(f"[{ts}] {input_id}: AUDIO (unknown size)")

                elif "seg_" in input_id:
                    # Segmenter output - show text
                    text = value[0].as_py() if hasattr(value[0], 'as_py') else str(value[0])
                    metadata = event.get("metadata", {})
                    print(f"[{ts}] {input_id}: TEXT '{text}' | metadata={metadata}")

                elif "_log" in input_id:
                    # Log messages
                    log_data = value[0].as_py() if hasattr(value[0], 'as_py') else str(value[0])
                    try:
                        log_obj = json.loads(log_data)
                        level = log_obj.get("level", "?")
                        msg = log_obj.get("message", log_data)
                        print(f"[{ts}] {input_id}: [{level}] {msg}")
                    except:
                        print(f"[{ts}] {input_id}: {log_data}")
                else:
                    # Other inputs
                    print(f"[{ts}] {input_id}: {value}")

            except Exception as e:
                print(f"[{ts}] {input_id}: ERROR parsing - {e}")

        elif event["type"] == "STOP":
            print("Viewer received STOP signal")
            break

if __name__ == "__main__":
    main()
