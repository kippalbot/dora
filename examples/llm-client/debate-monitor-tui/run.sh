#!/bin/bash

# Build and run the debate monitor TUI as a dynamic node

echo "Building debate monitor TUI..."
cargo build

if [ $? -eq 0 ]; then
    echo "Starting debate monitor TUI as a dynamic node..."
    echo "IMPORTANT: Make sure the Dora dataflow is already running before starting this monitor."
    echo "To start the dataflow, run in another terminal:"
    echo "  dora start ../dataflow-debate.yml"
    echo ""
    echo "Then run this monitor directly:"
    echo "  ./target/debug/debate-monitor-tui"
    echo ""
    echo "Controls (when running):"
    echo "  - Type in the input field to send prompts to the judge"
    echo "  - Press Enter to submit a prompt"
    echo "  - Press Esc to clear the input buffer"
    echo "  - Press 'q' to quit the application"
    echo ""
    echo "Starting dynamic node..."
    ./target/debug/debate-monitor-tui
else
    echo "Build failed. Please check the error messages above."
    exit 1
fi