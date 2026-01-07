# actors-interop

FFI interop layer between actors-cpp and actors-rust with **location transparency**.

Actors communicate using `ActorRef` without knowing whether targets are C++, Rust, or remote.

## Quick Start

```bash
# Build everything
make all

# Run examples
cd examples/ping_pong && ./ping_pong
cd examples/pubsub && ./pubsub
cd examples/rust_ping_cpp_pong && ./rust_ping_cpp_pong
cd examples/rust_subscribes_cpp_publisher && ./rust_subscribes_cpp_publisher
```

## Core Principle: Location Transparency

Actors MUST NOT know where other actors live:

```cpp
// C++ Actor - uses ActorRef, doesn't know target is Rust
ActorRef publisher = manager->get_ref("rust_publisher");
publisher.send(new msg::Subscribe{...}, this);
```

```rust
// Rust Actor - uses ActorRef, doesn't know target is C++
let publisher = get_actor_ref("cpp_publisher", "my_actor").unwrap();
publisher.send(Box::new(Subscribe{...}), None);
```

## Examples

| Example | Description |
|---------|-------------|
| `ping_pong` | C++ initiates ping-pong, Rust responds |
| `pubsub` | C++ subscribes to Rust publisher |
| `rust_ping_cpp_pong` | Rust initiates ping-pong, C++ responds |
| `rust_subscribes_cpp_publisher` | Rust subscribes to C++ publisher |

## Adding New Messages

1. Edit `messages/interop_messages.h`:
```c
INTEROP_MESSAGE(MyMessage, 1020)
typedef struct {
    int32_t value;
    char name[32];
} MyMessage;
```

2. Regenerate and rebuild:
```bash
make generate
make all
```

## Type Mapping

| C Type | C++ Type | Rust Type |
|--------|----------|-----------|
| int32_t | int32_t | i32 |
| int64_t | int64_t | i64 |
| double | double | f64 |
| int32_t (bool) | bool | bool |
| char[N] | std::array<char, N> | [u8; N] |
| double[N] | std::array<double, N> | [f64; N] |

## Documentation

See **[ARCHITECTURE.md](ARCHITECTURE.md)** for detailed technical documentation including:
- Directory structure and key files
- ActorRef variants (Local, Remote, Rust/Cpp)
- FFI bridge functions
- Message flow diagrams
- Initialization sequence
- Common patterns and debugging tips

## License

MIT License - see individual source files for copyright.
