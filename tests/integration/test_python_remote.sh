#!/bin/bash
# Integration test: Python remote actors via ZMQ
# Tests cross-process actor communication

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ACTORS_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PYTHON_DIR="$ACTORS_DIR/python"

echo "=== Testing Python remote ping-pong ==="

cd "$PYTHON_DIR/examples/remote_ping_pong"

cleanup() {
    # Kill background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT

# Start pong (server) in background
python3 pong_process.py &
PONG_PID=$!
sleep 1

# Check if pong is running
if ! kill -0 $PONG_PID 2>/dev/null; then
    echo "FAIL: pong_process.py failed to start"
    exit 1
fi

# Run ping (client) with timeout
output=$(timeout 10 python3 ping_process.py 2>&1) || {
    echo "FAIL: ping_process.py timed out or crashed"
    cleanup
    exit 1
}

# Check for expected output
if echo "$output" | grep -q "pong 5"; then
    echo "PASS: Python remote ping-pong completed successfully"
    exit 0
else
    echo "FAIL: Expected 'pong 5' in output"
    echo "$output"
    exit 1
fi
