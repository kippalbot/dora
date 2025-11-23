#!/usr/bin/env python3
"""
Debate Viewer - Monitor logs and events from LLM debate dataflow
Tracks LLM1, LLM2, Judge, and Bridge nodes
"""
import json
from datetime import datetime
import pyarrow as pa
from dora import Node


class Colors:
    """ANSI color codes for terminal output"""
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BLUE = '\033[94m'
    MAGENTA = '\033[95m'
    WHITE = '\033[97m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    DIM = '\033[2m'


STREAM_BUFFERS = {}

NODE_CONFIG_ENTRIES = [
    ("bridge-to-judge", {
        "name": "LLM1+LLM2->Judge",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge llm1+llm2->judge", {
        "name": "LLM1+LLM2->Judge",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge-to-llm1", {
        "name": "LLM2+Judge->LLM1",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge1", {
        "name": "LLM1+LLM2->Judge",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge llm2+judge->llm1", {
        "name": "LLM2+Judge->LLM1",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge-to-llm2", {
        "name": "LLM1+Judge->LLM2",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge2", {
        "name": "LLM1+Judge->LLM2",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge llm1+judge->llm2", {
        "name": "LLM1+Judge->LLM2",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("bridge3", {
        "name": "LLM2+Judge->LLM1",
        "icon": "🌉",
        "color": Colors.YELLOW
    }),
    ("llm1", {
        "name": "LLM1 (Debater A)",
        "icon": "🤖",
        "color": Colors.CYAN
    }),
    ("llm2", {
        "name": "LLM2 (Debater B)",
        "icon": "🤖",
        "color": Colors.GREEN
    }),
    ("judge", {
        "name": "Judge (Moderator)",
        "icon": "⚖️ ",
        "color": Colors.MAGENTA
    }),
    ("openai-response-client", {
        "name": "OpenAI Client",
        "icon": "🔌",
        "color": Colors.BLUE
    }),
    ("dora-maas-client", {
        "name": "MaaS Client",
        "icon": "🔌",
        "color": Colors.BLUE
    }),
    ("conference-controller", {
        "name": "Controller",
        "icon": "🎯",
        "color": Colors.BLUE
    }),
    ("controller", {
        "name": "Controller",
        "icon": "🎯",
        "color": Colors.BLUE
    }),
]


def format_timestamp(ts=None):
    """Format timestamp for display"""
    if ts:
        return datetime.fromtimestamp(ts).strftime("%H:%M:%S.%f")[:-3]
    return datetime.now().strftime("%H:%M:%S.%f")[:-3]


def get_node_config(node_name):
    """Get node configuration or default"""
    lower_name = node_name.lower()
    for key, config in NODE_CONFIG_ENTRIES:
        if key in lower_name:
            return config
    return {"name": node_name, "icon": "📦", "color": Colors.WHITE}


def get_node_config_from_input_id(input_id):
    """Get node configuration based on input ID (more reliable for MaaS clients)"""
    # Bridge logs: distinguish from LLM logs
    if "bridge" in input_id.lower():
        # Extract bridge number/name from input_id like "bridge3_log"
        if "bridge1" in input_id or "bridge-to-judge" in input_id:
            return {"name": "Bridge to Judge", "icon": "🌉", "color": Colors.YELLOW}
        elif "bridge2" in input_id or "bridge-to-llm2" in input_id:
            return {"name": "Bridge to LLM2", "icon": "🌉", "color": Colors.YELLOW}
        elif "bridge3" in input_id or "bridge-to-llm1" in input_id:
            return {"name": "Bridge to LLM1", "icon": "🌉", "color": Colors.YELLOW}
        else:
            return {"name": "Bridge", "icon": "🌉", "color": Colors.YELLOW}

    # LLM logs: distinguish which LLM instance
    elif "llm1" in input_id.lower():
        return {"name": "LLM1 (Debater A)", "icon": "🤖", "color": Colors.CYAN}
    elif "llm2" in input_id.lower():
        return {"name": "LLM2 (Debater B)", "icon": "🤖", "color": Colors.GREEN}
    elif "judge" in input_id.lower():
        return {"name": "Judge (Moderator)", "icon": "⚖️", "color": Colors.MAGENTA}
    elif "controller" in input_id.lower():
        return {"name": "Controller", "icon": "🎯", "color": Colors.BLUE}

    # Fallback to original logic
    else:
        # Extract base node name from input_id (remove suffix like _log, _status, _text)
        if "_" in input_id:
            node_name = input_id.rsplit("_", 1)[0]
        elif "/" in input_id:
            node_name = input_id.split("/")[0]
        else:
            node_name = input_id

        return get_node_config(node_name)


def get_level_color(level):
    """Get color for log level"""
    colors = {
        "ERROR": Colors.RED,
        "WARNING": Colors.YELLOW,
        "INFO": Colors.CYAN,
        "DEBUG": Colors.GREEN
    }
    return colors.get(level, Colors.ENDC)


def print_log(log_data, input_id=None):
    """Print formatted log message"""
    try:
        if isinstance(log_data, str):
            data = json.loads(log_data)
        else:
            data = log_data

        level = data.get("level", "INFO")
        message = data.get("message", "")
        timestamp = data.get("timestamp", None)

        # Filter out overly verbose DEBUG logs
        if level == "DEBUG":
            skip_phrases = [
                "Metadata:",
                "Received LLM chunk",
                "Received text delta",
                "Received other event"
            ]
            if any(phrase in message for phrase in skip_phrases):
                return

        # Determine node config: prioritize input_id for accurate identification
        if input_id:
            config = get_node_config_from_input_id(input_id)
        else:
            # Fallback to using log data node name
            node_name = data.get("node", "unknown")
            config = get_node_config(node_name)

        icon = config["icon"]
        node_color = config["color"]
        level_color = get_level_color(level)
        ts_str = format_timestamp(timestamp)

        # Format: [timestamp] ICON NODE_NAME: [LEVEL] message
        print(
            f"{Colors.BOLD}[{ts_str}]{Colors.ENDC} "
            f"{icon} {node_color}{config['name']}{Colors.ENDC}: "
            f"{level_color}[{level}]{Colors.ENDC} {message}",
            flush=True
        )

    except Exception as e:
        print(f"{Colors.RED}[Viewer] Failed to parse log: {e}{Colors.ENDC}", flush=True)


def print_status(node_id, status):
    """Print status update"""
    if isinstance(status, str) and status.lower().startswith("waiting"):
        return
    config = get_node_config(node_id)
    icon = config["icon"]
    color = config["color"]
    ts_str = format_timestamp()

    status_color = Colors.ENDC
    if "error" in status.lower():
        status_color = Colors.RED
    elif "complete" in status.lower():
        status_color = Colors.GREEN
    elif "processing" in status.lower() or "streaming" in status.lower():
        status_color = Colors.YELLOW

    print(
        f"{Colors.BOLD}[{ts_str}]{Colors.ENDC} "
        f"{icon} {color}{config['name']}{Colors.ENDC}: "
        f"{status_color}Status: {status}{Colors.ENDC}",
        flush=True
    )


def print_text_output(node_id, text, metadata=None):
    """Print text output from a node"""
    config = get_node_config(node_id)
    icon = config["icon"]
    color = config["color"]
    ts_str = format_timestamp()

    session_status = ""
    status = ""
    if metadata:
        status = metadata.get("session_status")
        if isinstance(status, list):
            status = status[0] if status else ""
        if status is not None and not isinstance(status, str):
            status = str(status)

    if status == "started":
        STREAM_BUFFERS[node_id] = text
        return
    elif status == "ongoing":
        previous = STREAM_BUFFERS.get(node_id, "")
        if text.startswith(previous):
            STREAM_BUFFERS[node_id] = text
        elif previous.endswith(text):
            STREAM_BUFFERS[node_id] = previous
        else:
            STREAM_BUFFERS[node_id] = previous + text
        return
    elif status == "ended":
        buffered = STREAM_BUFFERS.pop(node_id, "")
        if buffered:
            text = buffered
        status = ""

    if status:
        session_status = f" [{status}]"

    display_text = text if len(text) <= 100 else text[:100] + "..."

    print(
        f"{Colors.BOLD}[{ts_str}]{Colors.ENDC} "
        f"{icon} {color}{config['name']}{Colors.ENDC}{session_status}: "
        f"{Colors.DIM}{display_text}{Colors.ENDC}",
        flush=True
    )


def print_bridge_bundle(node_id, bundle_data):
    """Print bundled messages from bridge"""
    config = get_node_config(node_id)
    icon = config["icon"]
    color = config["color"]
    ts_str = format_timestamp()

    print(
        f"\n{Colors.BOLD}[{ts_str}]{Colors.ENDC} "
        f"{icon} {color}{config['name']} Forwarding Bundle:{Colors.ENDC}",
        flush=True
    )

    try:
        if isinstance(bundle_data, str):
            bundle = json.loads(bundle_data)
        else:
            bundle = bundle_data

        if isinstance(bundle, list):
            for msg in bundle:
                participant = msg.get("participant", "unknown")
                content = msg.get("content", "")
                complete = msg.get("complete", False)

                p_config = get_node_config(participant)
                p_icon = p_config["icon"]
                p_color = p_config["color"]

                status_suffix = " ✓" if complete else ""
                display_content = content if len(content) <= 80 else content[:80] + "..."

                print(
                    f"  {p_icon} {p_color}{participant.upper()}{Colors.ENDC}{status_suffix}: "
                    f"{Colors.DIM}{display_content}{Colors.ENDC}",
                    flush=True
                )
        print()  # Empty line after bundle

    except Exception as e:
        print(f"{Colors.RED}[Viewer] Failed to parse bundle: {e}{Colors.ENDC}", flush=True)


def print_control_command(node_id, control_cmd):
    """Print control command from controller"""
    config = get_node_config("controller")  # Use controller config for control commands
    icon = config["icon"]
    color = config["color"]
    ts_str = format_timestamp()

    # Map control input ID to the actual bridge that receives it
    if node_id == "control_judge":
        target_name = "bridge-to-judge"
        channel_desc = "control_judge input"
    elif node_id == "control_llm2":
        target_name = "bridge-to-llm2"
        channel_desc = "control_llm2 input"
    elif node_id == "control_llm1":
        target_name = "bridge-to-llm1"
        channel_desc = "control_llm1 input"
    elif node_id == "control":
        target_name = "UNKNOWN_BRIDGE"
        channel_desc = "control input"
    else:
        target_name = node_id
        channel_desc = f"{node_id} input"

    target_config = get_node_config(target_name)
    target_icon = target_config["icon"]
    target_color = target_config["color"]

    print(
        f"{Colors.BOLD}[{ts_str}]{Colors.ENDC} "
        f"{icon} {color}CONTROLLER SENDS{Colors.ENDC} → "
        f"{target_icon} {target_color}{target_config['name'].upper()}{Colors.ENDC} "
        f"({channel_desc}): {Colors.YELLOW}{control_cmd}{Colors.ENDC}",
        flush=True
    )


def main():
    """Main viewer loop"""
    node = Node("viewer")

    print("\n" + "="*80)
    print(f"{Colors.BOLD}⚖️  LLM Debate Viewer{Colors.ENDC}")
    print("="*80)
    print("Monitoring debate dataflow logs and events...\n")

    for event in node:
        if event["type"] == "INPUT":
            input_id = event["id"]

            try:
                # Parse input_id to get node name (format: nodename/output or nodename_output)
                if "/" in input_id:
                    node_id = input_id.split("/")[0]
                elif "_" in input_id:
                    parts = input_id.rsplit("_", 1)
                    node_id = parts[0]
                    output_name = parts[1] if len(parts) > 1 else ""
                else:
                    node_id = input_id
                    output_name = ""

                value = event["value"]
                metadata = event.get("metadata", {})

                # Handle log outputs
                if input_id.endswith("_log") or input_id.endswith("/log"):
                    if len(value) > 0:
                        log_data = value[0].as_py()
                        print_log(log_data, input_id)

                # Handle status outputs
                elif input_id.endswith("_status") or input_id.endswith("/status"):
                    if len(value) > 0:
                        status = value[0].as_py()
                        if isinstance(status, str) and status:
                            print_status(node_id, status)

                # Handle control command outputs (controller sending resume)
                # Use input_id directly for control commands since they are named control_judge, control_llm1, etc.
                elif input_id.startswith("control_") or input_id == "control":
                    if len(value) > 0:
                        control_cmd = value[0].as_py()
                        if isinstance(control_cmd, str) and control_cmd.strip():
                            print_control_command(input_id, control_cmd)

                # Handle text outputs
                elif input_id.endswith("_text") or input_id.endswith("/text"):
                    if len(value) > 0:
                        text = value[0].as_py()
                        if isinstance(text, str) and text:
                            # Check if it's a bridge bundle (JSON array)
                            stripped = text.lstrip()
                            if "bridge" in node_id.lower() and stripped.startswith("[{"):
                                print_bridge_bundle(node_id, stripped)
                            else:
                                print_text_output(node_id, text, metadata)

            except Exception as e:
                print(
                    f"{Colors.RED}[Viewer] Error processing event '{input_id}': {e}{Colors.ENDC}",
                    flush=True
                )

        elif event["type"] == "STOP":
            print(
                f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
                f"{Colors.YELLOW}🛑 Debate viewer stopped{Colors.ENDC}\n"
            )
            break


if __name__ == "__main__":
    main()
