#!/usr/bin/env python3
"""
Test receiver for MaaS client cancellation feature.

This script logs all events it receives, showing:
1. Text chunks as they arrive
2. Status updates (including "cancelled")
3. Cancellation detection via session_status="cancelled"
"""

import json
import sys
from pathlib import Path
from datetime import datetime

# Add dora-node-api to path for standalone testing
dora_path = Path(__file__).parent.parent.parent / "apis/python/node"
sys.path.insert(0, str(dora_path))

from dora import Node


def log_event(event_type, data, metadata):
    """Log an event with timestamp"""
    timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    print(f"[{timestamp}] 📨 {event_type}")

    if event_type == "text":
        session_status = metadata.get("session_status", "unknown")
        print(f"         Status: {session_status}")
        if session_status == "cancelled":
            print("         🚨 CANCELLATION DETECTED 🚨")
            try:
                error_data = json.loads(data)
                print(f"         Error: {error_data}")
            except:
                print(f"         Message: {data[:100]}...")
        elif data:
            print(f"         Data: {data[:100]}...")
        else:
            print("         Data: <empty>")

    elif event_type == "status":
        print(f"         Status: {data}")

    # Print metadata if interesting
    if metadata:
        interesting_keys = ['session_id', 'segment_index', 'cancellation_reason']
        interesting_metadata = {k: v for k, v in metadata.items() if k in interesting_keys}
        if interesting_metadata:
            print(f"         Metadata: {interesting_metadata}")

    print()  # Blank line for readability


def main():
    print("🎯 MaaS Client Cancellation Test - Receiver")
    print("=" * 70)
    print("Listening for events...\n")

    # Initialize Dora node
    node = Node()

    event_count = 0
    cancelled_sessions = set()

    while True:
        event = node.next()
        if event is None:
            break

        event_count += 1
        event_type = event["type"]
        data = event.get("data", [""])[0] if event.get("data") else ""
        metadata = event.get("metadata", {})

        # Track cancelled sessions via text with session_status="cancelled"
        if event_type == "text":
            session_status = metadata.get('session_status')
            session_id = metadata.get('session_id')
            if session_status == "cancelled" and session_id:
                cancelled_sessions.add(session_id)

        # Log the event
        log_event(event_type, data, metadata)

        # Summary every 10 events
        if event_count % 10 == 0:
            print("-" * 70)
            print(f"📊 Received {event_count} events so far...")
            print(f"🚫 {len(cancelled_sessions)} sessions cancelled")
            print("-" * 70)
            print()

    print("\n" + "=" * 70)
    print(f"🏁 Test receiver stopped. Total events: {event_count}")
    print(f"🚫 Sessions cancelled: {len(cancelled_sessions)}")
    if cancelled_sessions:
        print(f"   Sessions: {', '.join(cancelled_sessions)}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\n👋 Shutting down gracefully...")
        sys.exit(0)
