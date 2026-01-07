#!/bin/bash
# Integration test: GlobalRegistry with Python actors
# Tests registry-based actor discovery

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ACTORS_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PYTHON_DIR="$ACTORS_DIR/python"
REGISTRY_EXAMPLES="$ACTORS_DIR/registry/examples"

echo "=== Testing GlobalRegistry with Python actors ==="

cleanup() {
    # Kill background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT

# Start GlobalRegistry
cd "$PYTHON_DIR"
python3 -m actors.registry &
REGISTRY_PID=$!
sleep 1

# Check if registry is running
if ! kill -0 $REGISTRY_PID 2>/dev/null; then
    echo "FAIL: GlobalRegistry failed to start"
    exit 1
fi

# Start pong (registers with registry)
cd "$REGISTRY_EXAMPLES"
python3 registry_pong.py &
PONG_PID=$!
sleep 1

# Check if pong is running
if ! kill -0 $PONG_PID 2>/dev/null; then
    echo "FAIL: registry_pong.py failed to start"
    cleanup
    exit 1
fi

# Run ping (looks up pong via registry)
output=$(timeout 15 python3 registry_ping.py 2>&1) || {
    echo "FAIL: registry_ping.py timed out or crashed"
    cleanup
    exit 1
}

# Check for expected output
if echo "$output" | grep -q "Received pong 5"; then
    echo "PASS: Registry ping-pong completed successfully"
    exit 0
else
    echo "FAIL: Expected 'Received pong 5' in output"
    echo "$output"
    exit 1
fi
