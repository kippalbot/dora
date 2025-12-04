#!/usr/bin/env python3
"""
MAC-AEC Simple Segmentation Wrapper
Wraps the dora-mac-aec node and adds VAD-based segmentation
"""

import os
import sys
import time
import random
import numpy as np
import pyarrow as pa
from dora import Node
from typing import Optional
from collections import deque
from pathlib import Path
import json


def send_log(node, level, message):
    """Send log message through node output"""
    try:
        log_data = {
            "level": level,
            "message": message,
            "timestamp": time.time(),
            "node": "mac-aec"
        }
        node.send_output("log", pa.array([json.dumps(log_data)]))
    except:
        # Fallback to print if node not available
        print(f"[MAC-AEC] [{level}] {message}")


# Add node-hub to path for imports
node_hub_path = Path(__file__).parent / "../../node-hub"
if node_hub_path.exists():
    sys.path.insert(0, str(node_hub_path))

# Try to import from dora-aec (the working version with proper AEC)
dora_aec_path = Path(__file__).parent / "../../node-hub/dora-aec"
if dora_aec_path.exists():
    sys.path.insert(0, str(dora_aec_path))
    from dora_aec.aec_wrapper import AECWrapper
    # Silently use dora-aec
else:
    # Fallback to dora-mac-aec
    mac_aec_path = Path(__file__).parent / "../../node-hub/dora-mac-aec"
    if mac_aec_path.exists():
        sys.path.insert(0, str(mac_aec_path))
    from dora_mac_aec.aec_wrapper import AECWrapper
    from dora_mac_aec.utils.logger import AECLogger
    # Warning logged later when node is available


class MacAECSegmentation:
    """
    Wrapper for MAC-AEC that adds segmentation based on VAD
    """
    
    def __init__(self, node=None):
        """Initialize the MAC-AEC wrapper with segmentation"""
        self.node = node
        self.aec = None
        self.logger = None
        
        # VAD state tracking
        self.is_speaking = False
        self.speech_buffer = []
        self.silence_count = 0
        
        # Thresholds
        self.speech_start_threshold = 3  # Frames of speech to start
        # Speech end threshold: configurable via env var (frames of silence, ~10ms per frame)
        self.speech_end_threshold = int(os.getenv("SPEECH_END_FRAMES", "10"))  # Default 10 frames (~100ms)
        self.min_segment_size = 4800     # Minimum samples (0.3s at 16kHz)
        self.max_segment_size = 160000    # Maximum samples (10s at 16kHz)

        # Question end detection (same as speech-monitor)
        # Read from environment variables
        # IMPORTANT: Total silence = speech_end_threshold (~100ms) + question_end_silence_ms
        self.question_end_silence_ms = float(os.getenv("QUESTION_END_SILENCE_MS", "1000"))
        if self.node:
            send_log(self.node, "INFO", f"🔧 CONFIG: QUESTION_END_SILENCE_MS = {self.question_end_silence_ms}ms (from env or default)")
        self.last_speech_end_time = None      # Track when speech last ended
        self.question_end_sent = False        # Prevent duplicate question_ended signals

        # Question ID tracking - generate new ID only after question_ended
        self.current_question_id = random.randint(100000, 999999)  # 6-digit random ID

        # Audio buffer for segmentation
        self.audio_segment_buffer = []
        self.sample_rate = 16000

        # Debug counter
        self._debug_counter = 0
        
        # Initialize MAC-AEC
        # Note: dora-aec doesn't use AECLogger, just print directly
        
        # Find the native library - check multiple locations
        # 1. First check local lib directory (for standalone distribution)
        lib_path = Path(__file__).parent / "lib/libAudioCapture.dylib"
        
        if not lib_path.exists():
            # 2. Try dora-mac-aec node location
            lib_path = Path(__file__).parent / "../../node-hub/dora-mac-aec/dora_mac_aec/lib/libAudioCapture.dylib"
        
        if not lib_path.exists():
            # 3. Try dora-aec node location
            lib_path = Path(__file__).parent / "../../node-hub/dora-aec/dora_aec/lib/libAudioCapture.dylib"
        
        if not lib_path.exists():
            raise FileNotFoundError(
                f"MAC-AEC native library not found. Please ensure libAudioCapture.dylib is in one of:\n"
                f"  1. {Path(__file__).parent / 'lib/'}\n"
                f"  2. {Path(__file__).parent / '../../node-hub/dora-mac-aec/dora_mac_aec/lib/'}\n"
                f"  3. {Path(__file__).parent / '../../node-hub/dora-aec/dora_aec/lib/'}"
            )
        
        self.aec = AECWrapper(
            library_path=str(lib_path),
            enable_aec=True,
            enable_vad=True,
            sample_rate=self.sample_rate
        )
        if self.node:
            send_log(self.node, "INFO", f"MAC-AEC initialized with library: {lib_path.name}")
        
    def start(self):
        """Start the MAC-AEC capture"""
        try:
            self.aec.start_record()
            
            # Test if we can get audio
            import time
            time.sleep(0.1)
            
            test_data, test_vad = self.aec.get_audio_data()
            if self.node:
                if test_data:
                    send_log(self.node, "INFO", "MAC-AEC started successfully")
                else:
                    send_log(self.node, "WARNING", "MAC-AEC started but no audio data in test")
            
            return True
        except Exception as e:
            if self.node:
                send_log(self.node, "ERROR", f"Failed to start MAC-AEC: {e}")
            raise
        
    def stop(self):
        """Stop the MAC-AEC capture"""
        self.aec.stop_record()
        if self.node:
            send_log(self.node, "INFO", "MAC-AEC stopped")
            
    def process_audio(self, node):
        """Process audio and detect speech segments"""
        try:
            # Collect ALL available audio frames (drain the buffer)
            all_audio = []
            vad_results = []
            
            # Keep getting audio until buffer is empty
            max_iterations = 100  # Safety limit
            iterations = 0
            
            while iterations < max_iterations:
                try:
                    audio_data, vad_result = self.aec.get_audio_data()
                except Exception as e:
                    if self.node:
                        send_log(self.node, "ERROR", f"Error getting audio: {e}")
                    break
                    
                iterations += 1
                
                if audio_data is None:
                    break
                    
                if isinstance(audio_data, bytes) and len(audio_data) > 0:
                    # Convert and collect
                    audio_array = np.frombuffer(audio_data, dtype=np.int16).astype(np.float32) / 32768.0
                    all_audio.append(audio_array)
                    vad_results.append(vad_result)
                elif audio_data is not None:
                    if self.node:
                        send_log(self.node, "WARNING", f"Got non-bytes data: {type(audio_data)}")
                    break

            # CRITICAL: Check question_ended BEFORE early return (timer-based, runs every 10ms)
            question_ended = False
            if not self.is_speaking and self.last_speech_end_time and not self.question_end_sent:
                silence_since_speech_ms = (time.time() - self.last_speech_end_time) * 1000
                if silence_since_speech_ms >= self.question_end_silence_ms:
                    question_ended = True
                    self.question_end_sent = True
                    if self.node:
                        send_log(self.node, "INFO", f"⏱️ QUESTION_ENDED DETECTED - will send question_id={self.current_question_id} with question_ended signal (沉默时长: {silence_since_speech_ms:.0f}ms)")

            # If no audio was available, return with question_ended status
            if len(all_audio) == 0:
                return None, False, False, None, question_ended, self.current_question_id

            # Combine all audio chunks
            audio_array = np.concatenate(all_audio) if len(all_audio) > 1 else all_audio[0]
            vad_result = any(vad_results)  # True if any chunk had voice
            num_chunks = len(all_audio)  # Track number of chunks for accurate silence counting
                
            # Debug counter for internal tracking
            if not hasattr(self, '_debug_counter'):
                self._debug_counter = 0
            self._debug_counter += 1
                
            # Track VAD state changes
            speech_started = False
            speech_ended = False
            audio_segment = None
            # question_ended already set above (before early return check)

            # Update speech state based on VAD
            if vad_result:
                # Speech detected
                if not self.is_speaking:
                    self.silence_count = 0
                    self.speech_buffer.append(audio_array)

                    # Check if we have enough speech to start
                    if len(self.speech_buffer) >= self.speech_start_threshold:
                        self.is_speaking = True
                        speech_started = True
                        # Reset question_end tracking when new speech starts
                        self.question_end_sent = False
                        # TRACE: Log speech start with current question_id
                        if self.node:
                            send_log(self.node, "INFO", f"🎤 NEW SPEECH STARTED - keeping question_id={self.current_question_id}")

                        # Start new segment buffer
                        self.audio_segment_buffer = []
                        for buf in self.speech_buffer:
                            self.audio_segment_buffer.extend(buf)
                else:
                    # Continue adding to segment
                    self.audio_segment_buffer.extend(audio_array)
                    self.silence_count = 0
                    
                    # Check max segment size
                    if len(self.audio_segment_buffer) >= self.max_segment_size:
                        # Force segment end at max size
                        audio_segment = np.array(self.audio_segment_buffer, dtype=np.float32)
                        self.audio_segment_buffer = []
                        self.is_speaking = False
                        speech_ended = True
                        # Max segment size reached - no verbose logging
                        
            else:
                # No speech detected
                if self.is_speaking:
                    # Add to buffer even during silence (for natural endings)
                    self.audio_segment_buffer.extend(audio_array)
                    # Increment by number of chunks collected (not just 1) for accurate timing
                    self.silence_count += num_chunks

                    # Check if silence is long enough to end speech
                    if self.silence_count >= self.speech_end_threshold:
                        # Speech ended - create segment
                        if len(self.audio_segment_buffer) >= self.min_segment_size:
                            audio_segment = np.array(self.audio_segment_buffer, dtype=np.float32)
                            # Speech ended - no verbose logging

                        # Reset state
                        self.audio_segment_buffer = []
                        self.is_speaking = False
                        silence_frames = self.silence_count  # Save for logging
                        self.silence_count = 0
                        self.speech_buffer = []
                        speech_ended = True
                        # Track speech end time for question detection
                        self.last_speech_end_time = time.time()
                        self.question_end_sent = False
                        # TRACE: Log speech end with current question_id
                        if self.node:
                            send_log(self.node, "INFO", f"🔇 SPEECH ENDED - question_id={self.current_question_id}, silence_frames={silence_frames} (~{silence_frames*10}ms), starting question_end timer")
                else:
                    # Not speaking, clear buffers if any
                    if len(self.speech_buffer) > 0:
                        self.speech_buffer = []
                    # question_ended already checked above (before early return, timer-based)

            # Return current question_id (the one that's active/just ended)
            # We'll generate the new question_id AFTER sending the question_ended signal
            return audio_array, speech_started, speech_ended, audio_segment, question_ended, self.current_question_id
            
        except Exception as e:
            if self.node:
                send_log(self.node, "ERROR", f"Error processing audio: {e}")
            return None, False, False, None, False, self.current_question_id


def main():
    """Main node loop"""
    node = Node("mac-aec")
    
    send_log(node, "INFO", "MAC-AEC Simple Segmentation Node starting")
    send_log(node, "INFO", "Wrapping dora-mac-aec with VAD-based segmentation")

    # Initialize wrapper
    mac_aec = MacAECSegmentation(node)

    # Start MAC-AEC
    mac_aec.start()

    speech_end_frames = int(os.getenv("SPEECH_END_FRAMES", "10"))
    question_end_silence_ms = float(os.getenv("QUESTION_END_SILENCE_MS", "3000"))
    speech_end_ms = speech_end_frames * 10  # Approximate ms (assuming ~10ms per frame)
    total_silence_ms = speech_end_ms + question_end_silence_ms
    send_log(node, "INFO", f"Silence detection: speech_end={speech_end_ms}ms ({speech_end_frames} frames) + question_end={question_end_silence_ms}ms = total ~{total_silence_ms}ms")
    send_log(node, "INFO", "Node ready - outputting: audio, is_speaking, speech_started, speech_ended, audio_segment, question_ended")
    
    # State tracking
    frame_count = 0
    last_audio_time = time.time()
    no_audio_count = 0
    
    try:
        while True:
            # Check for events with timeout (short timeout for responsive question_ended detection)
            event = node.next(timeout=0.01)  # 10ms timeout for accurate timing
            
            if event and event["type"] == "INPUT":
                # Handle control commands if needed
                input_id = event.get("id", "")
                if input_id == "control":
                    # Handle control command
                    data = event["value"][0].as_py()
                    send_log(node, "INFO", f"Received control command: {data}")
                
            # Process audio continuously
            current_time = time.time()

            # Process audio every cycle (10ms polling rate)
            if current_time - last_audio_time >= 0.008:  # Trigger every ~10ms
                audio_frame, speech_started, speech_ended, audio_segment, question_ended, question_id = mac_aec.process_audio(node)

                if audio_frame is not None:
                    # Send ALL audio frames for complete recording
                    frame_count += 1
                    # Always send audio for continuous recording
                    audio_pa = pa.array(audio_frame, type=pa.float32())
                    node.send_output("audio", audio_pa)
                    no_audio_count = 0
                else:
                    # Track no audio frames
                    no_audio_count += 1
                    if no_audio_count > 100 and no_audio_count % 100 == 0:
                        send_log(node, "WARNING", f"No audio for {no_audio_count} frames")
                        
                # Send VAD state changes
                if speech_started:
                    node.send_output("speech_started", pa.array([current_time]))
                    node.send_output("is_speaking", pa.array([True]))

                if speech_ended:
                    node.send_output("speech_ended", pa.array([current_time]))
                    node.send_output("is_speaking", pa.array([False]))

                # Send question_ended signal (same as speech-monitor)
                if question_ended:
                    send_log(node, "INFO", f"📤 SENDING question_ended with OLD question_id={question_id}")
                    node.send_output("question_ended", pa.array([current_time]), metadata={"question_id": question_id})
                    # Generate new question_id for next question AFTER sending signal
                    new_qid = random.randint(100000, 999999)
                    mac_aec.current_question_id = new_qid
                    send_log(node, "INFO", f"🆕 GENERATED NEW question_id={new_qid} for NEXT question")

                # Send audio segment when ready
                if audio_segment is not None:
                    # Send as float32 array for ASR compatibility with question_id
                    send_log(node, "INFO", f"🎵 AUDIO_SEGMENT sent with question_id={question_id}")
                    node.send_output("audio_segment", pa.array(audio_segment, type=pa.float32()), metadata={"question_id": question_id})
                    
                last_audio_time = current_time
                
    except KeyboardInterrupt:
        send_log(node, "INFO", "Shutting down...")
    except Exception as e:
        send_log(node, "ERROR", f"Error in main loop: {e}")
        import traceback
        traceback.print_exc()
    finally:
        mac_aec.stop()
        send_log(node, "INFO", "Stopped")


if __name__ == "__main__":
    main()