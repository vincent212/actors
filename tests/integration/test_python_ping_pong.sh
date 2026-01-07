#!/bin/bash
# Integration test: Python ping-pong actors
# Tests local actor communication within same process

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ACTORS_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PYTHON_DIR="$ACTORS_DIR/python"

echo "=== Testing Python ping-pong ==="

cd "$PYTHON_DIR/examples"

# Run ping_pong example with timeout
output=$(timeout 10 python3 ping_pong.py 2>&1) || {
    echo "FAIL: ping_pong.py timed out or crashed"
    echo "$output"
    exit 1
}

# Check for expected output
if echo "$output" | grep -q "Received pong 5"; then
    echo "PASS: Python ping-pong completed successfully"
    exit 0
else
    echo "FAIL: Expected 'Received pong 5' in output"
    echo "$output"
    exit 1
fi
