#!/usr/bin/env python3
"""
Multi-Input Circular Buffer Audio Player with Backpressure Control.
Accepts 3 audio inputs (student1, student2, tutor) and concatenates them into one stream.
Sends buffer fullness percentage for external flow control.
"""

import argparse
import os
import time
import threading
import signal
import sys
import numpy as np
import pyarrow as pa
from dora import Node
import sounddevice as sd


class CircularAudioBuffer:
    """Thread-safe circular buffer for audio streaming."""

    def __init__(self, size_seconds=360, sample_rate=32000):
        self.sample_rate = sample_rate
        self.buffer_size = int(size_seconds * sample_rate)
        self.buffer = np.zeros(self.buffer_size, dtype=np.float32)

        self.write_pos = 0
        self.read_pos = 0
        self.available_samples = 0

        self.lock = threading.Lock()

        self.total_written = 0
        self.total_read = 0
        self.underruns = 0
        self.overruns = 0

    def write(self, audio_data: np.ndarray, participant: str = None) -> int:
        """Write audio data to the circular buffer. Concatenates in arrival order."""
        with self.lock:
            data_len = len(audio_data)

            if self.available_samples + data_len > self.buffer_size:
                self.overruns += 1
                overflow = (self.available_samples + data_len) - self.buffer_size
                self.read_pos = (self.read_pos + overflow) % self.buffer_size
                self.available_samples -= overflow
                print(f"[Buffer] Overrun! Skipping {overflow} samples (from {participant})")

            samples_written = 0
            while samples_written < data_len:
                chunk_size = min(data_len - samples_written, self.buffer_size - self.write_pos)
                end_pos = self.write_pos + chunk_size
                self.buffer[self.write_pos:end_pos] = audio_data[samples_written:samples_written + chunk_size]
                self.write_pos = end_pos % self.buffer_size
                samples_written += chunk_size

            self.available_samples = min(self.available_samples + data_len, self.buffer_size)
            self.total_written += data_len
            return data_len

    def read(self, num_samples: int) -> np.ndarray:
        """Read samples from the circular buffer for playback."""
        with self.lock:
            if self.available_samples < num_samples:
                # Partial read - return what we have plus zeros
                actual_samples = self.available_samples
                output = np.zeros(num_samples, dtype=np.float32)

                if actual_samples > 0:
                    samples_read = 0
                    while samples_read < actual_samples:
                        chunk_size = min(actual_samples - samples_read, self.buffer_size - self.read_pos)
                        end_pos = self.read_pos + chunk_size
                        output[samples_read:samples_read + chunk_size] = self.buffer[self.read_pos:end_pos]
                        self.read_pos = end_pos % self.buffer_size
                        samples_read += chunk_size

                    self.available_samples = 0
                    self.total_read += actual_samples

                if actual_samples < num_samples:
                    self.underruns += 1

                return output
            else:
                # Full read
                output = np.zeros(num_samples, dtype=np.float32)
                samples_read = 0

                while samples_read < num_samples:
                    chunk_size = min(num_samples - samples_read, self.buffer_size - self.read_pos)
                    end_pos = self.read_pos + chunk_size
                    output[samples_read:samples_read + chunk_size] = self.buffer[self.read_pos:end_pos]
                    self.read_pos = end_pos % self.buffer_size
                    samples_read += chunk_size

                self.available_samples -= num_samples
                self.total_read += num_samples
                return output

    def get_stats(self):
        """Get buffer statistics."""
        with self.lock:
            buffer_fill_percentage = (self.available_samples / self.buffer_size) * 100 if self.buffer_size > 0 else 0
            available_seconds = self.available_samples / self.sample_rate if self.sample_rate > 0 else 0

            return {
                'available_samples': self.available_samples,
                'available_seconds': available_seconds,
                'buffer_size': self.buffer_size,
                'buffer_fill': buffer_fill_percentage,
                'underruns': self.underruns,
                'overruns': self.overruns,
                'total_written': self.total_written,
                'total_read': self.total_read,
            }

    def reset(self):
        """Reset buffer state."""
        with self.lock:
            self.write_pos = 0
            self.read_pos = 0
            self.available_samples = 0
            self.total_written = 0
            self.total_read = 0


class CircularBufferAudioPlayer:
    """Audio player with circular buffer and playback control."""

    def __init__(self, buffer_seconds=360, sample_rate=32000, blocksize=2048):
        self.buffer = CircularAudioBuffer(size_seconds=buffer_seconds, sample_rate=sample_rate)
        self.sample_rate = sample_rate
        self.blocksize = blocksize
        self.stream = None
        self.is_playing = False

    def audio_callback(self, outdata, frames, time_info, status):
        """Callback for sounddevice stream."""
        if status:
            print(f"[Audio Callback] Status: {status}")

        data = self.buffer.read(frames)
        outdata[:, 0] = data

    def start(self):
        """Start the audio stream."""
        if self.stream is None:
            self.stream = sd.OutputStream(
                samplerate=self.sample_rate,
                channels=1,
                dtype='float32',
                blocksize=self.blocksize,
                callback=self.audio_callback
            )
            self.stream.start()

    def set_sample_rate(self, new_rate):
        """Update sample rate if changed."""
        if new_rate != self.sample_rate:
            self.sample_rate = new_rate
            self.buffer.sample_rate = new_rate
            print(f"[Audio Player] Sample rate updated to {new_rate} Hz")

    def add_audio(self, audio_data, participant=None):
        """Add audio data to buffer. Concatenates from all participants."""
        self.buffer.write(audio_data, participant)

    def pause(self):
        """Pause playback."""
        self.is_playing = False

    def resume(self):
        """Resume playback."""
        self.is_playing = True

    def reset(self):
        """Reset the audio buffer."""
        self.pause()
        self.buffer.reset()
        print("[Audio Player] Buffer reset to empty")


shutdown_flag = threading.Event()

def signal_handler(signum, frame):
    print("\n[Multi-Audio Player] Shutting down...")
    shutdown_flag.set()


def main():
    # Read buffer size from environment variable with fallback
    default_buffer_seconds = int(os.getenv("BUFFER_SECONDS", "360"))

    parser = argparse.ArgumentParser(description="Multi-input circular buffer audio player")
    parser.add_argument("--sample-rate", type=int, default=32000,
                        help="Initial playback sample rate (Hz)")
    parser.add_argument("--buffer-seconds", type=int, default=default_buffer_seconds,
                        help="Buffer capacity in seconds")
    parser.add_argument("--blocksize", type=int, default=2048,
                        help="Audio callback blocksize")
    args = parser.parse_args()

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    try:
        node = Node("audio-player")
        player = CircularBufferAudioPlayer(
            buffer_seconds=args.buffer_seconds,
            sample_rate=args.sample_rate,
            blocksize=args.blocksize,
        )
        player.start()

        # Clear screen
        print("\033[2J\033[H", end="")

        print("=" * 60)
        print("MULTI-INPUT CIRCULAR BUFFER AUDIO PLAYER")
        print("=" * 60)
        print(f"Buffer: {args.buffer_seconds} seconds")
        print(f"Inputs: student1, student2, tutor")
        print("Audio streams concatenated in arrival order (FIFO)")
        print("Outputs buffer percentage for backpressure control")
        print("=" * 60 + "\n")

        # State
        playback_started = False

        # Stats per participant
        segments_per_participant = {"student1": 0, "student2": 0, "tutor": 0}

        # Timing
        last_status_time = time.time()
        status_interval = 1.0  # Send status every second

        # Get configurable timeout from environment
        node_timeout_ms = int(os.getenv("NODE_TIMEOUT_MS", "1000"))
        node_timeout = node_timeout_ms / 1000.0

        while not shutdown_flag.is_set():
            # Process events with timeout
            try:
                event = node.next(timeout=node_timeout)
            except KeyboardInterrupt:
                break
            except Exception:
                event = None

            # Handle 3 audio inputs: audio_student1, audio_student2, audio_tutor
            if event and event["type"] == "INPUT" and event["id"] in ["audio_student1", "audio_student2", "audio_tutor"]:
                try:
                    participant = event["id"].replace("audio_", "")  # Extract: student1, student2, tutor
                    raw_value = event.get("value")

                    if raw_value and len(raw_value) > 0:
                        audio_data = raw_value[0].as_py()
                        if audio_data is not None:
                            if not isinstance(audio_data, np.ndarray):
                                audio_data = np.array(audio_data, dtype=np.float32)

                            if len(audio_data) > 0:
                                metadata = event.get("metadata", {})

                                # Update sample rate if provided
                                incoming_rate = metadata.get("sample_rate")
                                if incoming_rate is not None:
                                    player.set_sample_rate(incoming_rate)

                                # Calculate duration
                                effective_rate = float(player.sample_rate)
                                duration = len(audio_data) / effective_rate if effective_rate > 0 else 0.0

                                # Add audio to buffer (concatenates in arrival order)
                                player.add_audio(audio_data, participant)

                                segments_per_participant[participant] += 1
                                segment_index = metadata.get("segment_index", -1)

                                print(f"[Audio Player] 🎵 {participant.upper()}: "
                                      f"segment {segment_index + 1}, "
                                      f"{len(audio_data)} samples, "
                                      f"{duration:.3f}s", flush=True)

                                # Auto-start playback
                                if not playback_started:
                                    player.resume()
                                    playback_started = True
                                    print(f"[Audio Player] ▶️  Playback STARTED")

                except Exception as e:
                    print(f"[Error] Processing audio from {event['id']}: {e}")

            # Send buffer status periodically
            current_time = time.time()
            if current_time - last_status_time >= status_interval:
                stats = player.buffer.get_stats()
                buffer_percentage = stats['buffer_fill']

                # Send buffer percentage to controller
                node.send_output("buffer_status",
                    pa.array([buffer_percentage], type=pa.float64()))

                # Print status
                print(f"\r[Buffer] {stats['available_seconds']:.1f}s ({buffer_percentage:.1f}%) | "
                      f"Student1: {segments_per_participant['student1']}, "
                      f"Student2: {segments_per_participant['student2']}, "
                      f"Tutor: {segments_per_participant['tutor']}",
                      end="", flush=True)

                last_status_time = current_time

    except Exception as e:
        print(f"\n[Error] Main loop: {e}")
        import traceback
        traceback.print_exc()
    finally:
        if player.stream:
            player.stream.stop()
            player.stream.close()
        print("\n[Multi-Audio Player] Shutdown complete")


if __name__ == "__main__":
    main()
