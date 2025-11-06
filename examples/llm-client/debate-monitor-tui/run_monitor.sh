#!/bin/bash

# Script to run the Rust debate monitor with proper error handling

echo "=== Rust Debate Monitor ==="
echo "Make sure the Dora dataflow is running before starting this monitor."
echo ""
echo "To start the dataflow, run in another terminal:"
echo "  cd .. && dora start dataflow-debate.yml"
echo ""
echo "Press Enter to continue or Ctrl+C to cancel..."
read

echo "Starting Rust debate monitor..."
echo "Controls:"
echo "  - Type in the input field to send prompts to the judge"
echo "  - Press Enter to submit a prompt"
echo "  - Press Esc to clear the input buffer"
echo "  - Type 'reset' and press Enter to reset bridges"
echo "  - Press 'q' to quit the application"
echo ""

# Run the monitor
cargo run -r

echo ""
echo "Monitor exited."