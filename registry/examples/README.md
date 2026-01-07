# Registry Examples

End-to-end examples showing how to use the GlobalRegistry for cross-process actor communication.

Examples are available in **C++**, **Python**, and **Rust**. All implementations are interoperable - you can mix and match languages (e.g., C++ pong with Python ping).

## Quick Start

### 1. Start GlobalRegistry (Terminal 1)

```bash
cd ~/actors/python
python -m actors.registry
```

You should see:
```
2025-01-01 12:00:00 [INFO] actors.registry: Starting GlobalRegistry on tcp://0.0.0.0:5555
2025-01-01 12:00:00 [INFO] actors.registry: GlobalRegistry ready, waiting for messages...
```

### 2. Start Pong Server (Terminal 2)

Choose one implementation:

**C++:**
```bash
cd ~/actors/cpp && make
cd ~/actors/registry/examples && make
./registry_pong
```

**Python:**
```bash
python ~/actors/registry/examples/registry_pong.py
```

**Rust:**
```bash
cd ~/actors/rust
cargo run --example registry_pong
```

### 3. Start Ping Client (Terminal 3)

Choose one implementation (can be different language from pong):

**C++:**
```bash
./registry_ping
```

**Python:**
```bash
python ~/actors/registry/examples/registry_ping.py
```

**Rust:**
```bash
cd ~/actors/rust
cargo run --example registry_ping
```

## How It Works

1. **GlobalRegistry** (Python) runs as a central server on port 5555
   - Accepts registrations from Managers
   - Tracks actor name → endpoint mappings
   - Monitors Manager health via heartbeats (2s interval, 6s timeout)

2. **Pong Server** (any language)
   - Creates a `PongActor` that responds to Ping messages
   - Creates a `RegistryClient` to communicate with GlobalRegistry
   - Registers "pong" actor with its ZMQ endpoint
   - Starts heartbeat thread (sends heartbeat every 2s)

3. **Ping Client** (any language)
   - Creates a `RegistryClient` to communicate with GlobalRegistry
   - Looks up "pong" actor by name
   - Gets back the ZMQ endpoint where pong is listening
   - Sends Ping messages and receives Pong replies

## Cross-Language Interoperability

All examples use the same protocol, so you can mix languages:

```bash
# Python pong + Rust ping
python registry_pong.py         # Terminal 2
cargo run --example registry_ping  # Terminal 3

# Rust pong + C++ ping
cargo run --example registry_pong  # Terminal 2
./registry_ping                    # Terminal 3

# C++ pong + Python ping
./registry_pong                    # Terminal 2
python registry_ping.py            # Terminal 3
```

## Custom Registry Endpoint

By default, examples connect to `tcp://localhost:5555`. To use a different endpoint:

```bash
# Start registry on different port
python -m actors.registry --endpoint tcp://0.0.0.0:6666

# C++
./registry_pong tcp://localhost:6666
./registry_ping tcp://localhost:6666

# Python
python registry_pong.py tcp://localhost:6666
python registry_ping.py tcp://localhost:6666

# Rust
cargo run --example registry_pong -- tcp://localhost:6666
cargo run --example registry_ping -- tcp://localhost:6666
```

## Files

| File | Language | Description |
|------|----------|-------------|
| `registry_pong.cpp` | C++ | Server that registers "pong" actor |
| `registry_ping.cpp` | C++ | Client that looks up "pong" and sends pings |
| `registry_pong.py` | Python | Server that registers "pong" actor |
| `registry_ping.py` | Python | Client that looks up "pong" and sends pings |
| `registry_pong.rs` | Rust | Server that registers "pong" actor |
| `registry_ping.rs` | Rust | Client that looks up "pong" and sends pings |
| `Makefile` | - | Build the C++ examples |

## Building

### C++

```bash
cd ~/actors/cpp && make
cd ~/actors/registry/examples && make
```

### Python

No build required. Make sure the `actors` package is in your Python path:
```bash
export PYTHONPATH=~/actors/python:$PYTHONPATH
```

Or the examples add it automatically via `sys.path.insert()`.

### Rust

```bash
cd ~/actors/rust
cargo build --examples
```

## Troubleshooting

### "Connection refused"
- Make sure GlobalRegistry is running first
- Check the endpoint matches between registry and examples

### "pong not found"
- Make sure the pong server is running and registered successfully
- Check GlobalRegistry logs for registration messages

### "Heartbeat timeout"
- The Manager is not sending heartbeats
- Actors are marked offline after 6 seconds without heartbeat
- Restart the Manager process
