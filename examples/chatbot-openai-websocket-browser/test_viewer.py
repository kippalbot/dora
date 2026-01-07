#!/usr/bin/env python3
"""Simple viewer for maas-client test - prints all received outputs"""
import sys
from dora import Node

def main():
    node = Node("viewer")
    print("Viewer ready. Waiting for outputs...", flush=True)

    for event in node:
        if event["type"] == "STOP":
            print("Stopped", flush=True)
            break

        if event["type"] == "INPUT":
            input_id = event["id"]
            value = event["value"]

            try:
                if len(value) > 0:
                    data = value[0].as_py()
                    print(f"[{input_id}] {data}", flush=True)
            except Exception as e:
                print(f"[{input_id}] Error: {e}", flush=True)

if __name__ == "__main__":
    main()
