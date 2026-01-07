#!/bin/bash
# Run all integration tests

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

PASSED=0
FAILED=0
TESTS=()

echo "========================================"
echo "Running Integration Tests"
echo "========================================"
echo

run_test() {
    local test_script="$1"
    local test_name=$(basename "$test_script" .sh)

    echo "----------------------------------------"
    echo "Running: $test_name"
    echo "----------------------------------------"

    if bash "$test_script"; then
        PASSED=$((PASSED + 1))
        TESTS+=("PASS: $test_name")
    else
        FAILED=$((FAILED + 1))
        TESTS+=("FAIL: $test_name")
    fi
    echo
}

# Run all test scripts
for test_script in "$SCRIPT_DIR"/test_*.sh; do
    if [ -f "$test_script" ]; then
        run_test "$test_script"
    fi
done

# Summary
echo "========================================"
echo "Integration Test Summary"
echo "========================================"
for result in "${TESTS[@]}"; do
    echo "  $result"
done
echo "----------------------------------------"
echo "Passed: $PASSED, Failed: $FAILED"
echo "========================================"

if [ $FAILED -gt 0 ]; then
    exit 1
fi
exit 0
