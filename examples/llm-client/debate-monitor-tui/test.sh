#!/bin/bash

# Test script for the debate monitor TUI

echo "Testing debate monitor TUI..."

# Check if cargo is available
if ! command -v cargo &> /dev/null
then
    echo "Error: cargo is not installed or not in PATH"
    exit 1
fi

# Run cargo check to verify compilation
echo "1. Checking compilation..."
if cargo check; then
    echo "✓ Compilation check passed"
else
    echo "✗ Compilation check failed"
    exit 1
fi

# Run cargo build to verify full build
echo "2. Building project..."
if cargo build; then
    echo "✓ Build successful"
else
    echo "✗ Build failed"
    exit 1
fi

echo "✓ All tests passed"
echo "You can now run the application with: cargo run"
echo "Or use the run script: ./run.sh"