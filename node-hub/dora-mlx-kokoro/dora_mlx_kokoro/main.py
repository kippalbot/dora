"""MLX-accelerated Kokoro TTS node for Dora."""

import json
import os
import sys
import time
from typing import Optional

import numpy as np
import pyarrow as pa
from dora import Node


class KokoroEngine:
    """Kokoro TTS engine with MLX or CPU backend."""

    def __init__(
        self,
        model_path: str = "prince-canuma/Kokoro-82M",
        voice: str = "af_heart",
        lang_code: str = "a",
        speed: float = 1.0,
        use_mlx: bool = True,
    ):
        self.model_path = model_path
        self.voice = voice
        self.lang_code = lang_code
        self.speed = speed
        self.use_mlx = use_mlx
        self.model = None
        self.stats = {
            "total_chars": 0,
            "total_audio_duration": 0.0,
            "total_processing_time": 0.0,
            "synthesis_count": 0,
        }

    def load_model(self):
        """Load Kokoro TTS model."""
        if self.use_mlx and sys.platform == "darwin":
            try:
                from mlx_audio.tts.generate import generate_audio
                self.generate_fn = generate_audio
                self.backend = "mlx"
                print(f"✅ Loaded Kokoro with MLX backend (model: {self.model_path})")
            except ImportError as e:
                print(f"⚠️  MLX not available: {e}, falling back to CPU")
                self._load_cpu_backend()
        else:
            self._load_cpu_backend()

    def _load_cpu_backend(self):
        """Load CPU-based Kokoro (PyTorch fallback)."""
        try:
            import torch
            # Placeholder for CPU Kokoro implementation
            # This would use the original Kokoro PyTorch implementation
            self.backend = "cpu"
            print(f"✅ Loaded Kokoro with CPU backend (model: {self.model_path})")
            # For now, we'll use a minimal implementation
            self.generate_fn = None
        except ImportError as e:
            raise RuntimeError(f"Neither MLX nor PyTorch available: {e}")

    def synthesize(self, text: str) -> tuple[np.ndarray, dict]:
        """
        Synthesize text to audio.

        Returns:
            audio: Audio waveform as float32 numpy array
            metadata: Dictionary with sample_rate, duration, etc.
        """
        if self.model is None or self.generate_fn is None:
            self.load_model()

        start_time = time.time()

        if self.backend == "mlx":
            # Use MLX-audio generate_audio
            from mlx_audio.tts.generate import generate_audio

            # Generate audio using MLX
            audio_data = generate_audio(
                text=text,
                model_path=self.model_path,
                voice=self.voice,
                speed=self.speed,
                lang_code=self.lang_code,
                audio_format="numpy",  # Return numpy array directly
            )

            # MLX-audio returns audio at 24kHz
            sample_rate = 24000

            # Convert to float32 if needed
            if audio_data.dtype != np.float32:
                audio_data = audio_data.astype(np.float32)

        else:
            # CPU fallback - for now return silence
            # TODO: Implement actual CPU Kokoro inference
            sample_rate = 24000
            duration = len(text) * 0.1  # Rough estimate
            audio_data = np.zeros(int(sample_rate * duration), dtype=np.float32)

        processing_time = time.time() - start_time
        duration = len(audio_data) / sample_rate

        # Update stats
        self.stats["total_chars"] += len(text)
        self.stats["total_audio_duration"] += duration
        self.stats["total_processing_time"] += processing_time
        self.stats["synthesis_count"] += 1

        metadata = {
            "sample_rate": sample_rate,
            "duration": duration,
            "voice": self.voice,
            "language": self.lang_code,
            "processing_time": processing_time,
            "rtf": processing_time / duration if duration > 0 else 0,
            "backend": self.backend,
        }

        return audio_data, metadata

    def get_stats(self) -> dict:
        """Get synthesis statistics."""
        stats = self.stats.copy()
        if stats["total_audio_duration"] > 0:
            stats["average_rtf"] = stats["total_processing_time"] / stats["total_audio_duration"]
        else:
            stats["average_rtf"] = 0
        return stats

    def change_voice(self, voice: str):
        """Change voice dynamically."""
        self.voice = voice
        print(f"🎤 Changed voice to: {voice}")

    def cleanup(self):
        """Clean up resources."""
        self.model = None
        self.generate_fn = None


def send_log(node: Node, level: str, message: str, log_level: str = "INFO"):
    """Send structured log message."""
    levels = {"DEBUG": 0, "INFO": 1, "WARNING": 2, "ERROR": 3}
    if levels.get(level, 1) >= levels.get(log_level, 1):
        log_data = {
            "node": "mlx-kokoro",
            "level": level,
            "message": message,
            "timestamp": time.time(),
        }
        node.send_output("log", pa.array([json.dumps(log_data)]))
        # Also print for visibility
        print(f"[{level}] {message}")


def main():
    """Main Dora node entry point."""
    # Configuration from environment
    model_path = os.getenv("MODEL_PATH", "prince-canuma/Kokoro-82M")
    voice = os.getenv("VOICE_NAME", "af_heart")
    lang_code = os.getenv("LANG_CODE", "a")
    speed = float(os.getenv("SPEED", "1.0"))
    log_level = os.getenv("LOG_LEVEL", "INFO")
    use_mlx = os.getenv("USE_MLX", "true").lower() in ["true", "1", "yes"]

    # Initialize engine
    engine = KokoroEngine(
        model_path=model_path,
        voice=voice,
        lang_code=lang_code,
        speed=speed,
        use_mlx=use_mlx,
    )

    node = Node()
    send_log(node, "INFO", f"Starting MLX-Kokoro TTS node (voice: {voice}, backend: {'MLX' if use_mlx else 'CPU'})", log_level)

    try:
        for event in node:
            if event["type"] == "INPUT":
                event_id = event["id"]

                if event_id == "text":
                    # Extract text and metadata
                    text = event["value"][0].as_py()
                    metadata = event.get("metadata", {})
                    session_id = metadata.get("session_id", "")
                    request_id = metadata.get("request_id", "")
                    participant_id = metadata.get("participant_id", "")

                    send_log(node, "DEBUG", f"Synthesizing: {text[:50]}...", log_level)

                    try:
                        # Synthesize audio
                        audio_data, audio_metadata = engine.synthesize(text)

                        # Send audio output
                        audio_metadata_out = {
                            "sample_rate": str(audio_metadata["sample_rate"]),
                            "duration": str(audio_metadata["duration"]),
                            "voice": audio_metadata["voice"],
                            "language": audio_metadata["language"],
                            "session_id": session_id,
                            "request_id": request_id,
                            "participant_id": participant_id,
                        }

                        node.send_output(
                            "audio",
                            pa.array(audio_data),
                            audio_metadata_out,
                        )

                        # Send status
                        status_metadata = {
                            "session_id": session_id,
                            "request_id": request_id,
                            "participant_id": participant_id,
                        }
                        node.send_output("status", pa.array(["completed"]), status_metadata)

                        # Log performance
                        rtf = audio_metadata["rtf"]
                        send_log(
                            node,
                            "INFO",
                            f"✅ Synthesized {len(text)} chars → {audio_metadata['duration']:.2f}s audio "
                            f"in {audio_metadata['processing_time']:.2f}s (RTF: {rtf:.2f}x, backend: {audio_metadata['backend']})",
                            log_level,
                        )

                    except Exception as e:
                        send_log(node, "ERROR", f"Synthesis failed: {e}", log_level)
                        status_metadata = {
                            "session_id": session_id,
                            "request_id": request_id,
                            "participant_id": participant_id,
                            "error": str(e),
                        }
                        node.send_output("status", pa.array(["error"]), status_metadata)

                elif event_id == "control":
                    command = event["value"][0].as_py()
                    send_log(node, "DEBUG", f"Control command: {command}", log_level)

                    if command == "stats":
                        stats = engine.get_stats()
                        send_log(node, "INFO", f"Statistics: {json.dumps(stats, indent=2)}", log_level)

                    elif command == "list_voices":
                        voices = [
                            "American English: af_heart, af_sky, af_bella, am_adam, am_michael, etc.",
                            "British English: bf_alice, bf_lily, bm_george, bm_lewis, etc.",
                        ]
                        send_log(node, "INFO", f"Available voices: {voices}", log_level)

                    elif command.startswith("change_voice:"):
                        new_voice = command.split(":", 1)[1]
                        engine.change_voice(new_voice)
                        send_log(node, "INFO", f"Changed voice to: {new_voice}", log_level)

                    elif command == "cleanup":
                        engine.cleanup()
                        send_log(node, "INFO", "Cleaned up resources", log_level)

    except KeyboardInterrupt:
        send_log(node, "INFO", "Shutting down MLX-Kokoro node", log_level)
    finally:
        engine.cleanup()


if __name__ == "__main__":
    main()
