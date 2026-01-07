# Global Actor Registry

Cross-process actor lookup with heartbeat-based health monitoring for the actors framework.

## Overview

The Global Actor Registry enables actors in different processes (and different languages) to find and communicate with each other by name. It provides:

- **Name-based lookup**: Find any actor by its registered name
- **Heartbeat monitoring**: Automatic detection of failed managers
- **Multi-language support**: C++, Rust, and Python clients
- **Process control**: Start/stop/restart managers via systemd + SSH

## Quick Start

### 1. Start the GlobalRegistry

```bash
# Install (first time only)
cd ~/actors/registry/systemd
sudo ./install.sh

# Start
sudo systemctl start global-registry

# Check status
sudo systemctl status global-registry
```

### 2. Register an Actor (C++)

```cpp
#include "actors/registry/RegistryClient.hpp"

// Create registry client
auto registry_ref = ActorRef("GlobalRegistry", "tcp://localhost:5555", zmq_sender);
RegistryClient client("MyManager", registry_ref);

// Start heartbeat (required!)
client.start_heartbeat();

// Register your actor
client.register_actor("MyActor", "tcp://localhost:5556");
```

### 3. Look Up an Actor (C++)

```cpp
// Look up by name
try {
    std::string endpoint = client.lookup("OtherActor");
    auto ref = ActorRef("OtherActor", endpoint, zmq_sender);
    ref.send(new MyMessage{...}, this);
} catch (const ActorNotFoundError& e) {
    // Actor not registered
} catch (const ActorOfflineError& e) {
    // Manager missed heartbeats
}
```

### 4. Register and Lookup (Rust)

```rust
use actors_registry::{RegistryClient, RegistryError};

let client = RegistryClient::new("MyManager", registry_ref);
client.start_heartbeat();

// Register
client.register("MyActor", "tcp://localhost:5556")?;

// Lookup
match client.lookup("OtherActor") {
    Ok(endpoint) => { /* use endpoint */ }
    Err(RegistryError::NotFound(_)) => { /* not registered */ }
    Err(RegistryError::Offline(_)) => { /* manager down */ }
}
```

### 5. Register and Lookup (Python)

```python
from actors_registry import RegisterActor, LookupActor, Heartbeat

# Send registration (via ZMQ to GlobalRegistry)
msg = RegisterActor(
    manager_id="MyManager",
    actor_name="MyActor",
    actor_endpoint="tcp://localhost:5556"
)
registry_socket.send(serialize(msg))

# Send heartbeats every 2 seconds
import threading
def heartbeat_loop():
    while running:
        registry_socket.send(serialize(Heartbeat(manager_id="MyManager")))
        time.sleep(2)
threading.Thread(target=heartbeat_loop, daemon=True).start()
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     GlobalRegistry                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ registry: {actor_name → (endpoint, manager_id)}         │ │
│  │ heartbeats: {manager_id → last_heartbeat_time}          │ │
│  └─────────────────────────────────────────────────────────┘ │
│                           │                                  │
│        RegisterActor / LookupActor / Heartbeat               │
│                           │                                  │
└───────────────────────────┼──────────────────────────────────┘
                            │ ZMQ (tcp://*:5555)
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
   ┌─────────┐         ┌─────────┐         ┌─────────┐
   │Manager A│         │Manager B│         │Manager C│
   │  (C++)  │         │ (Rust)  │         │(Python) │
   │         │         │         │         │         │
   │ Actor1  │         │ Actor3  │         │ Actor5  │
   │ Actor2  │         │ Actor4  │         │         │
   └─────────┘         └─────────┘         └─────────┘
```

## Message Protocol

| Message | ID | Direction | Description |
|---------|-----|-----------|-------------|
| RegisterActor | 900 | Manager → Registry | Register an actor |
| UnregisterActor | 901 | Manager → Registry | Remove an actor |
| RegistrationOk | 902 | Registry → Manager | Registration succeeded |
| RegistrationFailed | 903 | Registry → Manager | Registration failed |
| LookupActor | 904 | Manager → Registry | Find an actor by name |
| LookupResult | 905 | Registry → Manager | Lookup response |
| Heartbeat | 906 | Manager → Registry | Health check |
| HeartbeatAck | 907 | Registry → Manager | Heartbeat acknowledged |

## Heartbeat Protocol

- **Interval**: 2 seconds
- **Timeout**: 6 seconds (3 missed heartbeats)
- **On timeout**: All actors from that manager marked offline
- **Recovery**: Actors come back online when heartbeats resume

## Configuration

Create `/etc/actors/registry.json`:

```json
{
  "registry_endpoint": "tcp://0.0.0.0:5555",
  "heartbeat_timeout_s": 6.0,
  "heartbeat_check_interval_s": 1.0,
  "hosts": {
    "server1": {
      "ssh": "actors@192.168.1.10",
      "managers": {
        "OrderProcessor": {
          "service": "manager-order-processor",
          "language": "cpp"
        }
      }
    }
  }
}
```

## Error Handling

### C++

```cpp
try {
    client.register_actor("MyActor", endpoint);
} catch (const RegistrationFailedError& e) {
    std::cerr << "Registration failed: " << e.reason() << std::endl;
}

try {
    std::string ep = client.lookup("Actor");
} catch (const ActorNotFoundError& e) {
    std::cerr << "Not found: " << e.actor_name() << std::endl;
} catch (const ActorOfflineError& e) {
    std::cerr << "Offline: " << e.actor_name() << std::endl;
} catch (const TimeoutError& e) {
    std::cerr << "Timeout: " << e.what() << std::endl;
}
```

### Rust

```rust
match client.lookup("Actor") {
    Ok(endpoint) => { /* success */ }
    Err(RegistryError::NotFound(name)) => { /* not registered */ }
    Err(RegistryError::Offline(name)) => { /* manager down */ }
    Err(RegistryError::Timeout(msg)) => { /* no response */ }
    Err(RegistryError::ConnectionError(msg)) => { /* network error */ }
}
```

## Multi-Host Deployment

See [examples/multi_host/README.md](examples/multi_host/README.md) for deploying across multiple machines.

Quick overview:
```bash
# Start everything
./examples/multi_host/start_system.sh

# Check status
./examples/multi_host/status_system.sh

# Stop everything
./examples/multi_host/stop_system.sh
```

## Examples

| Example | Description |
|---------|-------------|
| [cpp_to_python](examples/cpp_to_python/) | C++ OrderProcessor calls Python RiskChecker |
| [multi_host](examples/multi_host/) | Deploy across multiple machines |

## Directory Structure

```
registry/
├── README.md           # This file
├── ARCHITECTURE.md     # AI-readable architecture doc
├── cpp/                # C++ client library
├── rust/               # Rust client crate
├── python/             # Python GlobalRegistry + messages
├── systemd/            # Service files and install script
├── config/             # Example configuration
└── examples/           # Usage examples
```

## API Reference

### C++ RegistryClient

```cpp
class RegistryClient {
    RegistryClient(const std::string& manager_id, ActorRef registry_ref);

    void start_heartbeat();   // Start background heartbeat thread
    void stop_heartbeat();    // Stop heartbeat thread

    void register_actor(const std::string& name, const std::string& endpoint);
    std::string lookup(const std::string& name);
    std::pair<std::string, bool> lookup_allow_offline(const std::string& name);
};
```

### Rust RegistryClient

```rust
impl RegistryClient {
    pub fn new(manager_id: &str, registry_ref: ActorRef) -> Self;

    pub fn start_heartbeat(&self);
    pub fn stop_heartbeat(&self);

    pub fn register(&self, actor_name: &str, endpoint: &str) -> Result<(), RegistryError>;
    pub fn lookup(&self, actor_name: &str) -> Result<String, RegistryError>;
    pub fn lookup_allow_offline(&self, actor_name: &str) -> Result<(String, bool), RegistryError>;
}
```

### Python GlobalRegistry

```python
class GlobalRegistry:
    def __init__(self, config_path: Optional[str] = None):
        ...

    def run(self):
        """Start the registry (blocks)"""

    def stop(self):
        """Stop the registry"""
```

## License

MIT License - Copyright 2025 Vincent Maciejewski & M2 Tech
