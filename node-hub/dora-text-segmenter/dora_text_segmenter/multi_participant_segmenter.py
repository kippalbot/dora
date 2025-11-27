#!/usr/bin/env python3
"""
Multi-Participant Queue-based Text Segmenter with Session-Based FIFO

New Architecture:
1. Dynamic participant discovery from YAML input ports
2. Per-participant queues with session boundaries
3. Active queue selection based on oldest session timestamp
4. Decoupled receiving (enqueue) and sending (dequeue) logic
5. TTS_COMPLETE driven sending with session completion tracking
"""

import os
import time
import re
import json
import uuid
from typing import Optional, List, Dict
from dataclasses import dataclass, field
import pyarrow as pa
from dora import Node
from collections import deque


def send_log(node, level, message, config_level="INFO"):
    """Send log message through log output channel."""
    LOG_LEVELS = {"DEBUG": 10, "INFO": 20, "WARNING": 30, "ERROR": 40}

    if LOG_LEVELS.get(level, 0) < LOG_LEVELS.get(config_level, 20):
        return

    formatted_message = f"[{level}] {message}"
    log_data = {
        "node": "multi-text-segmenter",
        "level": level,
        "message": message
    }
    node.send_output("log", pa.array([json.dumps(log_data)]))
    print(formatted_message, flush=True)


def parse_int_env(name, default):
    """Parse integer from environment variable."""
    try:
        return int(os.getenv(name, str(default)))
    except ValueError:
        return default


def detect_session_event(metadata):
    """Detect session lifecycle event from metadata."""
    session_status = metadata.get("session_status")
    if session_status == "started":
        return "SESSION_START"
    elif session_status == "ended":
        return "SESSION_END"
    else:
        return "SESSION_CHUNK"


def remove_speaker_id(text, node, log_level):
    """Remove [Speaker Name] prefix from text."""
    pattern = r'^\[([^\]]+)\]\s*'
    match = re.match(pattern, text)
    if match:
        speaker = match.group(1)
        cleaned = re.sub(pattern, '', text)
        send_log(node, "DEBUG", f"Removed speaker ID [{speaker}], cleaned: '{cleaned}'", log_level)
        return cleaned
    return text


def should_skip_segment(text, punctuation_marks, node, log_level):
    """Check if segment should be skipped (empty or only punctuation)."""
    if not text.strip():
        return True
    if all(c in punctuation_marks or c.isspace() for c in text):
        send_log(node, "DEBUG", f"Skipping punctuation-only segment: '{text}'", log_level)
        return True
    return False


def segment_by_punctuation(text, min_length, max_length, punctuation_marks, node, log_level):
    """
    Segment text by punctuation marks, respecting MAX_SEGMENT_LENGTH when possible.

    Logic:
    - Find all punctuation marks in the text
    - If a segment is <= MAX_SEGMENT_LENGTH, keep it as-is
    - If a segment is > MAX_SEGMENT_LENGTH, split it at intermediate punctuation marks
    - Never split mid-sentence (always split at punctuation boundaries)

    Returns: (complete_segments, incomplete_text, keep_incomplete)
    """
    if not text:
        return [], "", False

    escaped_punctuation = re.escape(punctuation_marks)
    pattern = f'[^{escaped_punctuation}]+[{escaped_punctuation}]'

    segments = []
    last_end = 0
    accumulator = ""

    for match in re.finditer(pattern, text):
        segment_text = match.group().strip()
        if not segment_text:
            continue

        # Add to accumulator
        if accumulator:
            combined = accumulator + segment_text
        else:
            combined = segment_text

        # Check if we should flush the accumulator
        if max_length > 0 and len(combined) > max_length:
            # Combined segment is too long
            # Flush the accumulator (if not empty) as a separate segment
            if accumulator:
                segments.append(accumulator)
                send_log(node, "DEBUG",
                    f"Segmentation: Flushed segment at max_length: '{accumulator}' (len={len(accumulator)})",
                    log_level)
                accumulator = segment_text  # Start new accumulator with current segment
            else:
                # Current segment alone is longer than max_length
                # Send it anyway (can't split mid-sentence)
                segments.append(segment_text)
                send_log(node, "DEBUG",
                    f"Segmentation: Segment exceeds max_length: '{segment_text}' (len={len(segment_text)})",
                    log_level)
                accumulator = ""
        else:
            # Combined segment is within limit, keep accumulating
            accumulator = combined

        last_end = match.end()

    # Flush any remaining accumulator
    if accumulator:
        segments.append(accumulator)
        send_log(node, "DEBUG",
            f"Segmentation: Final segment: '{accumulator}' (len={len(accumulator)})",
            log_level)

    # Anything left over is incomplete (no ending punctuation)
    incomplete = text[last_end:].strip()

    if incomplete:
        send_log(node, "DEBUG",
            f"Segmentation: Incomplete text buffered: '{incomplete}'",
            log_level)

    return segments, incomplete, bool(incomplete)


def is_participant_port(event_id):
    """Check if event_id is a participant input port (not control or TTS)."""
    CONTROL_PORTS = {"control", "reset"}
    if event_id in CONTROL_PORTS:
        return False
    if event_id.startswith("tts_complete_"):
        return False
    return True


def select_oldest_session_queue(participant_names, session_timestamps, segment_queues):
    """
    Find participant queue with oldest session timestamp.
    Only considers queues that have both session timestamp AND segments.
    """
    candidates = []

    for participant in participant_names:
        if session_timestamps[participant] and segment_queues[participant]:
            oldest_ts = session_timestamps[participant][0]["timestamp"]
            candidates.append((participant, oldest_ts))

    if not candidates:
        return None

    # Sort by timestamp, return oldest
    candidates.sort(key=lambda x: x[1])
    return candidates[0][0]


def complete_session_and_activate_next(completed_participant, node, participant_names, session_timestamps, segment_queues, active_queue_ref, is_sending, kick_start_sending, log_level):
    """Complete session and activate next session - called after TTS complete of last chunk"""
    send_log(node, "INFO", f"🏁 COMPLETING SESSION: {completed_participant}", log_level)

    # Session complete - remove from timestamp queue
    if session_timestamps[completed_participant]:
        completed_session = session_timestamps[completed_participant].popleft()
        send_log(node, "INFO",
            f"✅ SESSION COMPLETE: {completed_participant}, session_id={completed_session['session_id']}",
            log_level)

    # Deactivate current and select next
    active_queue_ref[0] = None

    # Debug: Log state of all participants before selecting next queue
    send_log(node, "INFO", f"🔍 Selecting next queue. State:", log_level)
    for p in participant_names:
        has_session = len(session_timestamps[p]) > 0
        has_segments = len(segment_queues[p]) > 0
        if has_session:
            oldest_ts = session_timestamps[p][0]["timestamp"]
            send_log(node, "INFO", f"  {p}: sessions={len(session_timestamps[p])}, segments={len(segment_queues[p])}, oldest_ts={oldest_ts:.3f}", log_level)
        else:
            send_log(node, "INFO", f"  {p}: sessions=0, segments={len(segment_queues[p])}", log_level)

    # Find next oldest session (might be same participant's next session, or different participant)
    next_queue = select_oldest_session_queue(participant_names, session_timestamps, segment_queues)
    if next_queue:
        active_queue_ref[0] = next_queue
        send_log(node, "INFO", f"🎯 ACTIVATED NEXT QUEUE: {active_queue_ref[0]}", log_level)
        kick_start_sending(next_queue)
    else:
        send_log(node, "DEBUG", "No more queues with sessions, idle", log_level)


def main():
    print("[STARTUP] multi_participant_segmenter.py main() called", flush=True)
    node = Node()
    print("[STARTUP] Node created", flush=True)

    # Configuration
    min_segment_length = max(1, parse_int_env("MIN_SEGMENT_LENGTH", 5))
    max_segment_length = parse_int_env("MAX_SEGMENT_LENGTH", 15)
    punctuation_marks = os.getenv("PUNCTUATION_MARKS", "。！？.!?，,、；：""''（）【】《》")
    log_level = os.getenv("LOG_LEVEL", "INFO")
    segment_mode = os.getenv("SEGMENT_MODE", "sentence").lower()
    remove_speaker_id_enabled = os.getenv("REMOVE_SPEAKER_ID", "true").lower() in {"1", "true", "yes"}

    send_log(
        node,
        "INFO",
        f"Multi-Participant Segmenter configured — mode: {segment_mode}, "
        f"min: {min_segment_length}, max: {max_segment_length}, "
        f"punctuation: '{punctuation_marks}', remove_speaker_id: {remove_speaker_id_enabled}",
        log_level,
    )

    # Dynamically discovered participants
    participant_names = []

    # Per-participant data structures (initialized on-demand)
    segment_queues = {}        # participant -> deque([{text, session_id, is_session_end}, ...])
    text_buffers = {}          # participant -> str (incomplete text)
    session_timestamps = {}    # participant -> deque([{session_id, timestamp}, ...])
    current_session = {}       # participant -> session_id (currently receiving)
    is_sending = {}            # participant -> bool (TTS busy flag, for kick-start only)
    last_session_end_sent = {} # participant -> bool (track if last chunk of session was sent)

    # Global state
    active_queue = None        # Which participant's queue is currently sending (only ONE)

    def ensure_participant_initialized(participant):
        """Initialize data structures for a newly discovered participant."""
        if participant not in participant_names:
            participant_names.append(participant)
            segment_queues[participant] = deque()
            text_buffers[participant] = ""
            session_timestamps[participant] = deque()
            current_session[participant] = None
            is_sending[participant] = False
            last_session_end_sent[participant] = False
            send_log(node, "INFO", f"Discovered participant: {participant}", log_level)

    def kick_start_sending(participant):
        """Mark queue as ready to send. First segment will be sent by simulated TTS_COMPLETE."""
        if not segment_queues[participant]:
            return

        send_log(node, "DEBUG",
            f"🚀 KICK-START {participant}: Queue activated with {len(segment_queues[participant])} segments",
            log_level)

        # Mark as not sending so the immediate "simulated" TTS_COMPLETE can trigger first send
        is_sending[participant] = False

        # Immediately trigger first send by simulating TTS_COMPLETE logic
        segment = segment_queues[participant].popleft()
        output_port = f"text_segment_{participant}"

        send_log(node, "INFO",
            f"🎤 SENDING to {participant}: '{segment['text']}' "
            f"(session_id={segment['session_id']}, is_end={segment['is_session_end']}, "
            f"queue_remaining={len(segment_queues[participant])})",
            log_level)

        node.send_output(
            output_port,
            pa.array([segment["text"]]),
            metadata={"session_id": segment["session_id"]}
        )
        is_sending[participant] = True

    def try_activate_queue():
        """If no active queue, select oldest and activate."""
        nonlocal active_queue

        if active_queue is not None:
            return  # Already have active queue

        next_queue = select_oldest_session_queue(participant_names, session_timestamps, segment_queues)
        if next_queue:
            active_queue = next_queue
            send_log(node, "INFO", f"🎯 ACTIVATED QUEUE: {active_queue}", log_level)
            kick_start_sending(next_queue)

    send_log(node, "INFO", "Multi-Participant Text Segmenter started (session-based FIFO)", log_level)
    print("[STARTUP] Entering event loop", flush=True)

    for event in node:
        event_id = event["id"]

        # ==================== RECEIVING SIDE: Participant Input Events ====================
        if is_participant_port(event_id):
            participant = event_id
            ensure_participant_initialized(participant)

            text = event["value"][0].as_py() if event.get("value") else ""
            metadata = event.get("metadata", {})
            session_event = detect_session_event(metadata)

            if session_event == "SESSION_START":
                # New session starting
                session_id = str(uuid.uuid4())
                timestamp = time.time()

                session_timestamps[participant].append({
                    "session_id": session_id,
                    "timestamp": timestamp
                })
                current_session[participant] = session_id

                send_log(node, "INFO",
                    f"📥 SESSION_START: {participant}, session_id={session_id}, ts={timestamp:.3f}",
                    log_level)

                # FIX: Process the text content that comes with SESSION_START
                # The first chunk with session_status="started" contains actual text that must be processed
                if remove_speaker_id_enabled:
                    text = remove_speaker_id(text, node, log_level)

                send_log(node, "INFO",
                    f"📥 FIRST CHUNK from {participant}: '{text}' (len={len(text)})",
                    log_level)

                # Process this first chunk through the same pipeline as SESSION_CHUNK
                combined_text = text_buffers[participant] + text

                # Segment by punctuation
                complete_segments, incomplete_text, keep_incomplete = segment_by_punctuation(
                    combined_text,
                    min_segment_length,
                    max_segment_length,
                    punctuation_marks,
                    node,
                    log_level
                )

                # Update text buffer
                text_buffers[participant] = incomplete_text if keep_incomplete else ""

                # Enqueue segments from the first chunk
                for i, segment_text in enumerate(complete_segments):
                    if not should_skip_segment(segment_text, punctuation_marks, node, log_level):
                        segment_queues[participant].append({
                            "text": segment_text,
                            "session_id": current_session[participant],
                            "is_session_end": False
                        })
                        send_log(node, "INFO",
                            f"📝 ENQUEUED FIRST segment for {participant}: '{segment_text}' (queue_size: {len(segment_queues[participant])})",
                            log_level)

                # Try to activate queue if idle
                try_activate_queue()

            elif session_event == "SESSION_CHUNK":
                # Process text chunk
                if current_session[participant] is None:
                    send_log(node, "WARNING",
                        f"Received chunk for {participant} but no current session", log_level)
                    continue

                # Apply speaker ID removal
                if remove_speaker_id_enabled:
                    text = remove_speaker_id(text, node, log_level)

                send_log(node, "INFO",
                    f"📥 CHUNK from {participant}: '{text}' (len={len(text)})",
                    log_level)

                # Combine with text buffer
                combined_text = text_buffers[participant] + text

                # Segment by punctuation
                complete_segments, incomplete_text, keep_incomplete = segment_by_punctuation(
                    combined_text,
                    min_segment_length,
                    max_segment_length,
                    punctuation_marks,
                    node,
                    log_level
                )

                # Update text buffer
                text_buffers[participant] = incomplete_text if keep_incomplete else ""

                # Enqueue segments
                for i, segment_text in enumerate(complete_segments):
                    if not should_skip_segment(segment_text, punctuation_marks, node, log_level):
                        segment_queues[participant].append({
                            "text": segment_text,
                            "session_id": current_session[participant],
                            "is_session_end": False
                        })
                        send_log(node, "INFO",
                            f"📝 ENQUEUED segment for {participant}: '{segment_text}' (queue_size: {len(segment_queues[participant])})",
                            log_level)

                # Try to activate queue if idle
                try_activate_queue()

            elif session_event == "SESSION_END":
                # Session ended - flush buffer
                if current_session[participant] is None:
                    send_log(node, "WARNING",
                        f"Received SESSION_END for {participant} but no current session", log_level)
                    continue

                send_log(node, "INFO",
                    f"🏁 SESSION_END: {participant}, session_id={current_session[participant]}",
                    log_level)

                # Flush incomplete buffer as final segment
                if text_buffers[participant].strip():
                    incomplete_text = text_buffers[participant].strip()
                    if not should_skip_segment(incomplete_text, punctuation_marks, node, log_level):
                        segment_queues[participant].append({
                            "text": incomplete_text,
                            "session_id": current_session[participant],
                            "is_session_end": True  # Mark as session end
                        })
                        send_log(node, "DEBUG",
                            f"🔥 Flushed buffer as final segment: '{incomplete_text}'", log_level)
                    text_buffers[participant] = ""
                else:
                    # No buffer to flush, mark last segment as session end
                    if segment_queues[participant]:
                        segment_queues[participant][-1]["is_session_end"] = True

                current_session[participant] = None

                # Try to activate queue
                try_activate_queue()

        # ==================== SENDING SIDE: TTS Complete Events ====================
        elif event_id.startswith("tts_complete_"):
            participant = event_id.replace("tts_complete_", "")
            is_sending[participant] = False

            send_log(node, "DEBUG", f"✅ TTS_COMPLETE from {participant}", log_level)

            # FIX: Check if this TTS complete is for the last chunk of a session that needs activation
            if last_session_end_sent[participant] and active_queue == participant:
                # This is the TTS complete for the last chunk of active session - time to activate next!
                send_log(node, "INFO",
                    f"🏁 TTS COMPLETE for LAST CHUNK: {participant}, activating next session",
                    log_level)

                # Complete the session and activate next
                active_queue_ref = [active_queue]
                complete_session_and_activate_next(participant, node, participant_names, session_timestamps, segment_queues, active_queue_ref, is_sending, kick_start_sending, log_level)
                active_queue = active_queue_ref[0]
                last_session_end_sent[participant] = False
                continue  # Skip the normal TTS complete processing

            # Only process if this participant's queue is active
            if active_queue != participant:
                send_log(node, "DEBUG",
                    f"TTS_COMPLETE from {participant} but active_queue={active_queue}, ignoring",
                    log_level)
                continue

            # Active queue - continue draining
            if not segment_queues[participant]:
                send_log(node, "DEBUG",
                    f"Active queue {participant} is empty, waiting for more chunks", log_level)
                continue

            # Dequeue next segment
            segment = segment_queues[participant].popleft()
            output_port = f"text_segment_{participant}"

            send_log(node, "INFO",
                f"🎤 SENDING to {participant}: '{segment['text']}' "
                f"(session_id={segment['session_id']}, is_end={segment['is_session_end']}, "
                f"queue_remaining={len(segment_queues[participant])})",
                log_level)

            node.send_output(
                output_port,
                pa.array([segment["text"]]),
                metadata={"session_id": segment["session_id"]}
            )
            is_sending[participant] = True

            # Check if this was the last segment of a session
            if segment["is_session_end"]:
                # Mark that the last chunk of this session has been sent
                last_session_end_sent[participant] = True
                send_log(node, "INFO",
                    f"📤 LAST CHUNK SENT: {participant}, waiting for TTS complete to activate next session",
                    log_level)
            else:
                # Not last chunk - continue draining queue normally
                send_log(node, "DEBUG",
                    f"🔄 CONTINUING DRAIN: {participant}, more segments remaining",
                    log_level)

        # ==================== CONTROL EVENTS ====================
        elif event_id in ["control", "reset"]:
            command = event["value"][0].as_py() if event.get("value") else None

            if command in ["reset", "cancel"]:
                send_log(node, "INFO", f"🔄 {command.upper()} - Clearing all queues", log_level)

                # Clear all queues and state
                for participant in participant_names:
                    segment_queues[participant].clear()
                    text_buffers[participant] = ""
                    session_timestamps[participant].clear()
                    current_session[participant] = None
                    is_sending[participant] = False
                    last_session_end_sent[participant] = False

                active_queue = None


if __name__ == "__main__":
    main()
