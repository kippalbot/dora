#!/bin/bash

echo "=== Cleaning up and restarting PrimeSpeech ==="

# Kill any existing PrimeSpeech processes
echo "1. Killing existing PrimeSpeech processes..."
pkill -f dora-primespeech 2>/dev/null
pkill -f primespeech 2>/dev/null

# Clear Python cache
echo "2. Clearing Python cache files..."
find /Users/yuechen/home/fresh/dora/node-hub/dora-primespeech -type f -name "*.pyc" -delete 2>/dev/null
find /Users/yuechen/home/fresh/dora/node-hub/dora-primespeech -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null

# Reinstall the package
echo "3. Reinstalling dora-primespeech package..."
cd /Users/yuechen/home/fresh/dora/node-hub/dora-primespeech
pip uninstall dora-primespeech -y > /dev/null 2>&1
pip install -e . --no-cache-dir > /dev/null 2>&1

echo "4. Ready to start fresh!"
echo ""
echo "Now run: dora start voice-chat-with-aec.yml"
echo ""
echo "You should see debug messages like:"
echo "  [PRIMESPEECH MODULE] Module is being imported"
echo "  [PRIMESPEECH START] Entering main function"
echo "  [PRIMESPEECH DEBUG] Received text: '...'"
echo ""