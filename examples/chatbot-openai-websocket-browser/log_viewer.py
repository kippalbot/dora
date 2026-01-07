#!/usr/bin/env python3
"""
Log Viewer - Monitor logs from WebSocket chatbot dataflow
Tracks: websocket-server, speech-monitor, asr, maas-client, text-segmenter, primespeech
"""
import json
import os
import sys
import signal
import argparse
from datetime import datetime
from dora import Node

# Global flag for shutdown
_shutdown_requested = False


def _signal_handler(signum, frame):
    """Handle interrupt signals"""
    global _shutdown_requested
    _shutdown_requested = True
    print(f"\n{Colors.YELLOW}[Viewer] Shutdown requested (Ctrl-C)...{Colors.ENDC}", flush=True)


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


# Log level configuration
LOG_LEVELS = {"DEBUG": 10, "INFO": 20, "WARNING": 30, "ERROR": 40}
VIEWER_LOG_THRESHOLD = 20  # Default to INFO


# Node configuration for display
NODE_CONFIG = {
    "websocket-server": {"name": "WebSocket", "icon": "🔌", "color": Colors.BLUE},
    "wserver": {"name": "WebSocket", "icon": "🔌", "color": Colors.BLUE},
    "speech-monitor": {"name": "Speech Monitor", "icon": "👂", "color": Colors.YELLOW},
    "speechmonitor": {"name": "Speech Monitor", "icon": "👂", "color": Colors.YELLOW},
    "asr": {"name": "ASR", "icon": "🎙️", "color": Colors.GREEN},
    "maas-client": {"name": "MaaS Client", "icon": "🤖", "color": Colors.MAGENTA},
    "text-segmenter": {"name": "Text Segmenter", "icon": "✂️", "color": Colors.CYAN},
    "primespeech": {"name": "PrimeSpeech TTS", "icon": "🔊", "color": Colors.BLUE},
}


def format_timestamp(ts=None):
    """Format timestamp for display"""
    if ts:
        if ts > 1e10:  # If timestamp is in milliseconds
            ts = ts / 1000.0
        return datetime.fromtimestamp(ts).strftime("%H:%M:%S.%f")[:-3]
    return datetime.now().strftime("%H:%M:%S.%f")[:-3]


def get_node_config(node_name):
    """Get node configuration or default"""
    lower_name = node_name.lower()
    for key, config in NODE_CONFIG.items():
        if key in lower_name:
            return config
    return {"name": node_name, "icon": "📦", "color": Colors.WHITE}


def get_node_config_from_input_id(input_id):
    """Get node configuration based on input ID"""
    lower_id = input_id.lower()

    for key, config in NODE_CONFIG.items():
        if key in lower_id:
            return config

    # Extract base node name from input_id
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

        # Determine node config
        if input_id:
            config = get_node_config_from_input_id(input_id)
        else:
            node_name = data.get("node", "unknown")
            config = get_node_config(node_name)

        icon = config["icon"]
        node_color = config["color"]
        level_color = get_level_color(level)
        ts_str = format_timestamp(timestamp)

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


def print_speech_event(event_type, value, metadata=None):
    """Print speech monitor event"""
    config = NODE_CONFIG.get("speech-monitor", {"name": "Speech Monitor", "icon": "👂", "color": Colors.YELLOW})
    icon = config["icon"]
    color = config["color"]
    ts_str = format_timestamp()

    # Event-specific formatting
    event_icons = {
        "speech_started": "🎤",
        "speech_ended": "🔇",
        "question_ended": "❓",
        "is_speaking": "🗣️",
    }
    event_colors = {
        "speech_started": Colors.GREEN,
        "speech_ended": Colors.YELLOW,
        "question_ended": Colors.CYAN,
        "is_speaking": Colors.MAGENTA,
    }

    event_icon = event_icons.get(event_type, "📢")
    event_color = event_colors.get(event_type, Colors.WHITE)

    # Format the value for display
    if isinstance(value, bool):
        value_str = "True" if value else "False"
    elif value is None:
        value_str = "(signal)"
    else:
        value_str = str(value)

    print(
        f"{Colors.BOLD}[{ts_str}]{Colors.ENDC} "
        f"{icon} {color}{config['name']}{Colors.ENDC} "
        f"{event_icon} {event_color}{event_type}{Colors.ENDC}: {value_str}",
        flush=True
    )


def main(node_name: str):
    """Main viewer loop"""
    global _shutdown_requested

    # Register signal handlers
    signal.signal(signal.SIGINT, _signal_handler)
    signal.signal(signal.SIGTERM, _signal_handler)

    node = Node(node_name)

    print("\n" + "="*80)
    print(f"{Colors.BOLD}🌐 WebSocket Chatbot Log Viewer{Colors.ENDC}")
    print("="*80)
    print("Monitoring chatbot dataflow logs and events... (Ctrl-C to stop)\n")

    while not _shutdown_requested:
        event = node.next(timeout=0.5)
        if event is None:
            continue

        if event["type"] == "INPUT":
            input_id = event["id"]

            try:
                # Parse input_id to get node name
                if "/" in input_id:
                    node_id = input_id.split("/")[0]
                elif "_" in input_id:
                    parts = input_id.rsplit("_", 1)
                    node_id = parts[0]
                else:
                    node_id = input_id

                value = event["value"]

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

                # Handle speech monitor events
                elif input_id in ("speech_started", "speech_ended", "question_ended", "is_speaking"):
                    if len(value) > 0:
                        event_value = value[0].as_py()
                        print_speech_event(input_id, event_value, event.get("metadata"))

            except Exception as e:
                print(
                    f"{Colors.RED}[Viewer] Error processing event '{input_id}': {e}{Colors.ENDC}",
                    flush=True
                )

        elif event["type"] == "STOP":
            print(
                f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
                f"{Colors.YELLOW}Viewer stopped (dataflow ended){Colors.ENDC}\n"
            )
            break

    if _shutdown_requested:
        print(
            f"{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
            f"{Colors.YELLOW}Viewer exited{Colors.ENDC}\n"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="WebSocket Chatbot Log Viewer")
    parser.add_argument(
        "--name",
        type=str,
        default=os.environ.get("DORA_NODE_ID", "viewer"),
        help="Node name for dynamic node initialization"
    )
    parser.add_argument(
        "--log-level",
        type=str,
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        default=os.environ.get("LOG_LEVEL", "INFO").upper(),
        help="Set the minimum log level to display"
    )
    args = parser.parse_args()

    VIEWER_LOG_THRESHOLD = LOG_LEVELS.get(args.log_level, 20)
    print(f"{Colors.DIM}[Viewer] Log level set to: {args.log_level}{Colors.ENDC}")

    main(args.name)
