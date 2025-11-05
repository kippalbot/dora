#!/usr/bin/env python3
"""
Terminal Chat Interface for Dora LLM Client
Uses Textual for rich terminal UI and integrates with Dora dataflow.
"""

import argparse
import sys
import json
import time
import threading
from typing import Optional
from queue import Queue

import pyarrow as pa
from dora import Node
from textual.app import App, ComposeResult
from textual.containers import Container, Vertical, ScrollableContainer
from textual.widgets import Header, Footer, Input, Static, RichLog, Markdown as TextualMarkdown
from textual.binding import Binding
from rich.text import Text
from rich.panel import Panel
from rich.console import Console
from rich.markdown import Markdown as RichMarkdown

# Default node name; updated during startup
NODE_NAME = "chat-terminal"

# Queue for communication between Dora thread and Textual app
message_queue = Queue()
dora_node_queue = Queue()  # For sending messages to Dora


def send_log(node, level, message, config_level="INFO"):
    """Send log message through log output channel."""
    LOG_LEVELS = {"DEBUG": 10, "INFO": 20, "WARNING": 30, "ERROR": 40}

    if LOG_LEVELS.get(level, 0) < LOG_LEVELS.get(config_level, 20):
        return

    formatted_message = f"[{level}] {message}"
    log_data = {
        "node": NODE_NAME,
        "level": level,
        "message": formatted_message,
        "timestamp": time.time()
    }
    node.send_output("log", pa.array([json.dumps(log_data, ensure_ascii=False)]))


class ChatMessage(Static):
    """A single chat message widget"""

    def __init__(self, role: str, content: str, **kwargs):
        # Add CSS class based on role
        if "classes" not in kwargs:
            kwargs["classes"] = role
        else:
            kwargs["classes"] = f"{kwargs['classes']} {role}"

        # Initialize parent first with empty content
        super().__init__("", **kwargs)

        # Then set our attributes
        self.role = role
        self.content = content

        # Now update with the actual renderable
        self.update(self._create_renderable())

    def _normalize_markdown(self, text: str) -> str:
        """Normalize markdown text to ensure proper rendering"""
        import re

        # Ensure headers have newlines before them
        text = re.sub(r'([^\n])(#{1,6} )', r'\1\n\n\2', text)

        # Ensure lists have newlines before them
        text = re.sub(r'([^\n])(\n[-*+] )', r'\1\n\2', text)
        text = re.sub(r'([^\n])(\n\d+\. )', r'\1\n\2', text)

        # Ensure code blocks have newlines
        text = re.sub(r'([^\n])(```)', r'\1\n\n\2', text)

        return text

    def _create_renderable(self):
        """Create the renderable content based on role and content"""
        from rich.console import Group
        from rich import box

        if self.role == "user":
            style = "bold blue on rgb(40,40,80)"  # Blue text on darker blue background
            title_style = "blue"
            title_text = "You"
            # User messages as plain text with right justify
            title = Text(f"{title_text}\n", style=f"bold {title_style}", justify="right")
            content_renderable = Text(self.content, style=style, justify="right")
        elif self.role == "assistant":
            title_style = "green"
            title_text = "Assistant"
            # Assistant messages as Markdown - normalize and render
            title = Text(f"{title_text}\n", style=f"bold {title_style}")
            normalized_content = self._normalize_markdown(self.content)
            content_renderable = RichMarkdown(normalized_content)
        elif self.role == "system":
            style = "bold yellow on rgb(80,80,40)"  # Yellow text on darker yellow background
            title_style = "yellow"
            title_text = "System"
            # System messages as plain text
            title = Text(f"{title_text}\n", style=f"bold {title_style}", justify="center")
            content_renderable = Text(self.content, style=style, justify="center")
        else:
            style = "bold white"
            title_style = "white"
            title_text = self.role.capitalize()
            # Other messages as plain text
            title = Text(f"{title_text}\n", style=f"bold {title_style}")
            content_renderable = Text(self.content, style=style)

        return Group(title, content_renderable)

    def update_content(self, new_content: str):
        """Update the message content and refresh display"""
        self.content = new_content
        self.update(self._create_renderable())


class ChatDisplay(ScrollableContainer):
    """Scrollable container for chat messages"""

    def add_message(self, role: str, content: str, message_id: str = None):
        """Add a new message to the chat display wrapped in a container for alignment"""
        from textual.containers import Container

        # Create a container for alignment
        container = Container(classes=f"message-container {role}")
        if message_id:
            container.id = f"container_{message_id}"

        # Create the message widget
        message = ChatMessage(role, content)
        if message_id:
            message.id = message_id

        # Mount message inside container
        self.mount(container)
        container.mount(message)

        # Auto-scroll to bottom
        self.scroll_end(animate=False)

        return message


class ChatTerminalApp(App):
    """Terminal chat interface using Textual"""

    CSS = """
    Screen {
        background: $surface;
    }

    ChatDisplay {
        height: 1fr;
        border: solid $primary;
        margin: 1;
    }

    Input {
        dock: bottom;
        margin: 0 1;
    }

    #status {
        dock: bottom;
        height: 3;
        border: solid $accent;
        margin: 0 1 1 1;
        padding: 1;
    }

    .message-container {
        width: 100%;
        height: auto;
    }

    .message-container.user {
        align: right top;
    }

    .message-container.assistant {
        align: left top;
    }

    .message-container.system {
        align: center top;
    }

    ChatMessage {
        width: 70%;
        margin: 0 0 2 0;
        padding: 1;
        border: none;
        background: transparent;
    }

    ChatMessage.user {
        width: 70%;
    }

    ChatMessage.assistant {
        width: 70%;
    }
    """

    BINDINGS = [
        Binding("ctrl+c", "quit", "Quit", priority=True),
        Binding("ctrl+l", "clear", "Clear Chat"),
        Binding("ctrl+r", "reset", "Reset Conversation"),
    ]

    def __init__(self):
        super().__init__()
        self.current_assistant_message = ""
        self.is_receiving = False
        self.dora_node: Optional[Node] = None
        self.message_counter = 0  # Track message count for unique IDs
        self.current_message_id = None  # Track current streaming message ID

    def compose(self) -> ComposeResult:
        """Create child widgets for the app"""
        yield Header(show_clock=True)
        yield ChatDisplay(id="chat_display")
        yield Static("Ready. Type your message below.", id="status")
        yield Input(placeholder="Type your message and press Enter...", id="input")
        yield Footer()

    def on_mount(self) -> None:
        """Called when app starts"""
        # Set focus to input
        self.query_one(Input).focus()

        # Display welcome message (system messages don't need unique IDs)
        chat_display = self.query_one(ChatDisplay)
        chat_display.add_message(
            "system",
            "Welcome to Dora LLM Chat!\n\n"
            "Commands:\n"
            "  /help  - Show this help message\n"
            "  /clear - Clear chat history\n"
            "  /reset - Reset conversation\n"
            "  /quit  - Exit application\n"
            "\n"
            "Or use keyboard shortcuts (see footer)"
        )

        # Start monitoring message queue
        self.set_interval(0.1, self.check_message_queue)

    def check_message_queue(self) -> None:
        """Check for messages from Dora thread and update UI"""
        while not message_queue.empty():
            msg = message_queue.get()
            msg_type = msg.get("type")

            if msg_type == "llm_chunk":
                # Streaming LLM response chunk
                chunk = msg.get("content", "")
                self.handle_llm_chunk(chunk)

            elif msg_type == "llm_complete":
                # LLM response complete
                self.handle_llm_complete()

            elif msg_type == "status":
                # Status update
                status_text = msg.get("content", "")
                self.update_status(status_text)

            elif msg_type == "error":
                # Error message
                error_text = msg.get("content", "")
                self.handle_error(error_text)

    def handle_llm_chunk(self, chunk: str) -> None:
        """Handle streaming LLM response chunk"""
        chat_display = self.query_one(ChatDisplay)

        if not self.is_receiving:
            # First chunk - create new assistant message with unique ID
            self.is_receiving = True
            self.message_counter += 1
            self.current_message_id = f"msg_{self.message_counter}"
            self.current_assistant_message = chunk

            # Add message with unique ID
            chat_display.add_message("assistant", chunk, message_id=self.current_message_id)
        else:
            # Append to existing message
            self.current_assistant_message += chunk

            # Update the current streaming message
            try:
                streaming_msg = self.query_one(f"#{self.current_message_id}", ChatMessage)
                # Use the update_content method to refresh with Markdown
                streaming_msg.update_content(self.current_assistant_message)
                chat_display.scroll_end(animate=False)
            except Exception as e:
                # If widget not found, show error for debugging
                import traceback
                self.update_status(f"Update error: {e}")
                traceback.print_exc()

    def handle_llm_complete(self) -> None:
        """Handle LLM response completion"""
        self.is_receiving = False
        self.current_assistant_message = ""
        self.current_message_id = None
        self.update_status("Ready")

    def update_status(self, text: str) -> None:
        """Update status bar"""
        status = self.query_one("#status", Static)
        status.update(text)

    def handle_error(self, error_text: str) -> None:
        """Handle error message"""
        chat_display = self.query_one(ChatDisplay)
        chat_display.add_message("system", f"Error: {error_text}")
        self.update_status("Error occurred")

    async def on_input_submitted(self, event: Input.Submitted) -> None:
        """Handle user input submission"""
        user_text = event.value.strip()

        if not user_text:
            return

        # Clear input
        event.input.value = ""

        # Handle commands
        if user_text.startswith("/"):
            self.handle_command(user_text)
            return

        # Add user message to display
        chat_display = self.query_one(ChatDisplay)
        chat_display.add_message("user", user_text)

        # Update status
        self.update_status("Sending to LLM...")

        # Send to Dora node
        dora_node_queue.put({"type": "text", "content": user_text})

    def handle_command(self, command: str) -> None:
        """Handle special commands"""
        command = command.lower()

        if command == "/quit":
            self.action_quit()

        elif command == "/clear":
            self.action_clear()

        elif command == "/reset":
            self.action_reset()

        elif command == "/help":
            chat_display = self.query_one(ChatDisplay)
            chat_display.add_message(
                "system",
                "Commands:\n"
                "  /help  - Show this help message\n"
                "  /clear - Clear chat history\n"
                "  /reset - Reset conversation\n"
                "  /quit  - Exit application\n"
                "\nKeyboard Shortcuts:\n"
                "  Ctrl+C - Quit\n"
                "  Ctrl+L - Clear chat\n"
                "  Ctrl+R - Reset conversation"
            )

        else:
            chat_display = self.query_one(ChatDisplay)
            chat_display.add_message("system", f"Unknown command: {command}")

    def action_clear(self) -> None:
        """Clear chat display"""
        from textual.containers import Container
        chat_display = self.query_one(ChatDisplay)
        # Remove all message containers
        for container in chat_display.query(".message-container"):
            container.remove()
        self.update_status("Chat cleared")

    def action_reset(self) -> None:
        """Reset conversation (send reset control to LLM)"""
        dora_node_queue.put({"type": "control", "content": "reset"})
        self.action_clear()
        chat_display = self.query_one(ChatDisplay)
        chat_display.add_message("system", "Conversation reset")

    def action_quit(self) -> None:
        """Quit the application"""
        self.exit()


def dora_event_loop(node: Node):
    """Run Dora event loop in separate thread"""
    log_level = "DEBUG"  # Enable debug logging
    last_chunk_time = 0
    chunk_timeout = 2.0  # If no chunks for 2 seconds, consider response complete

    try:
        send_log(node, "INFO", "Chat terminal connected to dataflow", log_level)
        message_queue.put({"type": "status", "content": "Connected to Dora dataflow"})

        while True:
            # Check for outgoing messages first (non-blocking)
            while not dora_node_queue.empty():
                msg = dora_node_queue.get()
                msg_type = msg.get("type")
                content = msg.get("content")

                try:
                    if msg_type == "text":
                        # Send user text to LLM
                        send_log(node, "INFO", f"Sending user message to LLM: '{content}'", log_level)
                        node.send_output("text", pa.array([content]))
                        send_log(node, "INFO", "Message sent successfully", log_level)
                        # Reset streaming state when new message sent
                        last_chunk_time = 0

                    elif msg_type == "control":
                        # Send control command
                        send_log(node, "INFO", f"Sending control command: {content}", log_level)
                        node.send_output("control", pa.array([content]))
                        send_log(node, "INFO", "Control command sent successfully", log_level)

                except Exception as e:
                    send_log(node, "ERROR", f"Failed to send output: {e}", log_level)
                    message_queue.put({"type": "error", "content": f"Failed to send: {e}"})

            # Check for timeout completion (no chunks received for a while)
            import time
            current_time = time.time()
            if last_chunk_time > 0 and (current_time - last_chunk_time) > chunk_timeout:
                send_log(node, "INFO", "LLM response timed out (no more chunks)", log_level)
                message_queue.put({"type": "llm_complete"})
                last_chunk_time = 0  # Reset

            # Now check for incoming events (non-blocking with timeout)
            event = node.next(0.1)  # 100ms timeout

            if event is None:
                # No event received, loop again to check queue
                continue

            if event["type"] == "INPUT":
                input_id = event["id"]
                send_log(node, "DEBUG", f"Received INPUT event: {input_id}", log_level)

                if input_id == "llm_text":
                    # Streaming LLM response
                    text_chunk = event["value"][0].as_py()

                    # Check for completion markers
                    if text_chunk == "" or text_chunk == "[DONE]" or text_chunk.strip() == "":
                        send_log(node, "INFO", "LLM response complete (empty chunk received)", log_level)
                        message_queue.put({"type": "llm_complete"})
                        last_chunk_time = 0
                        continue

                    send_log(node, "DEBUG", f"Received LLM chunk (len={len(text_chunk)}): {text_chunk[:50]}...", log_level)
                    message_queue.put({"type": "llm_chunk", "content": text_chunk})
                    last_chunk_time = current_time  # Update last chunk time

                    # Check metadata for completion
                    metadata = event.get("metadata", {})
                    send_log(node, "DEBUG", f"Metadata: {metadata}", log_level)
                    if metadata.get("is_complete", False) or metadata.get("finished", False):
                        send_log(node, "INFO", "LLM response complete (metadata flag)", log_level)
                        message_queue.put({"type": "llm_complete"})
                        last_chunk_time = 0

                elif input_id == "llm_status":
                    # Status update - check if it indicates completion
                    status = event["value"][0].as_py()
                    send_log(node, "INFO", f"LLM status: {status}", log_level)

                    # Check if status indicates completion
                    status_lower = status.lower() if isinstance(status, str) else ""
                    if "complete" in status_lower or "finished" in status_lower or "done" in status_lower:
                        send_log(node, "INFO", "LLM response complete (status indication)", log_level)
                        message_queue.put({"type": "llm_complete"})
                        last_chunk_time = 0

                    message_queue.put({"type": "status", "content": status})

                elif input_id == "llm_log":
                    # Log message from LLM - just pass through
                    log_msg = event["value"][0].as_py()
                    send_log(node, "DEBUG", f"LLM log: {log_msg}", log_level)

            elif event["type"] == "STOP":
                send_log(node, "INFO", "Received STOP event", log_level)
                break

    except Exception as e:
        import traceback
        error_trace = traceback.format_exc()
        send_log(node, "ERROR", f"Exception in event loop: {e}\n{error_trace}", log_level)
        message_queue.put({"type": "error", "content": str(e)})


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(description="Dora LLM chat terminal")
    parser.add_argument(
        "--node-name",
        "--name",
        dest="node_name",
        default="chat-terminal",
        help="Node identifier to register with Dora (default: chat-terminal)",
    )
    args = parser.parse_args()

    global NODE_NAME
    NODE_NAME = args.node_name

    # Initialize Dora node with the node ID from dataflow.yml
    # The node ID must match the "id" field in dataflow.yml
    node = Node(NODE_NAME)

    # Start Dora event loop in separate thread
    dora_thread = threading.Thread(target=dora_event_loop, args=(node,), daemon=True)
    dora_thread.start()

    # Run Textual app
    app = ChatTerminalApp()
    app.dora_node = node
    app.run()


if __name__ == "__main__":
    main()
