#!/usr/bin/env python3
"""
Direct test of TTS with timing and audio file saving.
"""

import sys
import time
import os
import argparse
import numpy as np
import soundfile as sf
from pathlib import Path

def test_tts_timing(wrapper_class, voice='doubao', device='cpu'):
    """Test TTS with Chinese text and save audio.
    
    Args:
        wrapper_class: The StreamingMoYoYoTTSWrapper class
        voice: Voice name to use
        device: Device to use (cpu or cuda)
    """
    
    print("=" * 80)
    print("Direct TTS Timing Test")
    print("=" * 80)
    
    # The Chinese text from the test
    chinese_text = """我们说中国式现代化是百年大战略，这又分为三个阶段。第一个阶段，我们先用30年时间建成了独立完整的工业体系和国民经济体系；再用40年，到2021年，全面建成了小康社会。我们现在正处于第三个阶段，这又被分成上下两篇：上半篇是到2035年基本实现社会主义现代化；下半篇是到本世纪中叶，也就是2050年，建成社会主义现代化强国。"""
    
    print(f"\nText: {len(chinese_text)} characters")
    print("-" * 60)
    print(chinese_text)
    print("-" * 60)
    
    # Initialize TTS
    print("\nInitializing TTS engine...")
    init_start = time.time()
    
    wrapper = wrapper_class(
        voice=voice,
        device=device,
        enable_streaming=False  # Use batch mode for simplicity
    )
    
    init_time = time.time() - init_start
    print(f"Initialization time: {init_time:.2f}s")
    
    # Generate audio
    print("\nGenerating audio...")
    synthesis_start = time.time()
    
    sample_rate, audio_data = wrapper.synthesize(
        chinese_text, 
        language='zh', 
        speed=1.0
    )
    
    synthesis_time = time.time() - synthesis_start
    audio_duration = len(audio_data) / sample_rate
    
    # Calculate metrics
    chars_per_second = len(chinese_text) / synthesis_time
    real_time_factor = audio_duration / synthesis_time
    
    # Save audio
    output_dir = Path("tts_output")
    output_dir.mkdir(exist_ok=True)
    output_file = output_dir / "chinese_tts_output.wav"
    
    sf.write(output_file, audio_data, sample_rate)
    
    # Print results
    print("\n" + "=" * 80)
    print("RESULTS")
    print("=" * 80)
    print(f"Text length: {len(chinese_text)} characters")
    print(f"Audio duration: {audio_duration:.2f} seconds")
    print(f"Synthesis time: {synthesis_time:.2f} seconds")
    print(f"Real-time factor: {real_time_factor:.2f}x {'(faster than real-time)' if real_time_factor > 1 else '(slower than real-time)'}")
    print(f"Processing speed: {chars_per_second:.1f} characters/second")
    print(f"\nAudio saved to: {output_file.absolute()}")
    print(f"File size: {output_file.stat().st_size / 1024:.1f} KB")
    print("=" * 80)
    
    return output_file


def main():
    """Main function with argument parsing."""
    parser = argparse.ArgumentParser(
        description="Test TTS with timing and audio file saving",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        epilog="""
Examples:
  # Use default paths
  python test_tts_direct.py
  
  # Specify custom paths
  python test_tts_direct.py --primespeech-path /path/to/dora-primespeech --model-dir /path/to/models
  
  # Use different voice
  python test_tts_direct.py --voice maple
  
  # Use GPU acceleration
  python test_tts_direct.py --device cuda
        """
    )
    
    parser.add_argument(
        '--primespeech-path',
        type=str,
        default=os.path.expanduser('../../../node-hub/dora-primespeech'),
        help='Path to dora-primespeech module'
    )
    
    parser.add_argument(
        '--model-dir',
        type=str,
        default=os.path.expanduser('~/.dora/models/primespeech'),
        help='Path to PrimeSpeech models directory'
    )
    
    parser.add_argument(
        '--voice',
        type=str,
        default='luoxiang',
        choices=['luoxiang', 'mayun', 'bys', 'dnz', 'yfc', 'doubao'],
        help='Voice to use for TTS (available voices with model files)'
    )
    
    # Auto-detect best device
    import torch
    if torch.cuda.is_available():
        default_device = 'cuda'
    elif torch.backends.mps.is_available():
        default_device = 'mps'
    else:
        default_device = 'cpu'

    parser.add_argument(
        '--device',
        type=str,
        default=default_device,
        choices=['cpu', 'cuda', 'mps'],
        help=f'Device to use for TTS (auto-detected: {default_device})'
    )

    args = parser.parse_args()
    
    # Setup paths
    primespeech_path = Path(args.primespeech_path).expanduser().resolve()
    model_dir = Path(args.model_dir).expanduser().resolve()
    
    # Validate paths
    if not primespeech_path.exists():
        print(f"Error: PrimeSpeech path does not exist: {primespeech_path}")
        sys.exit(1)
    
    if not model_dir.exists():
        print(f"Error: Model directory does not exist: {model_dir}")
        print(f"Please download models first using download_models.py")
        sys.exit(1)
    
    # Add PrimeSpeech to path
    sys.path.insert(0, str(primespeech_path))
    
    # Set environment variable
    os.environ['PRIMESPEECH_MODEL_DIR'] = str(model_dir)
    
    print(f"Using PrimeSpeech path: {primespeech_path}")
    print(f"Using model directory: {model_dir}")
    print(f"Using voice: {args.voice}")
    print(f"Using device: {args.device}")
    
    # Import after setting up paths
    try:
        from dora_primespeech.moyoyo_tts_wrapper_streaming_fix import StreamingMoYoYoTTSWrapper
    except ImportError as e:
        print(f"Error importing TTS wrapper: {e}")
        print("Make sure dora-primespeech is properly installed")
        sys.exit(1)
    
    # Run test
    try:
        audio_file = test_tts_timing(StreamingMoYoYoTTSWrapper, args.voice, args.device)
        print(f"\n✓ Test completed successfully!")
        print(f"✓ Audio file: {audio_file}")
    except Exception as e:
        print(f"\n✗ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()