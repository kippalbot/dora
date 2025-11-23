#!/usr/bin/env python3
"""
Simple text sender for testing MiniMax T2A
"""
import time
from dora import Node
import pyarrow as pa

def main():
    node = Node("text-sender")

    # Wait a bit for other nodes to initialize
    time.sleep(2)

    # Send test text
    test_text = "Hello, this is a test."
    print(f"Sending test text: '{test_text}'")

    node.send_output(
        "text",
        pa.array([test_text]),
        metadata={"segment_index": 0, "segments_remaining": 0}
    )

    print("Text sent, waiting 30 seconds for processing...")

    # Keep running for 30 seconds to allow processing
    start_time = time.time()
    for event in node:
        if event["type"] == "STOP":
            break

        # Exit after 30 seconds
        if time.time() - start_time > 30:
            print("Timeout reached, exiting...")
            break

    print("Text sender stopped")

if __name__ == "__main__":
    main()
