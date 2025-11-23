#!/usr/bin/env python3
"""
Debate Monitor - Real-time visualization of LLM debate
Displays 3 participants: LLM1 (Debater A), Judge (Moderator), LLM2 (Debater B)
"""

import argparse
import sys
import os
import json
import time
import threading
from typing import Optional, Dict, Any, List
from queue import Queue, Empty

import pyarrow as pa
from dora import Node
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical, ScrollableContainer, Container
from textual.widgets import Header, Footer, Input, Static, Button
from textual.binding import Binding
from rich.text import Text
from rich.console import Group, Console
from rich.markdown import Markdown as RichMarkdown

# Default node name
NODE_NAME = "debate-monitor"

PARTICIPANT_DISPLAY_NAMES = {
    "llm1": "LLM1 (Debater A)",
    "llm2": "LLM2 (Debater B)",
    "judge": "Judge (Moderator)",
    "conversation": "Dialogue Bundle",
    "system": "System",
}

CONVERSATION_INPUT_IDS = {"bundle_text"}
PROMPT_SUFFIX = "_prompt"

# Queue for communication between Dora thread and Textual app
message_queue = Queue()
dora_node_queue = Queue()  # For sending control messages to judge
stop_event = threading.Event()


def _parameter_to_primitive(value: Any):
    """Convert Dora Parameter representations to plain Python values"""
    if value is None:
        return None
    if isinstance(value, (str, bool, int, float)):
        return value
    if isinstance(value, dict):
        if "value" in value and len(value) == 1:
            return _parameter_to_primitive(value["value"])
        return {k: _parameter_to_primitive(v) for k, v in value.items()}
    if hasattr(value, "as_py"):
        try:
            return value.as_py()
        except Exception:
            pass
    if hasattr(value, "value"):
        inner = getattr(value, "value")
        if isinstance(inner, (str, bool, int, float)):
            return inner
    value_str = str(value)
    if value_str.startswith('Parameter(String("') and value_str.endswith('"))'):
        return value_str[len('Parameter(String("'):-2]
    if value_str.startswith('Parameter(Bool(') and value_str.endswith('))'):
        inner = value_str[len('Parameter(Bool('):-2]
        if inner.lower() in {"true", "false"}:
            return inner.lower() == "true"
    return value_str


def _extract_first_value(value_array: Any) -> str:
    """Return first element from a pyarrow array or python iterable as string"""
    try:
        if value_array is None:
            return ""
        if hasattr(value_array, "__len__") and len(value_array) > 0:
            item = value_array[0]
            if hasattr(item, "as_py"):
                return item.as_py() or ""
            return item or ""
        if hasattr(value_array, "as_py"):
            result = value_array.as_py()
            if isinstance(result, list) and result:
                first = result[0]
                return first if isinstance(first, str) else str(first)
            return result if isinstance(result, str) else str(result)
    except Exception as err:
        print(f"[debate-monitor] Failed to extract value: {err}")
    return ""


def _normalize_metadata_value(metadata: Dict[str, Any], key: str, default=None):
    """Extract metadata value with best-effort conversion to primitive"""
    if metadata is None:
        return default

    raw = default

    if hasattr(metadata, "get"):
        raw = metadata.get(key, default)
    elif hasattr(metadata, "parameters"):
        params = getattr(metadata, "parameters")
        if hasattr(params, "get"):
            raw = params.get(key, default)
        elif hasattr(params, "__getitem__"):
            try:
                raw = params[key]
            except Exception:
                raw = default

    if raw is None:
        return default
    return _parameter_to_primitive(raw)


class ParticipantMessage(Static):
    """A message from a debate participant"""

    def __init__(self, participant: str, content: str, **kwargs):
        if "classes" not in kwargs:
            kwargs["classes"] = f"participant-msg {participant}"
        else:
            kwargs["classes"] = f"{kwargs['classes']} participant-msg {participant}"

        super().__init__("", **kwargs)
        self.participant = participant
        self.content = content
        self.update(self._create_renderable())

    def _normalize_markdown(self, text: str) -> str:
        """Normalize markdown text for proper rendering"""
        import re
        text = re.sub(r'([^\n])(#{1,6} )', r'\1\n\n\2', text)
        text = re.sub(r'([^\n])(\n[-*+] )', r'\1\n\2', text)
        text = re.sub(r'([^\n])(\n\d+\. )', r'\1\n\2', text)
        text = re.sub(r'([^\n])(```)', r'\1\n\n\2', text)
        return text

    def _create_renderable(self):
        """Create renderable content with participant styling"""
        normalized_content = self._normalize_markdown(self.content)
        return RichMarkdown(normalized_content)

    def update_content(self, content: str):
        """Update message content"""
        self.content = content
        self.update(self._create_renderable())


class ParticipantPanel(ScrollableContainer):
    """Panel displaying messages from one participant"""

    def __init__(self, participant: str, title: str, color: str, **kwargs):
        super().__init__(**kwargs)
        self.participant = participant
        self.title = title
        self.color = color
        self.current_message: Optional[ParticipantMessage] = None
        self.current_message_id = 0
        self.border_title = f"[{color}]{title}[/]"

    def add_message(self, content: str):
        """Add a new message to this panel"""
        self.current_message_id += 1
        msg_id = f"{self.participant}_msg_{self.current_message_id}"
        msg = ParticipantMessage(self.participant, content, id=msg_id)
        self.current_message = msg
        self.mount(msg)
        self.scroll_end(animate=False)

    def update_current_message(self, content: str):
        """Update the current streaming message"""
        if self.current_message:
            self.current_message.update_content(content)
            self.scroll_end(animate=False)
        else:
            self.add_message(content)

    def mark_complete(self):
        """Mark current message as complete"""
        self.current_message = None


class StatusBar(Static):
    """Status bar showing debate state"""

    def __init__(self, **kwargs):
        super().__init__("", **kwargs)
        self.statuses: Dict[str, str] = {
            "llm1": "idle",
            "llm2": "idle",
            "judge": "idle"
        }
        self.update_display()

    def set_status(self, participant: str, status: str):
        """Update participant status"""
        self.statuses[participant] = status
        self.update_display()

    def update_display(self):
        """Update the status display"""
        llm1_color = self._get_status_color(self.statuses["llm1"])
        llm2_color = self._get_status_color(self.statuses["llm2"])
        judge_color = self._get_status_color(self.statuses["judge"])

        status_text = Text.assemble(
            ("LLM1: ", "bold"),
            (self.statuses["llm1"], llm1_color),
            (" | ", "dim"),
            ("Judge: ", "bold"),
            (self.statuses["judge"], judge_color),
            (" | ", "dim"),
            ("LLM2: ", "bold"),
            (self.statuses["llm2"], llm2_color),
        )
        self.update(status_text)

    def _get_status_color(self, status: str) -> str:
        """Get color for status"""
        colors = {
            "idle": "dim",
            "processing": "yellow",
            "streaming": "cyan",
            "complete": "green",
            "error": "red",
            "prompted": "magenta",
        }
        return colors.get(status, "white")


class DebateMonitorApp(App):
    """Textual app for monitoring LLM debate"""

    CSS = """
    Screen {
        background: $surface;
    }

    Header {
        background: $accent;
    }

    #content {
        height: 1fr;
        border: solid $primary;
    }

    #panel-judge {
        height: 0.45fr;
        padding: 1;
        border-bottom: solid $primary;
    }

    #panels-container {
        height: 0.55fr;
    }

    .participant-panel {
        width: 1fr;
        height: 100%;
        border: solid $primary;
        padding: 1;
    }

    .participant-msg {
        margin: 1 0;
        padding: 1;
    }

    .llm1 {
        background: rgb(40,40,80);
        color: rgb(120,150,255);
    }

    .judge {
        background: rgb(60,40,60);
        color: rgb(255,180,255);
    }

    .llm2 {
        background: rgb(40,60,40);
        color: rgb(150,255,150);
    }

    #status-bar {
        height: 1;
        background: $panel;
        padding: 0 1;
    }

    #controls {
        dock: bottom;
        height: 3;
        background: $panel;
    }

    #input {
        width: 1fr;
        margin: 0 1;
    }

    #button-container {
        width: 22;
        min-width: 22;
        max-width: 22;
    }

    #reset-button {
        width: 100%;
        margin: 0 1 0 0;
    }
    """

    BINDINGS = [
        Binding("ctrl+c", "quit", "Quit", priority=True),
        Binding("ctrl+l", "clear", "Clear"),
    ]

    def compose(self) -> ComposeResult:
        """Create the UI layout"""
        yield Header(show_clock=True)

        with Vertical(id="content"):
            yield ParticipantPanel("judge", "Judge (Moderator)", "magenta", classes="participant-panel", id="panel-judge")
            with Horizontal(id="panels-container"):
                yield ParticipantPanel("llm1", "LLM1 (Debater A)", "cyan", classes="participant-panel", id="panel-llm1")
                yield ParticipantPanel("llm2", "LLM2 (Debater B)", "green", classes="participant-panel", id="panel-llm2")

        yield StatusBar(id="status-bar")
        with Horizontal(id="controls"):
            yield Input(placeholder="Send prompt to Judge (control input)...", id="input")
            with Container(id="button-container"):
                yield Button("Reset Bridges", id="reset-button")
        yield Footer()

    def on_mount(self) -> None:
        """Start message processing when app mounts"""
        self.title = "LLM Debate Monitor"
        self.sub_title = "Real-time debate visualization"

        # Set focus to input
        self.query_one(Input).focus()

        # Start processing messages from queue
        self.set_interval(0.1, self.process_messages)

    def _request_bridge_reset(self) -> None:
        """Send a reset command to controller (which forwards to bridges and LLMs)"""
        # Send reset via control output to controller
        dora_node_queue.put({
            "type": "control",
            "content": "reset",
        })

        # Also signal local state reset in event loop
        dora_node_queue.put({
            "type": "reset_local_state",
        })

        message_queue.put({
            "type": "text_chunk",
            "participant": "judge",
            "content": "[System] Reset requested. Controller notified to reset all bridges and LLMs.",
        })
        message_queue.put({"type": "complete", "participant": "judge"})

        status_bar = self.query_one("#status-bar", StatusBar)
        for participant in ["llm1", "judge", "llm2"]:
            status_bar.set_status(participant, "idle")

    def process_messages(self) -> None:
        """Process messages from the queue"""
        while not message_queue.empty():
            msg = message_queue.get()
            msg_type = msg.get("type")

            if msg_type == "text_chunk":
                participant = msg.get("participant")
                content = msg.get("content")
                panel_id = f"panel-{participant}"
                panel = self.query_one(f"#{panel_id}", ParticipantPanel)
                panel.update_current_message(content)

            elif msg_type == "complete":
                participant = msg.get("participant")
                panel_id = f"panel-{participant}"
                panel = self.query_one(f"#{panel_id}", ParticipantPanel)
                panel.mark_complete()

            elif msg_type == "status":
                participant = msg.get("participant")
                status = msg.get("status")
                status_bar = self.query_one("#status-bar", StatusBar)
                status_bar.set_status(participant, status)

            elif msg_type == "conversation_append":
                # Conversation history is not rendered in the Textual UI; ignore.
                continue

    async def on_input_submitted(self, event: Input.Submitted) -> None:
        """Handle user input - send as control message to judge"""
        user_text = event.value.strip()

        if not user_text:
            return

        # Clear input
        event.input.value = ""

        lower_text = user_text.lower()
        if lower_text in {"reset", "/reset"}:
            self._request_bridge_reset()
            return
        # Update status
        status_bar = self.query_one("#status-bar", StatusBar)
        status_bar.set_status("judge", "receiving")

        # Send control message to judge with prompt field
        dora_node_queue.put({
            "type": "control",
            "content": json.dumps({"prompt": user_text})
        })

        # Echo control prompt in judge panel
        message_queue.put({
            "type": "text_chunk",
            "participant": "judge",
            "content": f"[Control] {user_text}"
        })
        message_queue.put({"type": "complete", "participant": "judge"})

    async def on_button_pressed(self, event: Button.Pressed) -> None:
        """Handle button actions"""
        if event.button.id == "reset-button":
            self._request_bridge_reset()

    def action_clear(self) -> None:
        """Clear all panels"""
        for participant in ["llm1", "judge", "llm2"]:
            panel_id = f"panel-{participant}"
            panel = self.query_one(f"#{panel_id}", ParticipantPanel)
            panel.remove_children()
            panel.current_message = None
            panel.current_message_id = 0

    def action_quit(self) -> None:
        """Quit the application"""
        self.exit()


def dora_event_loop(node: Node, stop_signal: threading.Event):
    """Process Dora events in background thread"""
    # Track streaming state for each participant
    streaming_state = {
        "llm1": {"content": "", "active": False},
        "llm2": {"content": "", "active": False},
        "judge": {"content": "", "active": False}
    }

    last_chunk_time = {
        "llm1": 0,
        "llm2": 0,
        "judge": 0
    }
    last_bundle_text = ""

    while not stop_signal.is_set():
        # Check for outgoing messages first (non-blocking)
        while not dora_node_queue.empty():
            msg = dora_node_queue.get()
            msg_type = msg.get("type")
            content = msg.get("content") or ""

            try:
                if msg_type == "control":
                    # Send control message to controller
                    node.send_output("control", pa.array([content]))
                elif msg_type == "reset_local_state":
                    # Reset local streaming state when reset command is sent
                    for participant in ["llm1", "llm2", "judge"]:
                        streaming_state[participant]["content"] = ""
                        streaming_state[participant]["active"] = False
                        last_chunk_time[participant] = 0
                    print("[debate-monitor] Local streaming state reset", file=sys.stderr)
            except Exception as e:
                print(f"Failed to send control message: {e}")

        event = node.next(0.1)

        if event is None:
            # Check for timeouts (no chunks for 2 seconds)
            current_time = time.time()
            for participant in ["llm1", "llm2", "judge"]:
                if streaming_state[participant]["active"]:
                    if last_chunk_time[participant] > 0 and (current_time - last_chunk_time[participant]) > 2.0:
                        final_content = streaming_state[participant]["content"]
                        if final_content:
                            message_queue.put({
                                "type": "text_chunk",
                                "participant": participant,
                                "content": final_content
                            })
                            entry_kind = "moderator" if participant == "judge" else "response"
                            message_queue.put({
                                "type": "conversation_append",
                                "entry": {
                                    "participant": participant,
                                    "content": final_content,
                                    "kind": entry_kind,
                                },
                            })
                        message_queue.put({"type": "complete", "participant": participant})
                        message_queue.put({"type": "status", "participant": participant, "status": "complete"})
                        streaming_state[participant]["content"] = ""
                        streaming_state[participant]["active"] = False
                        last_chunk_time[participant] = 0
            continue

        if event["type"] == "STOP":
            stop_signal.set()
            break

        if event["type"] != "INPUT":
            continue

        try:
            input_id = event["id"]
            value_array = event["value"]

            # Combined conversation stream coming from the bridge
            if input_id in CONVERSATION_INPUT_IDS:
                bundle_text = _extract_first_value(value_array)
                if bundle_text is not None:
                    bundle_str = bundle_text if isinstance(bundle_text, str) else str(bundle_text)
                    if bundle_str.strip() and bundle_str != last_bundle_text:
                        last_bundle_text = bundle_str
                        message_queue.put({
                            "type": "conversation_append",
                            "entry": {
                                "participant": "conversation",
                                "content": bundle_str,
                                "kind": "bundle",
                            },
                        })
                continue

            # Determine participant from input_id
            participant = None
            if "llm1" in input_id:
                participant = "llm1"
            elif "llm2" in input_id:
                participant = "llm2"
            elif "judge" in input_id:
                participant = "judge"

            if not participant:
                continue

            # Prompts sent into LLMs/Judge: display them and add to history
            if input_id.endswith(PROMPT_SUFFIX):
                prompt_text = _extract_first_value(value_array)
                if prompt_text:
                    prompt_str = prompt_text if isinstance(prompt_text, str) else str(prompt_text)
                    message_queue.put({
                        "type": "text_chunk",
                        "participant": participant,
                        "content": f"[Prompt] {prompt_str}",
                    })
                    message_queue.put({"type": "complete", "participant": participant})
                    message_queue.put({"type": "status", "participant": participant, "status": "prompted"})
                    message_queue.put({
                        "type": "conversation_append",
                        "entry": {
                            "participant": participant,
                            "content": prompt_str,
                            "kind": "prompt",
                        },
                    })
                streaming_state[participant]["content"] = ""
                streaming_state[participant]["active"] = False
                last_chunk_time[participant] = 0
                continue

            raw_metadata = event.get("metadata", {}) or {}
            if hasattr(raw_metadata, "parameters"):
                params = getattr(raw_metadata, "parameters")
                if hasattr(params, "items"):
                    metadata = {k: _parameter_to_primitive(v) for k, v in params.items()}
                else:
                    metadata = {}
            elif hasattr(raw_metadata, "items"):
                metadata = {k: _parameter_to_primitive(v) for k, v in raw_metadata.items()}
            elif isinstance(raw_metadata, dict):
                metadata = {k: _parameter_to_primitive(v) for k, v in raw_metadata.items()}
            else:
                metadata = {}

            # Handle text input
            if input_id.endswith("_text"):
                text_chunk = _extract_first_value(value_array)
                stripped_chunk = text_chunk.lstrip()

                # Conference bridge forwards bundled JSON arrays; unpack them into
                # individual participant updates so each panel behaves the same.
                if stripped_chunk.startswith("["):
                    try:
                        bundle = json.loads(text_chunk)
                        if isinstance(bundle, list) and bundle:
                            handled_bundle = False
                            for entry in bundle:
                                if not isinstance(entry, dict):
                                    continue
                                target = entry.get("participant") or participant
                                content = entry.get("content", "")
                                if not content:
                                    continue
                                handled_bundle = True
                                message_queue.put({
                                    "type": "text_chunk",
                                    "participant": target,
                                    "content": content,
                                })
                                if entry.get("complete", True):
                                    message_queue.put({"type": "complete", "participant": target})
                                    message_queue.put({
                                        "type": "status",
                                        "participant": target,
                                        "status": "complete",
                                    })
                                    if target in streaming_state:
                                        streaming_state[target]["content"] = ""
                                        streaming_state[target]["active"] = False
                                        last_chunk_time[target] = 0
                            if handled_bundle:
                                continue
                    except Exception:
                        pass
                session_status_raw = _normalize_metadata_value(metadata, "session_status")
                is_complete_value = _normalize_metadata_value(metadata, "is_complete", False)

                if isinstance(is_complete_value, bool):
                    is_complete = is_complete_value
                elif isinstance(is_complete_value, str):
                    is_complete = is_complete_value.lower() in {"true", "1", "yes"}
                else:
                    is_complete = bool(is_complete_value)

                status_lower = ""
                if isinstance(session_status_raw, str):
                    status_lower = session_status_raw.lower()
                elif session_status_raw is not None:
                    status_lower = str(session_status_raw).lower()

                # Check completion
                if status_lower == "ended" or is_complete:
                    if streaming_state[participant]["content"]:
                        final_content = streaming_state[participant]["content"]
                        message_queue.put({
                            "type": "text_chunk",
                            "participant": participant,
                            "content": final_content
                        })
                        entry_kind = "moderator" if participant == "judge" else "response"
                        message_queue.put({
                            "type": "conversation_append",
                            "entry": {
                                "participant": participant,
                                "content": final_content,
                                "kind": entry_kind,
                            },
                        })
                    message_queue.put({"type": "complete", "participant": participant})
                    message_queue.put({"type": "status", "participant": participant, "status": "complete"})
                    streaming_state[participant]["content"] = ""
                    streaming_state[participant]["active"] = False
                    last_chunk_time[participant] = 0
                    continue

                # Handle text chunk
                if text_chunk:
                    if not streaming_state[participant]["active"]:
                        streaming_state[participant]["active"] = True
                        streaming_state[participant]["content"] = ""
                        message_queue.put({"type": "status", "participant": participant, "status": "streaming"})

                    streaming_state[participant]["content"] += text_chunk
                    last_chunk_time[participant] = time.time()

                    message_queue.put({
                        "type": "text_chunk",
                        "participant": participant,
                        "content": streaming_state[participant]["content"]
                    })

            # Handle status input
            elif input_id.endswith("_status"):
                status_value = _extract_first_value(value_array)
                if status_value:
                    message_queue.put({"type": "status", "participant": participant, "status": status_value})

        except Exception as err:
            print(f"[debate-monitor] Error processing event {event}: {err}")


def headless_monitor_loop(stop_signal: threading.Event):
    """Fallback console monitor for non-TTY environments"""
    console = Console()
    statuses: Dict[str, str] = {"llm1": "idle", "llm2": "idle", "judge": "idle"}
    latest_text: Dict[str, str] = {"llm1": "", "llm2": "", "judge": ""}
    conversation_entries: List[Dict[str, str]] = []

    console.print("[bold]LLM Debate Monitor (headless mode)[/bold]")
    console.print("Streaming updates will appear below.\n")

    while not stop_signal.is_set() or not message_queue.empty():
        try:
            msg = message_queue.get(timeout=0.1)
        except Empty:
            continue

        msg_type = msg.get("type")
        participant = msg.get("participant")

        if msg_type == "status" and participant:
            statuses[participant] = msg.get("status", "")
            console.print(
                f"[dim]{participant.upper()} status[/dim]: {statuses[participant]}",
                highlight=False,
            )
        elif msg_type == "text_chunk" and participant:
            content = msg.get("content", "")
            latest_text[participant] = content
            preview = content if len(content) < 200 else content[:200] + "..."
            console.print(f"[cyan]{participant.upper()}[/cyan]: {preview}", highlight=False)
        elif msg_type == "conversation_append":
            entry = msg.get("entry") or {}
            normalized_entry = {
                "participant": entry.get("participant", ""),
                "kind": entry.get("kind", "message"),
                "content": entry.get("content", ""),
            }
            if not isinstance(normalized_entry["content"], str):
                normalized_entry["content"] = str(normalized_entry["content"])
            if conversation_entries and conversation_entries[-1] == normalized_entry:
                continue
            conversation_entries.append(normalized_entry)

            entry_participant = normalized_entry["participant"]
            entry_kind = normalized_entry["kind"]
            content_text = normalized_entry["content"]
            display_name = PARTICIPANT_DISPLAY_NAMES.get(entry_participant, entry_participant.upper() or "CONVERSATION")

            if entry_participant == "conversation" and entry_kind == "bundle":
                header = "Dialogue bundle"
            elif entry_kind == "prompt":
                header = f"{display_name} prompt"
            elif entry_participant == "judge":
                header = "Judge"
            else:
                header = display_name

            preview = content_text if len(content_text) < 200 else content_text[:200] + "..."
            console.print(f"[blue]{header}[/blue]: {preview}", highlight=False)
        elif msg_type == "complete" and participant:
            console.print(f"[green]{participant.upper()} complete[/green]", highlight=False)

    console.print("\n[dim]Debate monitor stopped[/dim]", highlight=False)


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(description="Dora LLM Debate Monitor")
    parser.add_argument(
        "--node-name",
        "--name",
        dest="node_name",
        default="debate-monitor",
        help="Node identifier to register with Dora (default: debate-monitor)",
    )
    args = parser.parse_args()

    global NODE_NAME, stop_event
    NODE_NAME = args.node_name

    # Initialize Dora node
    node = Node(NODE_NAME)

    headless_mode = (
        not sys.stdout.isatty()
        or os.environ.get("DEBATE_MONITOR_HEADLESS", "").lower() in {"1", "true", "yes"}
    )

    mode_label = "headless" if headless_mode else "textual"
    print(f"[debate-monitor] Starting in {mode_label} mode with node '{NODE_NAME}'", flush=True)

    # Start Dora event loop in background thread
    dora_thread = threading.Thread(
        target=dora_event_loop,
        args=(node, stop_event),
        daemon=True,
    )
    dora_thread.start()

    try:
        if headless_mode:
            headless_monitor_loop(stop_event)
        else:
            app = DebateMonitorApp()
            app.run()
    except KeyboardInterrupt:
        print("\nShutting down debate monitor...")
    finally:
        stop_event.set()
        dora_thread.join(timeout=1.0)
        sys.exit(0)


if __name__ == "__main__":
    main()
