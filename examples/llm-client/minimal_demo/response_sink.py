#!/usr/bin/env python3
"""
Simple sink that prints responses coming from openai-response-client.
"""

import json
from typing import Any

from dora import Node


def format_event(event_id: str, value: Any) -> str:
    """Return a human-friendly representation for the incoming value."""
    if hasattr(value, "to_pylist"):
        py_value = value.to_pylist()
        if len(py_value) == 1:
            payload = py_value[0]
        else:
            payload = py_value
    else:
        payload = value

    if event_id == "tool_calls" and isinstance(payload, str):
        try:
            parsed = json.loads(payload)
        except json.JSONDecodeError:
            return f"{event_id}: {payload}"
        return f"{event_id}: {json.dumps(parsed, indent=2)}"

    return f"{event_id}: {payload}"


def main() -> None:
    node = Node()
    while True:
        event = node.next()
        if event is None:
            continue
        if event["type"] == "STOP":
            break
        if event["type"] != "INPUT":
            continue

        print(format_event(event["id"], event["value"]))


if __name__ == "__main__":
    main()
