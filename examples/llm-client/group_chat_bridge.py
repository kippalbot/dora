#!/usr/bin/env python3
"""Bridge node that forwards user text to multiple LLM backends."""
import json
import os
from typing import List

import pyarrow as pa
from dora import Node


DEFAULT_TARGETS = [
    "openai-response-client",  # responses API
    "maas-client",             # legacy MaaS client
]


def load_targets() -> List[str]:
    env_value = os.getenv("GROUP_CHAT_TARGETS")
    if env_value:
        try:
            return json.loads(env_value)
        except json.JSONDecodeError:
            print("Failed to parse GROUP_CHAT_TARGETS, falling back to defaults")
    return DEFAULT_TARGETS


def main() -> None:
    targets = load_targets()
    print("Group chat bridge forwarding to:", targets)

    node = Node()

    while True:
        event = node.next()
        if event is None:
            continue
        if event["type"] == "STOP":
            break
        if event["type"] != "INPUT":
            continue

        payload = event["value"].to_pylist()
        arrow_payload = pa.array(payload)

        for target in targets:
            node.send_output(target, arrow_payload)


if __name__ == "__main__":
    main()
