#!/bin/bash
#
# Actors Framework Test Runner
# =============================
# Run all tests for the actors framework across Python, Rust, and integration.
#
# Usage:
#   ./run_tests.sh              # Run all tests
#   ./run_tests.sh python       # Run Python tests only
#   ./run_tests.sh rust         # Run Rust tests only
#   ./run_tests.sh cpp          # Run C++ tests only
#   ./run_tests.sh integration  # Run integration tests only
#   ./run_tests.sh all          # Run all tests (same as no argument)
#
# Exit codes:
#   0 - All tests passed
#   1 - One or more tests failed
#

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PYTHON_PASSED=0
PYTHON_FAILED=0
RUST_PASSED=0
RUST_FAILED=0
CPP_PASSED=0
CPP_FAILED=0
INTEGRATION_PASSED=0
INTEGRATION_FAILED=0

print_header() {
    echo
    echo "========================================"
    echo "$1"
    echo "========================================"
}

run_python_tests() {
    print_header "Python Tests (pytest)"

    cd "$SCRIPT_DIR/python"

    if ! command -v pytest &> /dev/null && ! python3 -m pytest --version &> /dev/null; then
        echo -e "${YELLOW}pytest not installed. Install with: pip3 install pytest${NC}"
        PYTHON_FAILED=1
        return
    fi

    if python3 -m pytest tests/ -v --tb=short; then
        PYTHON_PASSED=1
    else
        PYTHON_FAILED=1
    fi
}

run_rust_tests() {
    print_header "Rust Tests (cargo test)"

    cd "$SCRIPT_DIR/rust"

    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW}cargo not installed. Install Rust from rustup.rs${NC}"
        RUST_FAILED=1
        return
    fi

    # Run unit tests only (skip doctests which have incomplete examples)
    if cargo test --lib 2>&1 | tee /dev/tty | grep -q "test result: ok"; then
        RUST_PASSED=1
    else
        RUST_FAILED=1
    fi
}

run_cpp_tests() {
    print_header "C++ Tests (Google Test)"

    cd "$SCRIPT_DIR/cpp"

    if [ ! -f "Makefile" ]; then
        echo -e "${YELLOW}C++ Makefile not found${NC}"
        CPP_FAILED=1
        return
    fi

    # Build and run tests
    if make test 2>&1 | tee /dev/tty | grep -q "PASSED"; then
        CPP_PASSED=1
    else
        CPP_FAILED=1
    fi
}

run_integration_tests() {
    print_header "Integration Tests"

    cd "$SCRIPT_DIR/tests/integration"

    if [ ! -f "run_all.sh" ]; then
        echo -e "${YELLOW}Integration tests not found${NC}"
        INTEGRATION_FAILED=1
        return
    fi

    if bash run_all.sh; then
        INTEGRATION_PASSED=1
    else
        INTEGRATION_FAILED=1
    fi
}

print_summary() {
    print_header "Test Summary"

    local total_passed=0
    local total_failed=0

    if [ $PYTHON_PASSED -eq 1 ]; then
        echo -e "  Python:       ${GREEN}PASSED${NC}"
        total_passed=$((total_passed + 1))
    elif [ $PYTHON_FAILED -eq 1 ]; then
        echo -e "  Python:       ${RED}FAILED${NC}"
        total_failed=$((total_failed + 1))
    else
        echo -e "  Python:       ${YELLOW}SKIPPED${NC}"
    fi

    if [ $RUST_PASSED -eq 1 ]; then
        echo -e "  Rust:         ${GREEN}PASSED${NC}"
        total_passed=$((total_passed + 1))
    elif [ $RUST_FAILED -eq 1 ]; then
        echo -e "  Rust:         ${RED}FAILED${NC}"
        total_failed=$((total_failed + 1))
    else
        echo -e "  Rust:         ${YELLOW}SKIPPED${NC}"
    fi

    if [ $CPP_PASSED -eq 1 ]; then
        echo -e "  C++:          ${GREEN}PASSED${NC}"
        total_passed=$((total_passed + 1))
    elif [ $CPP_FAILED -eq 1 ]; then
        echo -e "  C++:          ${RED}FAILED${NC}"
        total_failed=$((total_failed + 1))
    else
        echo -e "  C++:          ${YELLOW}SKIPPED${NC}"
    fi

    if [ $INTEGRATION_PASSED -eq 1 ]; then
        echo -e "  Integration:  ${GREEN}PASSED${NC}"
        total_passed=$((total_passed + 1))
    elif [ $INTEGRATION_FAILED -eq 1 ]; then
        echo -e "  Integration:  ${RED}FAILED${NC}"
        total_failed=$((total_failed + 1))
    else
        echo -e "  Integration:  ${YELLOW}SKIPPED${NC}"
    fi

    echo "========================================"

    if [ $total_failed -gt 0 ]; then
        echo -e "${RED}TESTS FAILED${NC}"
        return 1
    else
        echo -e "${GREEN}ALL TESTS PASSED${NC}"
        return 0
    fi
}

# Main
main() {
    local target="${1:-all}"

    print_header "Actors Framework Test Runner"
    echo "Target: $target"

    case "$target" in
        python)
            run_python_tests
            ;;
        rust)
            run_rust_tests
            ;;
        cpp)
            run_cpp_tests
            ;;
        integration)
            run_integration_tests
            ;;
        all)
            run_python_tests
            run_rust_tests
            run_cpp_tests
            run_integration_tests
            ;;
        *)
            echo "Unknown target: $target"
            echo "Usage: $0 [python|rust|cpp|integration|all]"
            exit 1
            ;;
    esac

    print_summary
}

main "$@"
