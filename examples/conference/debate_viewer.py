#!/usr/bin/env python3
"""
Debate Viewer - Monitor logs and events from LLM debate dataflow
Tracks LLM1, LLM2, Judge, and Bridge nodes
"""
import json
import os
import sys
import argparse
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

# Log level configuration
LOG_LEVELS = {"DEBUG": 10, "INFO": 20, "WARNING": 30, "ERROR": 40}
# Will be set by parse_args() or default to env variable
VIEWER_LOG_THRESHOLD = 20  # Default to INFO

def get_participant_display_names():
    """Dynamically detect participant names from environment or use defaults"""
    # Check if we're in study mode by looking for study-specific environment variables
    if os.environ.get("DORA_STUDY_MODE", "").lower() in {"1", "true", "yes"}:
        return {
            "student1": "Student1 (Daniu)",
            "student2": "Student2 (Yifei)",
            "tutor": "Tutor (Sunwen)",
            "conversation": "Dialogue Bundle",
            "system": "System",
        }
    else:
        # Default debate mode
        return {
            "llm1": "LLM1 (Debater A)",
            "llm2": "LLM2 (Debater B)",
            "judge": "Judge (Moderator)",
            "conversation": "Dialogue Bundle",
            "system": "System",
        }

def get_display_name(participant_id):
    names = get_participant_display_names()
    return names.get(participant_id, participant_id.upper())

def get_node_config_entries():
    """Get dynamic node configuration entries based on mode"""
    display_names = get_participant_display_names()

    # Common configurations
    common_entries = [
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
        ("audio-player", {
            "name": "Audio Player",
            "icon": "🔊",
            "color": Colors.CYAN
        }),
        ("audio_player", {
            "name": "Audio Player",
            "icon": "🔊",
            "color": Colors.CYAN
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

    if "student1" in display_names:
        # Study mode configuration
        study_entries = [
            ("bridge-to-tutor", {
                "name": "Student1+Student2->Tutor",
                "icon": "🌉",
                "color": Colors.YELLOW
            }),
            ("bridge-to-student2", {
                "name": "Student1+Tutor->Student2",
                "icon": "🌉",
                "color": Colors.YELLOW
            }),
            ("bridge-to-student1", {
                "name": "Student2+Tutor->Student1",
                "icon": "🌉",
                "color": Colors.YELLOW
            }),
            ("bridge1", {
                "name": "Student1+Student2->Tutor",
                "icon": "🌉",
                "color": Colors.YELLOW
            }),
            ("bridge2", {
                "name": "Student1+Tutor->Student2",
                "icon": "🌉",
                "color": Colors.YELLOW
            }),
            ("bridge3", {
                "name": "Student2+Tutor->Student1",
                "icon": "🌉",
                "color": Colors.YELLOW
            }),
            ("student1", {
                "name": "Student1 (Daniu)",
                "icon": "🤖",
                "color": Colors.CYAN
            }),
            ("student2", {
                "name": "Student2 (Yifei)",
                "icon": "🤖",
                "color": Colors.GREEN
            }),
            ("tutor", {
                "name": "Tutor (Sunwen)",
                "icon": "⚖️ ",
                "color": Colors.MAGENTA
            }),
        ]
        return study_entries + common_entries
    else:
        # Debate mode configuration
        debate_entries = [
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
        ]
        return debate_entries + common_entries

NODE_CONFIG_ENTRIES = []  # Will be populated dynamically


def format_timestamp(ts=None):
    """Format timestamp for display"""
    if ts:
        # Handle both seconds and milliseconds (controller sends milliseconds)
        if ts > 1e10:  # If timestamp is in milliseconds (> year 2286 in seconds)
            ts = ts / 1000.0
        return datetime.fromtimestamp(ts).strftime("%H:%M:%S.%f")[:-3]
    return datetime.now().strftime("%H:%M:%S.%f")[:-3]


def get_node_config(node_name):
    """Get node configuration or default"""
    lower_name = node_name.lower()
    for key, config in get_node_config_entries():
        if key in lower_name:
            return config
    return {"name": node_name, "icon": "📦", "color": Colors.WHITE}


def get_node_config_from_input_id(input_id):
    """Get node configuration based on input ID (more reliable for MaaS clients)"""
    display_names = get_participant_display_names()

    # Bridge logs: distinguish from LLM logs
    if "bridge" in input_id.lower():
        # Extract bridge number/name from input_id like "bridge3_log"
        if "bridge1" in input_id or "bridge-to-tutor" in input_id or "bridge-to-judge" in input_id:
            target = "tutor" if "tutor" in display_names else "judge"
            target_display = get_display_name(target)
            return {"name": f"Bridge to {target_display}", "icon": "🌉", "color": Colors.YELLOW}
        elif "bridge2" in input_id or "bridge-to-student2" in input_id or "bridge-to-llm2" in input_id:
            source = "student2" if "student2" in display_names else "llm2"
            source_display = get_display_name(source)
            return {"name": f"Bridge to {source_display}", "icon": "🌉", "color": Colors.YELLOW}
        elif "bridge3" in input_id or "bridge-to-student1" in input_id or "bridge-to-llm1" in input_id:
            source = "student1" if "student1" in display_names else "llm1"
            source_display = get_display_name(source)
            return {"name": f"Bridge to {source_display}", "icon": "🌉", "color": Colors.YELLOW}
        else:
            return {"name": "Bridge", "icon": "🌉", "color": Colors.YELLOW}

    # LLM logs: distinguish which LLM instance
    for participant_id in ["student1", "student2", "tutor", "llm1", "llm2", "judge"]:
        if participant_id in input_id.lower():
            return {"name": get_display_name(participant_id), "icon": "🤖", "color": Colors.CYAN if participant_id in ["student1", "llm1"] else Colors.GREEN if participant_id in ["student2", "llm2"] else Colors.MAGENTA}

    if "controller" in input_id.lower():
        return {"name": "Controller", "icon": "🎯", "color": Colors.BLUE}

    if "audio" in input_id.lower() and ("player" in input_id.lower() or "status" in input_id.lower()):
        return {"name": "Audio Player", "icon": "🔊", "color": Colors.CYAN}

    # Human speech pipeline nodes
    if "asr" in input_id.lower():
        return {"name": "ASR", "icon": "🎙️", "color": Colors.GREEN}

    if "mac_aec" in input_id.lower() or "mac-aec" in input_id.lower():
        return {"name": "mac_aec", "icon": "📦", "color": Colors.YELLOW}

    # Fallback to original logic
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

        # Filter by log level threshold
        log_level_value = LOG_LEVELS.get(level, 20)
        if log_level_value < VIEWER_LOG_THRESHOLD:
            return

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

    display_names = get_participant_display_names()

    # Map control input ID to the actual bridge that receives it
    if node_id == "control_judge" or node_id == "control_tutor":
        if "tutor" in display_names:
            target_name = "bridge-to-tutor"
            channel_desc = "control_tutor input"
        else:
            target_name = "bridge-to-judge"
            channel_desc = "control_judge input"
    elif node_id == "control_llm2" or node_id == "control_student2":
        if "student2" in display_names:
            target_name = "bridge-to-student2"
            channel_desc = "control_student2 input"
        else:
            target_name = "bridge-to-llm2"
            channel_desc = "control_llm2 input"
    elif node_id == "control_llm1" or node_id == "control_student1":
        if "student1" in display_names:
            target_name = "bridge-to-student1"
            channel_desc = "control_student1 input"
        else:
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

    display_names = get_participant_display_names()

    # Determine mode and set appropriate title
    if "student1" in display_names:
        title = f"{Colors.BOLD}📚 Study Session Viewer{Colors.ENDC}"
        description = "Monitoring study session dataflow logs and events..."
    else:
        title = f"{Colors.BOLD}⚖️  LLM Debate Viewer{Colors.ENDC}"
        description = "Monitoring debate dataflow logs and events..."

    print("\n" + "="*80)
    print(title)
    print("="*80)
    print(f"{description}\n")

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
            display_names = get_participant_display_names()
            viewer_type = "Study Session" if "student1" in display_names else "Debate"
            print(
                f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
                f"{Colors.YELLOW}🛑 {viewer_type} viewer stopped{Colors.ENDC}\n"
            )
            break


def set_log_level_and_run():
    """Parse arguments, set log level, and run main."""
    global VIEWER_LOG_THRESHOLD

    # Parse command line arguments
    parser = argparse.ArgumentParser(
        description="Debate/Study Session Viewer - Monitor dataflow logs and events",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--log-level",
        type=str,
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        default=os.environ.get("LOG_LEVEL", "INFO").upper(),
        help="Set the minimum log level to display (default: INFO or from LOG_LEVEL env var)"
    )

    args = parser.parse_args()

    # Set global log threshold based on argument
    VIEWER_LOG_THRESHOLD = LOG_LEVELS.get(args.log_level, 20)

    print(f"{Colors.DIM}[Viewer] Log level set to: {args.log_level} (threshold: {VIEWER_LOG_THRESHOLD}){Colors.ENDC}")

    main()


if __name__ == "__main__":
    set_log_level_and_run()
