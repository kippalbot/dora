#!/bin/bash

# PrimeSpeech TTS Prompt Leakage Test Runner
# Tests mixed language segments to identify prompt leakage issues

set -e

echo "================================================"
echo "PrimeSpeech TTS Prompt Leakage Test"
echo "================================================"
echo

# Check if we're in the right directory
if [ ! -f "test_prompt_leakage_detection.py" ]; then
    echo "Error: Please run this script from the primespeech-validation directory"
    exit 1
fi

# Create output directory
mkdir -p tts_output

# Clean up previous test results
echo "Cleaning up previous test results..."
rm -f tts_output/*.wav
rm -f tts_output/*.json
rm -f *.log

echo "Starting prompt leakage detection test..."
echo

# Set environment variables for debugging
export LOG_LEVEL=debug
export TTS_DEBUG=true
export LEAKAGE_TEST_MODE=true

# Run the dataflow
echo "Running dataflow with leakage detection..."
echo "Dataflow: dataflow-leakage-test.yml"
echo

# Start the dataflow
dora start dataflow-leakage-test.yml --detach

echo "Dataflow started. Waiting for test completion..."
echo

# Wait for the test to run (monitor progress)
echo "Monitoring test progress (60 seconds)..."
sleep 60

# Check if dataflow is still running
echo "Checking dataflow status..."
dora dataflows list

echo
echo "Stopping dataflow..."
dora stop dataflow-leakage-test.yml || true

echo
echo "================================================"
echo "Test completed! Analyzing results..."
echo "================================================"

# Analyze results
echo
echo "Generated files:"
ls -la tts_output/

echo
echo "Audio files generated:"
find tts_output -name "*.wav" -exec ls -la {} \; 2>/dev/null || echo "No audio files found"

echo
echo "Log files generated:"
find tts_output -name "*.json" -exec ls -la {} \; 2>/dev/null || echo "No log files found"

echo
echo "================================================"
echo "Analysis Instructions:"
echo "================================================"
echo
echo "1. Check audio files for prompt leakage:"
echo "   - Listen for text from previous segments"
echo "   - Look for mixed language contamination"
echo "   - Verify segment boundaries"
echo
echo "2. Review metadata logs:"
echo "   - Check tts_output/prompt_leakage_test_results.json"
echo "   - Review tts_output/leakage_monitor_log.json"
echo "   - Analyze timing and ordering"
echo
echo "3. Key indicators of leakage:"
echo "   - Previous segment text in current audio"
echo "   - Incomplete or truncated segments"
echo "   - Language mixing between segments"
echo "   - Delayed audio output"
echo
echo "4. Compare expected vs actual:"
echo "   - Expected segments are in test_prompt_leakage_detection.py"
echo "   - Actual output is in generated audio files"
echo "   - Metadata should show segment ordering"
echo

# Show quick summary if results exist
if [ -f "tts_output/prompt_leakage_test_results.json" ]; then
    echo "Quick summary from test results:"
    python3 -c "
import json
with open('tts_output/prompt_leakage_test_results.json', 'r') as f:
    data = json.load(f)
print(f'Test type: {data.get(\"test_type\", \"unknown\")}')
print(f'Total scenarios: {data.get(\"total_scenarios\", 0)}')
print(f'Total segments: {data.get(\"total_segments\", 0)}')
print(f'Test time: {data.get(\"total_test_time\", 0):.2f}s')
print(f'Scenarios: {\", \".join(data.get(\"scenarios_tested\", []))}')
"
fi

echo
echo "================================================"
echo "Test complete! Check the files above for leakage evidence."
echo "================================================"