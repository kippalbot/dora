#!/bin/bash
set -e

echo "🚀 MaaS Client Cancellation Test"
echo "===================================="

# Check for OpenAI API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo "❌ Error: OPENAI_API_KEY environment variable not set"
    echo ""
    echo "Please set your API key before running:"
    echo "    export OPENAI_API_KEY='your-api-key-here'"
    echo ""
    exit 1
fi

echo "✅ OPENAI_API_KEY is set"
echo ""

# Create a temporary directory for logs
LOG_DIR="/tmp/maas-cancellation-test"
mkdir -p "$LOG_DIR"

# Clear any existing dora daemon
echo "🧹 Cleaning up any existing Dora daemon..."
dora stop --name maas-cancellation-test >/dev/null 2>&1 || true
sleep 1

# Terminal 1: Start dora dataflow in background
echo "🔄 Starting Dora dataflow..."
dora start dataflow.yml --name maas-cancellation-test > "$LOG_DIR/dataflow.log" 2>&1 &
DATAFLOW_PID=$!
sleep 3

# Check if dataflow started successfully
if ! ps -p $DATAFLOW_PID > /dev/null || grep -q "error" "$LOG_DIR/dataflow.log" 2>/dev/null; then
    echo "❌ Failed to start dataflow"
    echo "Check logs: $LOG_DIR/dataflow.log"
    echo ""
    echo "Log contents:"
    cat "$LOG_DIR/dataflow.log"
    exit 1
fi

echo "✅ Dataflow started (PID: $DATAFLOW_PID)"
echo ""

# Give it a moment to initialize
sleep 5

echo "✅ Dora daemon running. Starting test in 3 seconds..."
echo ""
sleep 3

# Terminal 2: Run test sender (this will be our main output)
echo "📝 Running test sender..."
echo "===================================="
python test_sender.py

# Give time for final events to propagate
echo ""
echo "⏳ Waiting for events to propagate..."
sleep 3

# Cleanup
echo ""
echo "🧹 Cleaning up..."
dora stop --name maas-cancellation-test >/dev/null 2>&1 || true
sleep 2

echo ""
echo "✅ Test completed!"
echo ""
echo "📊 Check the logs:"
echo "   Dataflow log: $LOG_DIR/dataflow.log"
echo ""
echo "🎯 What to look for in the output above:"
echo "   - Text chunks should STOP after cancellation"
echo "   - Cancellation events should be emitted"
echo "   - Status should change to 'cancelled'"
