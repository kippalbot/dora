#!/usr/bin/env python3
"""
LLM Client Viewer - Monitor logs and events from chat terminal and LLM
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
    ENDC = '\033[0m'
    BOLD = '\033[1m'


TARGET_LABELS = {
    "to_judge_text": "Judge",
    "to_llm1_text": "LLM1",
    "to_llm2_text": "LLM2",
}

PARTICIPANT_COLORS = {
    "judge": Colors.BLUE,
    "llm1": Colors.GREEN,
    "llm2": Colors.MAGENTA,
}

PARTICIPANT_ICONS = {
    "judge": "⚖️ ",
    "llm1": "🤖",
    "llm2": "🤖",
}


def format_timestamp(ts=None):
    """Format timestamp for display"""
    if ts:
        return datetime.fromtimestamp(ts).strftime("%H:%M:%S.%f")[:-3]
    return datetime.now().strftime("%H:%M:%S.%f")[:-3]


def get_node_icon(node_name):
    """Get emoji icon for each node"""
    icons = {
        "chat-terminal": "💬",
        "qwen3-llm": "🤖",
    }
    return icons.get(node_name, "📦")


def get_level_color(level):
    """Get color for log level"""
    colors = {
        "ERROR": Colors.RED,
        "WARNING": Colors.YELLOW,
        "INFO": Colors.CYAN,
        "DEBUG": Colors.GREEN
    }
    return colors.get(level, Colors.ENDC)


def print_log(log_data):
    """Print formatted log message"""
    try:
        if isinstance(log_data, str):
            data = json.loads(log_data)
        else:
            data = log_data

        node_name = data.get("node", "unknown")
        level = data.get("level", "INFO")
        message = data.get("message", "")
        timestamp = data.get("timestamp", None)

        icon = get_node_icon(node_name)
        color = get_level_color(level)
        ts_str = format_timestamp(timestamp)

        print(f"{Colors.BOLD}[{ts_str}]{Colors.ENDC} {icon} {color}{node_name.upper()}: {message}{Colors.ENDC}", flush=True)

    except Exception as e:
        print(f"{Colors.RED}[Viewer] Failed to parse log: {e}{Colors.ENDC}", flush=True)


def print_forwarded_message(target_id: str, raw_message) -> None:
    """Display a routed message with participant and target context."""
    target_label = TARGET_LABELS.get(target_id, target_id)
    participant = "unknown"
    content = raw_message
    complete = False

    if isinstance(raw_message, str):
        try:
            payload = json.loads(raw_message)
            participant = payload.get("participant", participant)
            content = payload.get("content", content)
            complete = payload.get("complete", False)
        except json.JSONDecodeError:
            pass
    elif isinstance(raw_message, dict):
        participant = raw_message.get("participant", participant)
        content = raw_message.get("content", content)
        complete = raw_message.get("complete", False)

    participant_key = participant.lower()
    color = PARTICIPANT_COLORS.get(participant_key, Colors.CYAN)
    icon = PARTICIPANT_ICONS.get(participant_key, "💬")

    status_suffix = " (complete)" if complete else ""

    print(
        f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
        f"{icon} {Colors.BOLD}{participant.upper()} ➜ {target_label}{Colors.ENDC}{status_suffix}\n"
        f"{color}{content}{Colors.ENDC}",
        flush=True,
    )


def main():
    """Main viewer loop"""
    node = Node("viewer")

    print("\n" + "="*70)
    print(f"{Colors.BOLD}💬 LLM Chat Viewer{Colors.ENDC}")
    print("="*70)
    print("Monitoring chat events and logs...\n")

    for event in node:
        if event["type"] == "INPUT":
            input_id = event["id"]

            try:
                # Log inputs - filter out DEBUG metadata logs
                if input_id.endswith("_log"):
                    log_data = event["value"][0].as_py()
                    # Parse log data and only show INFO level and above
                    if isinstance(log_data, str):
                        try:
                            data = json.loads(log_data)
                            level = data.get("level", "INFO")
                            # Skip DEBUG logs about metadata
                            if level == "DEBUG":
                                message = data.get("message", "")
                                if "Metadata:" in message or "Received LLM chunk" in message:
                                    continue
                        except:
                            pass
                    print_log(log_data)

                # User text input
                elif input_id == "user_text":
                    text = event["value"][0].as_py()
                    print(
                        f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
                        f"👤 {Colors.BLUE}User: {text}{Colors.ENDC}",
                        flush=True
                    )

                elif input_id in TARGET_LABELS:
                    text = event["value"][0].as_py()
                    print_forwarded_message(input_id, text)

                # LLM status updates - only show important ones
                elif input_id == "llm_status":
                    status = event["value"][0].as_py()
                    # Filter out routine status messages
                    if isinstance(status, str) and status.lower() not in ["processing", "generating"]:
                        print(
                            f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} "
                            f"ℹ️  {Colors.YELLOW}Status: {status}{Colors.ENDC}",
                            flush=True
                        )

            except Exception as e:
                print(f"{Colors.RED}[Viewer] Error processing event: {e}{Colors.ENDC}", flush=True)

        elif event["type"] == "STOP":
            print(f"\n{Colors.BOLD}[{format_timestamp()}]{Colors.ENDC} {Colors.YELLOW}🛑 Viewer stopped{Colors.ENDC}\n")
            break


if __name__ == "__main__":
    main()
