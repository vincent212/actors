# Testing the Actors Framework

This document describes how to run and write tests for the actors framework.

## Quick Start

Run all tests with a single command:

```bash
./run_tests.sh
```

## Test Structure

```
actors/
├── run_tests.sh                    # Master test runner
├── python/
│   ├── pytest.ini                  # Pytest configuration
│   └── tests/
│       ├── test_serialization.py   # Message serialization tests
│       ├── test_actor.py           # Actor/Envelope/ActorRef tests
│       ├── test_manager.py         # Manager lifecycle tests
│       ├── test_registry_messages.py # Registry message format tests
│       └── test_registry.py        # GlobalRegistry state tests
├── rust/
│   └── src/
│       └── registry.rs             # Contains inline #[test] functions
├── cpp/
│   ├── Makefile                    # Has 'make test' target
│   └── tests/
│       ├── test_message.cpp        # Message class tests
│       ├── test_queue.cpp          # BQueue tests
│       └── test_registry_messages.cpp # Registry message tests
└── tests/
    └── integration/
        ├── run_all.sh              # Integration test runner
        ├── test_python_ping_pong.sh
        ├── test_python_remote.sh
        ├── test_registry.sh
        └── test_rust_ping_pong.sh
```

## Running Tests

### All Tests

```bash
./run_tests.sh
```

### By Language/Category

```bash
./run_tests.sh python       # Python unit tests only
./run_tests.sh rust         # Rust unit tests only
./run_tests.sh cpp          # C++ unit tests only
./run_tests.sh integration  # Integration tests only
```

### Individual Test Suites

```bash
# Python (from ~/actors/python)
python3 -m pytest tests/ -v

# Rust (from ~/actors/rust)
cargo test --lib

# C++ (from ~/actors/cpp)
make test

# Integration (from ~/actors/tests/integration)
./run_all.sh
```

## Prerequisites

### Python Tests
- Python 3.9+
- pytest: `pip3 install pytest pytest-timeout`
- pyzmq: `pip3 install pyzmq`

### Rust Tests
- Rust toolchain (rustc, cargo)
- Dependencies installed via Cargo.toml

### C++ Tests
- C++20 compiler (g++ or clang++)
- Google Test: Usually `libgtest-dev` or built from source
- Boost: `libboost-dev` (for circular_buffer)

### Integration Tests
- All Python and Rust prerequisites
- Available ports: 5001, 5002, 5555

## Writing Tests

### Python Tests

Tests use pytest. Create test files in `python/tests/` following the naming convention `test_*.py`.

```python
# python/tests/test_example.py
import pytest
from actors.actor import Actor

class TestMyFeature:
    def test_basic_functionality(self):
        """Description of what this tests."""
        actor = Actor()
        assert actor._running is True

    def test_edge_case(self):
        """Test edge case behavior."""
        # ...
```

Run single test file:
```bash
python3 -m pytest tests/test_example.py -v
```

Run single test:
```bash
python3 -m pytest tests/test_example.py::TestMyFeature::test_basic_functionality -v
```

### Rust Tests

Tests are inline in source files using `#[cfg(test)]` modules.

```rust
// In src/mymodule.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

Run specific test:
```bash
cargo test test_basic_functionality
```

### C++ Tests

Tests use Google Test. Create test files in `cpp/tests/` following the naming convention `test_*.cpp`.

```cpp
// cpp/tests/test_example.cpp
#include <gtest/gtest.h>
#include "actors/Message.hpp"

TEST(ExampleTest, BasicFunctionality) {
    // Test code here
    EXPECT_EQ(1 + 1, 2);
}

TEST(ExampleTest, EdgeCase) {
    EXPECT_TRUE(true);
}
```

Run all C++ tests:
```bash
cd ~/actors/cpp && make test
```

### Integration Tests

Integration tests are bash scripts in `tests/integration/`.

```bash
#!/bin/bash
# tests/integration/test_my_feature.sh
set -e

echo "=== Testing My Feature ==="

# Setup
# ...

# Run test
output=$(timeout 10 python3 my_script.py 2>&1) || {
    echo "FAIL: Script failed"
    exit 1
}

# Verify
if echo "$output" | grep -q "expected string"; then
    echo "PASS: Test passed"
    exit 0
else
    echo "FAIL: Expected output not found"
    exit 1
fi
```

Integration test guidelines:
1. Name files `test_*.sh`
2. Use `set -e` for fail-fast behavior
3. Clean up background processes on exit
4. Use `timeout` for commands that might hang
5. Print PASS/FAIL for clear results

## Test Coverage

| Component | Python | Rust | C++ | Integration |
|-----------|--------|------|-----|-------------|
| Actor | ✓ | ✓ | - | ✓ |
| Envelope | ✓ | ✓ | - | - |
| LocalActorRef | ✓ | - | - | - |
| Manager | ✓ | ✓ | - | ✓ |
| Message | - | - | ✓ | - |
| BQueue | - | - | ✓ | - |
| Serialization | ✓ | ✓ | - | - |
| Registry Messages | ✓ | ✓ | ✓ | - |
| GlobalRegistry | ✓ | - | - | ✓ |
| RegistryClient | - | ✓ | - | ✓ |
| ZMQ Remote | - | - | - | ✓ |

## Continuous Integration

The `run_tests.sh` script returns:
- Exit code 0: All tests passed
- Exit code 1: One or more tests failed

This makes it suitable for CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Run tests
  run: ./run_tests.sh
```

## Troubleshooting

### Port Already in Use
Integration tests use ports 5001, 5002, 5555. If tests fail with "Address already in use":
```bash
# Find and kill processes using these ports
lsof -i :5001 -i :5002 -i :5555
```

### Pytest Not Found
```bash
pip3 install pytest pytest-timeout --user
```

### Cargo Not Found
Install Rust from https://rustup.rs/
