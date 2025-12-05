#!/usr/bin/env python3
"""
English Voice Verification Test for Kokoro TTS.
Tests all English male and female voices with performance comparison.
"""

import sys
import time
import os
import argparse
import numpy as np
import soundfile as sf
import json
from pathlib import Path
from typing import Dict, List, Tuple


# English voice definitions (from Kokoro documentation)
ENGLISH_VOICES = {
    'male': {
        'am_adam': 'English Male - Adam',
        'am_michael': 'English Male - Michael',
        'bm_george': 'English Male - George',
        'bm_lewis': 'English Male - Lewis',
    },
    'female': {
        'af_alloy': 'English Female - Alloy',
        'af_aoede': 'English Female - Aoede',
        'af_bella': 'English Female - Bella',
        'af_heart': 'English Female - Heart',
        'af_jessica': 'English Female - Jessica',
        'af_kore': 'English Female - Kore',
        'af_nicole': 'English Female - Nicole',
        'af_nova': 'English Female - Nova',
        'af_river': 'English Female - River',
        'af_sarah': 'English Female - Sarah',
        'af_sky': 'English Female - Sky',
        'bf_emma': 'English Female - Emma',
    }
}

# Test text - English passage
DEFAULT_TEXT = """Artificial intelligence is revolutionizing the way we interact with technology. From voice assistants to autonomous vehicles, AI systems are becoming increasingly sophisticated. Machine learning algorithms can now process vast amounts of data, recognize patterns, and make decisions with remarkable accuracy."""


def test_voice(voice_id: str, voice_name: str, text: str, lang_code: str = 'a') -> Dict:
    """Test a single voice and return metrics.

    Args:
        voice_id: Voice identifier (e.g., am_adam)
        voice_name: Human-readable voice name
        text: Text to synthesize
        lang_code: Language code ('a' for English)

    Returns:
        Dictionary with test results and metrics
    """
    print(f"\nTesting: {voice_name} ({voice_id})")
    print("-" * 60)

    try:
        from kokoro import KPipeline
    except ImportError as e:
        print(f"Error: Failed to import kokoro module: {e}")
        print("Please install kokoro: pip install kokoro")
        sys.exit(1)

    # Initialize pipeline
    init_start = time.time()
    pipeline = KPipeline(lang_code=lang_code)
    init_time = time.time() - init_start

    # Generate audio
    synthesis_start = time.time()
    audio_chunks = []
    sample_rate = 24000  # Kokoro default

    try:
        generator = pipeline(
            text,
            voice=voice_id,
            speed=1.0,
            split_pattern=r"\n+",
        )

        for _, (_, _, audio) in enumerate(generator):
            audio_np = audio.numpy()
            audio_chunks.append(audio_np)

    except Exception as e:
        print(f"✗ Error generating audio for {voice_id}: {e}")
        return {
            'voice_id': voice_id,
            'voice_name': voice_name,
            'status': 'error',
            'error': str(e)
        }

    # Concatenate chunks
    audio_data = np.concatenate(audio_chunks) if audio_chunks else np.array([])
    synthesis_time = time.time() - synthesis_start

    if len(audio_data) == 0:
        print(f"✗ No audio generated for {voice_id}")
        return {
            'voice_id': voice_id,
            'voice_name': voice_name,
            'status': 'error',
            'error': 'No audio generated'
        }

    # Calculate metrics
    audio_duration = len(audio_data) / sample_rate
    chars_per_second = len(text) / synthesis_time if synthesis_time > 0 else 0
    real_time_factor = audio_duration / synthesis_time if synthesis_time > 0 else 0

    # Save audio
    output_dir = Path("tts_output/english_voices")
    output_dir.mkdir(parents=True, exist_ok=True)
    output_file = output_dir / f"{voice_id}_test.wav"

    sf.write(output_file, audio_data, sample_rate)
    file_size = output_file.stat().st_size

    # Print results
    print(f"  Audio duration: {audio_duration:.2f}s")
    print(f"  Synthesis time: {synthesis_time:.3f}s")
    print(f"  RTF: {real_time_factor:.2f}x")
    print(f"  Speed: {chars_per_second:.1f} chars/sec")
    print(f"  File saved: {output_file.name} ({file_size / 1024:.1f} KB)")
    print(f"  ✓ Success")

    return {
        'voice_id': voice_id,
        'voice_name': voice_name,
        'status': 'success',
        'init_time': init_time,
        'synthesis_time': synthesis_time,
        'audio_duration': audio_duration,
        'rtf': real_time_factor,
        'chars_per_second': chars_per_second,
        'file_path': str(output_file),
        'file_size': file_size,
        'sample_rate': sample_rate,
        'text_length': len(text)
    }


def test_all_voices(voice_type: str = 'all', text: str = None) -> Dict:
    """Test all English voices or specific gender.

    Args:
        voice_type: 'male', 'female', or 'all'
        text: Custom text to synthesize (optional)

    Returns:
        Dictionary with all test results
    """
    print("=" * 80)
    print("ENGLISH VOICE VERIFICATION TEST - KOKORO TTS")
    print("=" * 80)

    test_text = text or DEFAULT_TEXT
    print(f"\nTest text: {len(test_text)} characters")
    print("-" * 60)
    print(test_text[:150] + "..." if len(test_text) > 150 else test_text)
    print("-" * 60)

    results = {
        'timestamp': time.strftime('%Y-%m-%d %H:%M:%S'),
        'text_length': len(test_text),
        'test_text': test_text,
        'male_voices': {},
        'female_voices': {},
        'summary': {}
    }

    # Test male voices
    if voice_type in ['male', 'all']:
        print("\n" + "=" * 80)
        print("TESTING MALE VOICES")
        print("=" * 80)

        for voice_id, voice_name in ENGLISH_VOICES['male'].items():
            result = test_voice(voice_id, voice_name, test_text)
            results['male_voices'][voice_id] = result

    # Test female voices
    if voice_type in ['female', 'all']:
        print("\n" + "=" * 80)
        print("TESTING FEMALE VOICES")
        print("=" * 80)

        for voice_id, voice_name in ENGLISH_VOICES['female'].items():
            result = test_voice(voice_id, voice_name, test_text)
            results['female_voices'][voice_id] = result

    # Calculate summary statistics
    all_results = list(results['male_voices'].values()) + list(results['female_voices'].values())
    successful_results = [r for r in all_results if r['status'] == 'success']

    if successful_results:
        results['summary'] = {
            'total_tested': len(all_results),
            'successful': len(successful_results),
            'failed': len(all_results) - len(successful_results),
            'avg_rtf': np.mean([r['rtf'] for r in successful_results]),
            'avg_synthesis_time': np.mean([r['synthesis_time'] for r in successful_results]),
            'avg_chars_per_second': np.mean([r['chars_per_second'] for r in successful_results]),
            'fastest_voice': max(successful_results, key=lambda x: x['rtf'])['voice_id'],
            'slowest_voice': min(successful_results, key=lambda x: x['rtf'])['voice_id'],
        }

    return results


def print_summary(results: Dict):
    """Print formatted summary of test results."""
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    summary = results.get('summary', {})

    print(f"\nTotal voices tested: {summary.get('total_tested', 0)}")
    print(f"Successful: {summary.get('successful', 0)}")
    print(f"Failed: {summary.get('failed', 0)}")

    if summary.get('successful', 0) > 0:
        print(f"\nAverage RTF: {summary.get('avg_rtf', 0):.2f}x")
        print(f"Average synthesis time: {summary.get('avg_synthesis_time', 0):.3f}s")
        print(f"Average processing speed: {summary.get('avg_chars_per_second', 0):.1f} chars/sec")
        print(f"\nFastest voice: {summary.get('fastest_voice', 'N/A')}")
        print(f"Slowest voice: {summary.get('slowest_voice', 'N/A')}")

    # Print voice comparison table
    print("\n" + "=" * 80)
    print("VOICE COMPARISON")
    print("=" * 80)
    print(f"{'Voice ID':<20} {'Type':<8} {'RTF':<8} {'Time (s)':<10} {'Speed':<12} {'Status':<10}")
    print("-" * 80)

    for voice_id, result in results.get('male_voices', {}).items():
        if result['status'] == 'success':
            print(f"{voice_id:<20} {'Male':<8} {result['rtf']:<8.2f} {result['synthesis_time']:<10.3f} "
                  f"{result['chars_per_second']:<12.1f} {'✓ Success':<10}")
        else:
            print(f"{voice_id:<20} {'Male':<8} {'N/A':<8} {'N/A':<10} {'N/A':<12} {'✗ Failed':<10}")

    for voice_id, result in results.get('female_voices', {}).items():
        if result['status'] == 'success':
            print(f"{voice_id:<20} {'Female':<8} {result['rtf']:<8.2f} {result['synthesis_time']:<10.3f} "
                  f"{result['chars_per_second']:<12.1f} {'✓ Success':<10}")
        else:
            print(f"{voice_id:<20} {'Female':<8} {'N/A':<8} {'N/A':<10} {'N/A':<12} {'✗ Failed':<10}")

    print("=" * 80)

    # Print audio file locations
    print("\n" + "=" * 80)
    print("GENERATED AUDIO FILES")
    print("=" * 80)
    print("\nAll audio files saved to: tts_output/english_voices/")
    print("\nMale voices:")
    for voice_id in results.get('male_voices', {}):
        print(f"  - {voice_id}_test.wav")
    print("\nFemale voices:")
    for voice_id in results.get('female_voices', {}):
        print(f"  - {voice_id}_test.wav")
    print("\nListen to these files to compare voice quality and characteristics.")
    print("=" * 80)


def main():
    """Main function with argument parsing."""
    parser = argparse.ArgumentParser(
        description="Test all English voices in Kokoro TTS",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        epilog="""
Examples:
  # Test all English voices (male and female)
  python test_english_voices.py

  # Test only male voices
  python test_english_voices.py --type male

  # Test only female voices
  python test_english_voices.py --type female

  # Test with custom text
  python test_english_voices.py --text "Hello world, this is a test."
        """
    )

    parser.add_argument(
        '--type',
        type=str,
        default='all',
        choices=['all', 'male', 'female'],
        help='Which voices to test'
    )

    parser.add_argument(
        '--text',
        type=str,
        default=None,
        help='Custom English text to synthesize (optional)'
    )

    parser.add_argument(
        '--save-json',
        type=str,
        default='tts_output/english_voices_results.json',
        help='Path to save JSON results'
    )

    args = parser.parse_args()

    # Run tests
    try:
        results = test_all_voices(args.type, args.text)

        # Print summary
        print_summary(results)

        # Save JSON results
        output_file = Path(args.save_json)
        output_file.parent.mkdir(parents=True, exist_ok=True)

        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)

        print(f"\n✓ Results saved to: {output_file}")
        print(f"✓ Test completed successfully!")

    except Exception as e:
        print(f"\n✗ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
