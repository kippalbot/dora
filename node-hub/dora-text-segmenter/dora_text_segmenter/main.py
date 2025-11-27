#!/usr/bin/env python3
"""
Text Segmenter Node - Default implementation using multi-participant segmenter.
This is the main entry point for the dora-text-segmenter node.
"""

# Import and use the multi-participant segmenter as the default
from .multi_participant_segmenter import main

if __name__ == "__main__":
    main()