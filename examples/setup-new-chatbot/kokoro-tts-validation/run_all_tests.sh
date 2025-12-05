#!/bin/bash

# Kokoro TTS Validation Test Suite
# Runs voice verification and TTS tests for dora-kokoro-tts

set -e  # Exit on error

echo "================================================================================"
echo "Kokoro TTS Voice Verification Suite"
echo "================================================================================"
echo ""

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name=$1
    local test_command=$2

    echo ""
    echo "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "${BLUE}Running: ${test_name}${NC}"
    echo "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    if eval "$test_command"; then
        echo ""
        echo "${GREEN}✓ ${test_name} PASSED${NC}"
        ((TESTS_PASSED++))
    else
        echo ""
        echo "${RED}✗ ${test_name} FAILED${NC}"
        ((TESTS_FAILED++))
    fi
}

echo "${YELLOW}Checking Dependencies${NC}"
echo "-----------------------------------"

# Check if kokoro is available
KOKORO_AVAILABLE=false
python3 -c "import kokoro" 2>/dev/null && KOKORO_AVAILABLE=true

if [ "$KOKORO_AVAILABLE" = true ]; then
    echo "${GREEN}✓${NC} Kokoro TTS is installed"
else
    echo "${RED}✗${NC} Kokoro TTS is NOT installed"
    echo "   Install with: pip install kokoro>=0.2.2"
    exit 1
fi

# Check for Chinese support
CHINESE_SUPPORT=false
python3 -c "import misaki" 2>/dev/null && CHINESE_SUPPORT=true

if [ "$CHINESE_SUPPORT" = true ]; then
    echo "${GREEN}✓${NC} Chinese support (misaki) is installed"
else
    echo "${YELLOW}⚠${NC} Chinese support (misaki) is NOT installed"
    echo "   Install with: pip install \"misaki[zh]\""
fi

echo ""
echo "================================================================================"
echo "Voice Verification Tests"
echo "================================================================================"

# Test 1: Chinese Voice Verification
if [ "$CHINESE_SUPPORT" = true ]; then
    run_test "Chinese Voice Verification (All Voices)" \
        "python test_chinese_voices.py --type all"
else
    echo "${YELLOW}Skipping Chinese Voice Verification (misaki not installed)${NC}"
fi

# Test 2: English Voice Verification
run_test "English Voice Verification (All Voices)" \
    "python test_english_voices.py --type all"

echo ""
echo "================================================================================"
echo "Basic TTS Tests"
echo "================================================================================"

# Test 3: Direct TTS Test - English
run_test "Direct TTS Test (English)" \
    "python test_tts_direct.py --language en"

# Test 4: Direct TTS Test - Chinese
if [ "$CHINESE_SUPPORT" = true ]; then
    run_test "Direct TTS Test (Chinese)" \
        "python test_tts_direct.py --language zh"
else
    echo "${YELLOW}Skipping Chinese TTS Test (misaki not installed)${NC}"
fi

# Note about dataflow test
if command -v dora &> /dev/null; then
    echo ""
    echo "${YELLOW}Note: Dataflow test requires manual intervention${NC}"
    echo "${YELLOW}To run dataflow test manually:${NC}"
    echo "${YELLOW}  dora destroy${NC}"
    echo "${YELLOW}  dora up${NC}"
    echo "${YELLOW}  dora start dataflow-static.yml${NC}"
    echo "${YELLOW}  # Wait for completion, then Ctrl+C${NC}"
else
    echo ""
    echo "${YELLOW}Warning: 'dora' command not found. Dataflow test not available.${NC}"
fi

# Summary
echo ""
echo "================================================================================"
echo "Test Summary"
echo "================================================================================"
echo ""
echo "Tests Passed: ${GREEN}${TESTS_PASSED}${NC}"
echo "Tests Failed: ${RED}${TESTS_FAILED}${NC}"
echo ""

# Check output files
echo "Output Files:"
echo "------------"

# Voice verification outputs
if [ -d "tts_output/chinese_voices" ]; then
    VOICE_COUNT=$(ls -1 tts_output/chinese_voices/*.wav 2>/dev/null | wc -l | tr -d ' ')
    if [ "$VOICE_COUNT" -gt 0 ]; then
        echo "${GREEN}✓${NC} Chinese voices: ${VOICE_COUNT} audio files generated"
        echo "   Directory: tts_output/chinese_voices/"
    fi
fi

if [ -d "tts_output/english_voices" ]; then
    VOICE_COUNT=$(ls -1 tts_output/english_voices/*.wav 2>/dev/null | wc -l | tr -d ' ')
    if [ "$VOICE_COUNT" -gt 0 ]; then
        echo "${GREEN}✓${NC} English voices: ${VOICE_COUNT} audio files generated"
        echo "   Directory: tts_output/english_voices/"
    fi
fi

# Direct TTS outputs
if [ -f "tts_output/kokoro_en_output.wav" ]; then
    SIZE=$(du -h "tts_output/kokoro_en_output.wav" | cut -f1)
    echo "${GREEN}✓${NC} English TTS: tts_output/kokoro_en_output.wav (${SIZE})"
fi

if [ -f "tts_output/kokoro_zh_output.wav" ]; then
    SIZE=$(du -h "tts_output/kokoro_zh_output.wav" | cut -f1)
    echo "${GREEN}✓${NC} Chinese TTS: tts_output/kokoro_zh_output.wav (${SIZE})"
fi

# JSON results
if [ -f "tts_output/chinese_voices_results.json" ]; then
    echo "${GREEN}✓${NC} Chinese voice metrics: tts_output/chinese_voices_results.json"
fi

if [ -f "tts_output/english_voices_results.json" ]; then
    echo "${GREEN}✓${NC} English voice metrics: tts_output/english_voices_results.json"
fi

echo ""
echo "================================================================================"

# Exit with appropriate code
if [ $TESTS_FAILED -eq 0 ]; then
    echo "${GREEN}All tests passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  - Listen to voice samples in tts_output/chinese_voices/ and tts_output/english_voices/"
    echo "  - Review voice performance metrics in JSON files"
    echo "  - Choose your preferred voices for your application"
    echo "  - Test with Dora dataflow: dora start dataflow-static.yml"
    exit 0
else
    echo "${RED}Some tests failed!${NC}"
    echo ""
    echo "Check the output above for details"
    exit 1
fi
