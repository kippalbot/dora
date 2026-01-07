#!/usr/bin/env python3
"""Simple test sender for maas-client - interactive mode"""
import time
from dora import Node
import pyarrow as pa

def main():
    node = Node("text-source")
    print("Text source ready. Type messages to send (Ctrl+C to quit):")
    print("-" * 50)

    try:
        while True:
            # Get user input
            user_input = input("\nYou: ").strip()

            if not user_input:
                continue

            if user_input.lower() in ['quit', 'exit', 'q']:
                print("Exiting...")
                break

            # Send message
            node.send_output(
                "text",
                pa.array([user_input]),
            )
            print("(sent, waiting for response...)")

            # Wait a bit for response to come through viewer
            time.sleep(0.5)

    except KeyboardInterrupt:
        print("\nStopped by user")
    except EOFError:
        print("\nInput closed")

if __name__ == "__main__":
    main()
