#!/bin/bash

# Script to connect the Rust debate monitor to a running dataflow
# This sets the required environment variables for dynamic node connection

echo "Connecting Rust debate monitor to running dataflow..."

# Build the project first
echo "Building project..."
if ! cargo build; then
    echo "Build failed!"
    exit 1
fi

# Set environment variables for dynamic node connection
# These would normally be set by the Dora framework
export DORA_NODE_ID="debate-monitor"
export DORA_DAEMON_ADDR="http://127.0.0.1:50051"  # Default Dora daemon address

# For DORA_NODE_CONFIG, we need to create a configuration that matches
# what's in the dataflow-debate.yml file
# This is a simplified version - in practice, this would be more complex
export DORA_NODE_CONFIG='{"node_id":"debate-monitor","dataflow_id":"default","inputs":[{"id":"llm1_text","type":"String"},{"id":"llm1_status","type":"String"},{"id":"llm1_prompt","type":"String"},{"id":"llm2_text","type":"String"},{"id":"llm2_status","type":"String"},{"id":"llm2_prompt","type":"String"},{"id":"bundle_text","type":"String"},{"id":"judge_text","type":"String"},{"id":"judge_status","type":"String"}],"outputs":[{"id":"control","type":"String"},{"id":"bridge_control","type":"String"}]}'

echo "Environment set. Starting debate monitor..."
echo "Make sure the dataflow is running: dora start ../dataflow-debate.yml"

# Run the binary
./target/debug/debate-monitor-tui