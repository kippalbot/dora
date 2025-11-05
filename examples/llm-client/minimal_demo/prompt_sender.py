#!/usr/bin/env python3
"""
Minimal interactive sender for the openai-response-client demo.

Reads lines from stdin and forwards them to the dataflow as UTF-8 text.
"""

import sys
from typing import Optional

import pyarrow as pa
from dora import Node


def read_line() -> Optional[str]:
    """Return the next line from stdin, stripped of its newline."""
    try:
        line = sys.stdin.readline()
    except KeyboardInterrupt:
        return None
    if not line:
        return None
    return line.strip()


def main() -> None:
    node = Node()
    print("Type a prompt and press Enter (Ctrl+D / Ctrl+C to quit):")

    while True:
        line = read_line()
        if line is None:
            break
        if not line:
            continue
        node.send_output("text", pa.array([line]))

        # Drain any pending events (e.g. STOP) so we exit cleanly.
        event = node.next(timeout=0)
        if event is not None and event["type"] == "STOP":
            break


if __name__ == "__main__":
    main()
