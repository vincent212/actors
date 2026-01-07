# Actors Framework Examples

## Registry Examples (Cross-Process Communication)

Located in `~/actors/registry/examples/`

**This is the primary way to enable actors in different processes to find and communicate with each other.**

Available in C++, Python, and Rust - all implementations are interoperable.

| Example | Language | Description |
|---------|----------|-------------|
| `registry_pong.cpp` | C++ | Server - registers "pong" actor with GlobalRegistry |
| `registry_ping.cpp` | C++ | Client - looks up "pong" via GlobalRegistry, sends Ping |
| `registry_pong.py` | Python | Server - registers "pong" actor with GlobalRegistry |
| `registry_ping.py` | Python | Client - looks up "pong" via GlobalRegistry, sends Ping |
| `registry_pong.rs` | Rust | Server - registers "pong" actor with GlobalRegistry |
| `registry_ping.rs` | Rust | Client - looks up "pong" via GlobalRegistry, sends Ping |

### Running the Registry Examples

**Step 1: Start GlobalRegistry (Terminal 1)**
```bash
cd ~/actors/python
python -m actors.registry
```

**Step 2: Start Pong Server (Terminal 2) - choose any language:**
```bash
# C++
cd ~/actors/cpp && make && cd ~/actors/registry/examples && make
./registry_pong

# Python
python ~/actors/registry/examples/registry_pong.py

# Rust
cd ~/actors/rust && cargo run --example registry_pong
```

**Step 3: Start Ping Client (Terminal 3) - choose any language:**
```bash
# C++
./registry_ping

# Python
python ~/actors/registry/examples/registry_ping.py

# Rust
cd ~/actors/rust && cargo run --example registry_ping
```

You can mix languages (e.g., Python pong with Rust ping) - they're fully interoperable.

See `~/actors/registry/examples/README.md` for detailed setup and troubleshooting.

---

## C++ Examples

Located in `~/actors/cpp/examples/`

### Local Examples

| Example | Description |
|---------|-------------|
| `ping_pong.cpp` | Basic ping-pong between two local actors in the same process |

### Remote Examples (ZMQ, no registry)

| Example | Description |
|---------|-------------|
| `remote_pong.cpp` | Server - receives Ping via ZMQ, sends Pong back |
| `remote_ping.cpp` | Client - sends Ping via ZMQ, receives Pong |

### Build and Run

```bash
cd ~/actors/cpp
make examples

# Local ping-pong
./examples/ping_pong

# Remote ping-pong (2 terminals)
./examples/remote_pong   # Terminal 1
./examples/remote_ping   # Terminal 2
```

Note: Registry examples are in `~/actors/registry/examples/` - see above.

---

## Interop Examples (C++ + Rust FFI)

Located in `~/actors/interop/examples/`

| Example | Description |
|---------|-------------|
| `ping_pong/` | C++ ping, Rust pong - same process FFI |
| `pubsub/` | C++ subscribes to Rust publisher |
| `rust_ping_cpp_pong/` | Rust ping, C++ pong - same process FFI |
| `rust_subscribes_cpp_publisher/` | Rust subscribes to C++ publisher |

---

## Python Examples

Located in `~/actors/python/examples/`

| Example | Description |
|---------|-------------|
| `ping_pong.py` | Basic ping-pong between two local actors |
| `timer_example.py` | Timer-based actor scheduling |
| `remote_ping_pong/` | Remote ping-pong via ZMQ |
| `remote_two_pings/` | Two ping actors communicating with one pong |
| `reject_example/` | Message rejection handling |

```bash
cd ~/actors/python/examples
python ping_pong.py
python timer_example.py
```

---

## Rust Examples

Located in `~/actors/rust/examples/`

| Example | Description |
|---------|-------------|
| `ping_pong.rs` | Basic ping-pong between two local actors |
| `group_workers.rs` | Worker group pattern |
| `timer_example.rs` | Timer-based actor scheduling |
| `remote_ping_pong/` | Remote ping-pong via ZMQ |
| `remote_rust_python/` | Rust actor communicating with Python actor |
| `reject_example/` | Message rejection handling |

```bash
cd ~/actors/rust
cargo run --example ping_pong
cargo run --example timer_example
cargo run --example group_workers
```
