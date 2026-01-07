# Remote Actors Guide

This guide covers remote actor communication in the Rust actors framework, including Rust-to-Rust and Rust-to-Python interoperability.

## Actor Lifecycle (Detailed)

Understanding the actor lifecycle is essential. Here's a step-by-step breakdown:

```rust
fn main() {
    // 1. CREATE MANAGER
    // Manager is the orchestrator - it owns all actors and manages their lifecycle.
    // It maintains a registry of actor names -> ActorRefs.
    let mut mgr = Manager::new();

    // 2. GET MANAGER HANDLE
    // ManagerHandle is a lightweight, cloneable reference to the Manager.
    // Actors use it to call terminate() when they want to shut down the system.
    // The handle uses an atomic flag internally - calling terminate() sets it,
    // and mgr.run() checks this flag to know when to stop.
    let handle = mgr.get_handle();

    // 3. CREATE AND REGISTER ACTORS
    // mgr.manage() does several things:
    //   a) Takes ownership of the actor (Box<dyn Actor>)
    //   b) Creates an mpsc channel - this is the actor's "mailbox"
    //      - Sender side: cloned into ActorRef for sending messages
    //      - Receiver side: stored in ActorRuntime for processing messages
    //   c) Spawns a dedicated thread for the actor (based on ThreadConfig)
    //   d) Returns an ActorRef - a lightweight handle containing:
    //      - The sender half of the channel
    //      - The actor's name (for debugging/routing)
    //
    // IMPORTANT: Order matters! Create actors that receive messages FIRST,
    // so their ActorRef can be passed to actors that send messages.
    let pong_ref = mgr.manage("pong", Box::new(PongActor), ThreadConfig::default());

    // Now create PingActor, passing it:
    //   - pong_ref: so it knows where to send Ping messages
    //   - handle: so it can call terminate() when done
    let ping_actor = PingActor::new(pong_ref, handle);
    let _ping_ref = mgr.manage("ping", Box::new(ping_actor), ThreadConfig::default());

    // 4. INITIALIZE ACTORS
    // mgr.init() sends a Start message to every registered actor.
    // Each actor's thread is already running and waiting for messages.
    // The Start message triggers on_start() handlers.
    // This is where actors typically send their first messages.
    mgr.init();

    // 5. RUN THE SYSTEM
    // mgr.run() blocks the main thread, waiting for terminate() to be called.
    // Meanwhile, actor threads are processing messages independently.
    // When any actor calls manager_handle.terminate(), the atomic flag is set,
    // and run() returns.
    mgr.run();

    // 6. SHUTDOWN
    // mgr.end() sends a Shutdown message to all actors and waits for
    // their threads to finish (joins them). This ensures clean shutdown.
    mgr.end();
}
```

### Message Flow Diagram

```
                    ┌─────────────────────────────────────────────────────────┐
                    │                      MANAGER                            │
                    │  - Owns all actors                                      │
                    │  - Spawns threads                                       │
                    │  - Sends Start/Shutdown                                 │
                    └─────────────────────────────────────────────────────────┘
                                              │
                         ┌────────────────────┼────────────────────┐
                         │                    │                    │
                         ▼                    ▼                    ▼
              ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
              │   Actor Thread   │  │   Actor Thread   │  │   Actor Thread   │
              │  ┌────────────┐  │  │  ┌────────────┐  │  │  ┌────────────┐  │
              │  │  Mailbox   │  │  │  │  Mailbox   │  │  │  │  Mailbox   │  │
              │  │  (mpsc rx) │  │  │  │  (mpsc rx) │  │  │  │  (mpsc rx) │  │
              │  └────────────┘  │  │  └────────────┘  │  │  └────────────┘  │
              │        │         │  │        │         │  │        │         │
              │        ▼         │  │        ▼         │  │        ▼         │
              │  ┌────────────┐  │  │  ┌────────────┐  │  │  ┌────────────┐  │
              │  │   Actor    │  │  │  │   Actor    │  │  │  │   Actor    │  │
              │  │  instance  │  │  │  │  instance  │  │  │  │  instance  │  │
              │  └────────────┘  │  │  └────────────┘  │  │  └────────────┘  │
              └──────────────────┘  └──────────────────┘  └──────────────────┘

    ActorRef contains:                   Message send:
    ┌─────────────────┐                  actor_ref.send(msg, sender)
    │ name: "pong"    │                        │
    │ sender: mpsc tx │ ◄──────────────────────┘
    └─────────────────┘        (puts msg in target's mailbox)
```

### What ActorRef.send() Does

```rust
// When you call:
pong_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());

// Internally it:
// 1. Wraps the message in an Envelope with sender info
// 2. Sends through the mpsc channel (non-blocking)
// 3. The target actor's thread receives it from its mailbox
// 4. Calls actor.process_message(msg, ctx)
// 5. If using handle_messages!, dispatches to the right handler
```

## Overview

Remote actors communicate via ZeroMQ (ZMQ) using a JSON wire protocol. The framework uses:
- **PUSH/PULL sockets** for reliable message delivery
- **JSON serialization** for cross-language compatibility
- **Pure-Rust zeromq crate** (no C dependencies)

## Quick Start

### Rust-to-Rust Communication

**Terminal 1 - Start Pong (receiver):**
```bash
cargo run --example remote_pong
```

**Terminal 2 - Start Ping (sender):**
```bash
cargo run --example remote_ping
```

### Rust-to-Python Communication

**Terminal 1 - Start Python Pong:**
```bash
cd /home/vm/actors-py/examples/remote_ping_pong
python3 pong_process.py
```

**Terminal 2 - Start Rust Ping:**
```bash
cargo run --example rust_ping
```

Or vice versa:

**Terminal 1 - Start Rust Pong:**
```bash
cargo run --example rust_pong
```

**Terminal 2 - Start Python Ping:**
```bash
cd /home/vm/actors-py/examples/remote_ping_pong
python3 ping_process.py
```

## Wire Protocol

Messages are serialized as JSON with this structure:

```json
{
    "sender_actor": "ping",
    "sender_endpoint": "tcp://localhost:5002",
    "receiver": "pong",
    "message_type": "Ping",
    "message": {"count": 1}
}
```

| Field | Description |
|-------|-------------|
| `sender_actor` | Name of the sending actor (for replies) |
| `sender_endpoint` | ZMQ endpoint of the sender process |
| `receiver` | Name of the target actor |
| `message_type` | Message class/struct name (e.g., "Ping") |
| `message` | JSON object with message fields |

## Setting Up Remote Actors (Rust)

### 1. Define Messages

Messages must derive `Serialize` and `Deserialize` from serde:

```rust
use serde::{Deserialize, Serialize};
use actors::{Message, define_message};

#[derive(Serialize, Deserialize)]
struct Ping {
    count: i32,
}
define_message!(Ping);

#[derive(Serialize, Deserialize)]
struct Pong {
    count: i32,
}
define_message!(Pong);
```

### 2. Register Messages

Messages must be registered for serialization. **Always use class names** (like `"Ping"`, `"Pong"`) - this ensures interoperability with Python and C++:

```rust
use actors::register_remote_message;

fn register_messages() {
    // Register with class names (NOT "Message_100" - use actual class names!)
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");
}
```

**Important:** Call `register_messages()` before any remote communication.

### 3. Create ZMQ Components

```rust
use std::sync::Arc;
use actors::{ZmqSender, ZmqReceiver};

// Create sender (used for outgoing messages and reply routing)
// ZmqSender runs on its own thread - sends never block the caller
let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5001"));

// Create receiver (binds to listen for incoming messages)
let zmq_receiver = ZmqReceiver::new("tcp://0.0.0.0:5001", Arc::clone(&zmq_sender));
```

**Endpoint formats:**
- Bind (receiver): `tcp://0.0.0.0:PORT` or `tcp://*:PORT`
- Connect (sender): `tcp://localhost:PORT` or `tcp://HOSTNAME:PORT`

### 4. Create Remote Actor Reference

```rust
// Reference to actor "pong" on another process at port 5001
let remote_pong = zmq_sender.remote_ref("pong", "tcp://localhost:5001");

// Convert to ActorRef for use with send()
let pong_ref: ActorRef = remote_pong.into_actor_ref();
```

### 5. Register Local Actors

Local actors that should receive remote messages must be registered:

```rust
let pong_ref = mgr.manage("pong", Box::new(PongActor::new()), ThreadConfig::default());

// Register with ZMQ receiver so remote messages reach this actor
zmq_receiver.register("pong", pong_ref);
```

### 6. Start the Receiver

```rust
// Start receiver thread (must be done before mgr.init())
let mut receiver_handle = zmq_receiver.start();

// Small delay to ensure socket is bound
std::thread::sleep(std::time::Duration::from_millis(100));

mgr.init();
mgr.run();

// Cleanup
receiver_handle.stop();
mgr.end();
```

## Complete Example: Rust Pong Process

```rust
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use actors::{
    ActorContext, Manager, ThreadConfig,
    ZmqReceiver, ZmqSender, register_remote_message,
    define_message, handle_messages,
};

#[derive(Serialize, Deserialize, Default)]
struct Ping { count: i32 }
define_message!(Ping);

#[derive(Serialize, Deserialize, Default)]
struct Pong { count: i32 }
define_message!(Pong);

fn register_messages() {
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");
}

struct PongActor;

// handle_messages! generates the Actor trait implementation
// It dispatches messages to handler methods based on type
handle_messages!(PongActor,
    Ping => on_ping
);

impl PongActor {
    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        println!("Received ping {}", msg.count);
        ctx.reply(Box::new(Pong { count: msg.count }));
    }
}

fn main() {
    register_messages();

    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5001"));
    let zmq_receiver = ZmqReceiver::new("tcp://0.0.0.0:5001", Arc::clone(&zmq_sender));

    // Manager orchestrates actor lifecycle (creation, threading, shutdown)
    let mut mgr = Manager::new();

    // mgr.manage() does three things:
    // 1. Creates a dedicated thread for the actor
    // 2. Sets up an mpsc channel (mailbox) for receiving messages
    // 3. Returns an ActorRef - a lightweight handle for sending messages to the actor
    let pong_ref = mgr.manage("pong", Box::new(PongActor), ThreadConfig::default());

    // zmq_receiver.register() maps the name "pong" to the ActorRef
    // When a remote message arrives with receiver="pong", ZmqReceiver
    // looks up this mapping and forwards the message to pong_ref
    zmq_receiver.register("pong", pong_ref);

    // zmq_receiver.start() spawns a background thread that:
    // 1. Binds a ZMQ PULL socket to receive incoming messages
    // 2. Deserializes JSON messages and routes them to registered actors
    // 3. Returns a handle to stop the receiver later
    let mut receiver_handle = zmq_receiver.start();

    // Small delay to ensure ZMQ socket is bound before sending messages
    thread::sleep(Duration::from_millis(100));

    // mgr.init() sends a Start message to all managed actors
    mgr.init();
    println!("Pong ready on port 5001...");

    // mgr.run() blocks until manager.terminate() is called
    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();
}
```

## Complete Example: Rust Ping Process

```rust
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use actors::{
    ActorContext, ActorRef, Manager, ManagerHandle,
    Start, ThreadConfig, ZmqReceiver, ZmqSender,
    register_remote_message, define_message, handle_messages,
};

#[derive(Serialize, Deserialize, Default)]
struct Ping { count: i32 }
define_message!(Ping);

#[derive(Serialize, Deserialize, Default)]
struct Pong { count: i32 }
define_message!(Pong);

fn register_messages() {
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");
}

struct PingActor {
    pong_ref: ActorRef,
    manager_handle: ManagerHandle,
}

handle_messages!(PingActor,
    Start => on_start,
    Pong => on_pong
);

impl PingActor {
    fn new(pong_ref: ActorRef, manager_handle: ManagerHandle) -> Self {
        PingActor { pong_ref, manager_handle }
    }

    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("Starting ping-pong");
        self.pong_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("Received pong {}", msg.count);
        if msg.count >= 5 {
            println!("Done!");
            self.manager_handle.terminate();
        } else {
            self.pong_ref.send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
        }
    }
}

fn main() {
    register_messages();

    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5002"));
    let remote_pong = zmq_sender.remote_ref("pong", "tcp://localhost:5001");

    let mut mgr = Manager::new();
    let handle = mgr.get_handle();

    let ping_ref = mgr.manage(
        "ping",
        Box::new(PingActor::new(remote_pong.into_actor_ref(), handle)),
        ThreadConfig::default()
    );

    let zmq_receiver = ZmqReceiver::new("tcp://0.0.0.0:5002", Arc::clone(&zmq_sender));
    zmq_receiver.register("ping", ping_ref);
    let mut receiver_handle = zmq_receiver.start();

    thread::sleep(Duration::from_millis(100));

    mgr.init();
    println!("Ping starting...");
    mgr.run();

    receiver_handle.stop();
    mgr.end();
}
```

## Setting Up Remote Actors (Python)

### 1. Define and Register Messages

```python
from actors import register_message

@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count

@register_message
class Pong:
    def __init__(self, count: int):
        self.count = count
```

### 2. Create ZMQ Components

```python
from actors import Manager, ZmqSender, ZmqReceiver

ENDPOINT = "tcp://*:5001"

mgr = Manager(endpoint=ENDPOINT)
zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5001")
zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)
```

### 3. Create Remote Actor Reference

```python
from actors import RemoteActorRef

# Reference to actor "ping" on Rust process at port 5002
remote_ping = RemoteActorRef("ping", "tcp://localhost:5002", zmq_sender)
```

### 4. Set Up Manager

```python
mgr.manage("zmq_receiver", zmq_receiver)
mgr.manage("pong", PongActor())

mgr.init()
mgr.run()
mgr.end()
```

## Python Interoperability

### Message Name Matching (Critical)

**The `message_type` field must match exactly between Rust and Python.** This is case-sensitive and must be identical on both sides.

When a message is sent over the wire, it includes the type name:
```json
{"message_type": "Ping", "message": {"count": 1}, ...}
```

The receiving process uses `message_type` to look up the correct deserializer. If names don't match:
- **Python**: `Unknown message type: Ping`
- **Rust**: `Message type 'Ping' not registered` panic

Example of matching registration:

| Rust | Python |
|------|--------|
| `register_remote_message::<Ping>("Ping")` | `@register_message class Ping:` |

### Field Naming

Message fields are serialized as JSON object keys. Ensure field names match:

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
struct Ping {
    count: i32,      // JSON: {"count": 1}
}
```

**Python:**
```python
class Ping:
    def __init__(self, count: int):
        self.count = count  # JSON: {"count": 1}
```

### Port Convention

By convention in the examples:
- **Pong process**: Port 5001
- **Ping process**: Port 5002

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Rust Process A                           │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   PingActor  │───▶│  ZmqSender   │───▶│ PUSH Socket  │──────┼──┐
│  └──────────────┘    └──────────────┘    └──────────────┘      │  │
│         ▲                                                       │  │
│         │            ┌──────────────┐    ┌──────────────┐      │  │
│         └────────────│ ZmqReceiver  │◀───│ PULL Socket  │◀─────┼──┼──┐
│                      └──────────────┘    └──────────────┘      │  │  │
│                           Port 5002                             │  │  │
└─────────────────────────────────────────────────────────────────┘  │  │
                                                                     │  │
                              JSON over TCP                          │  │
                                                                     │  │
┌─────────────────────────────────────────────────────────────────┐  │  │
│                   Python/Rust Process B                         │  │  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │  │  │
│  │   PongActor  │───▶│  ZmqSender   │───▶│ PUSH Socket  │──────┼──┼──┘
│  └──────────────┘    └──────────────┘    └──────────────┘      │  │
│         ▲                                                       │  │
│         │            ┌──────────────┐    ┌──────────────┐      │  │
│         └────────────│ ZmqReceiver  │◀───│ PULL Socket  │◀─────┼──┘
│                      └──────────────┘    └──────────────┘      │
│                           Port 5001                             │
└─────────────────────────────────────────────────────────────────┘
```

## Troubleshooting

### "Address already in use"

Kill existing processes:
```bash
fuser -k 5001/tcp
fuser -k 5002/tcp
```

### Messages not arriving

1. **Check port numbers** - sender connects to receiver's port
2. **Check actor names** - receiver name must match `remote_ref("name", ...)`
3. **Check message registration** - both ends must register the same message types
4. **Check message type names** - must match exactly (case-sensitive)

### Python: "Unknown message type"

Ensure the message class is decorated with `@register_message`:
```python
@register_message  # Don't forget this!
class Ping:
    ...
```

### Rust: Serialization panic

Ensure message registration is done:
```rust
register_remote_message::<Ping>("Ping");
```

### Connection timing

Start the receiver (pong) before the sender (ping). Add a small delay after starting the receiver:
```rust
let mut receiver_handle = zmq_receiver.start();
thread::sleep(Duration::from_millis(100));  // Let socket bind
mgr.init();
```

## API Reference

### ZmqSender

`ZmqSender` runs on its own dedicated thread. Sends are async and never block the caller - messages are serialized on the caller's thread and queued to the sender thread for ZMQ transmission.

```rust
impl ZmqSender {
    /// Create a new sender with local endpoint for reply routing.
    /// Spawns a dedicated sender thread for async sending.
    pub fn new(local_endpoint: &str) -> Self;

    /// Create a remote actor reference
    pub fn remote_ref(&self, name: &str, endpoint: &str) -> RemoteActorRef;

    /// Send message to endpoint/actor (async - returns immediately)
    /// Message is serialized on caller's thread, then queued to sender thread.
    pub fn send_to(&self, endpoint: &str, actor: &str, msg: Box<dyn Message>, sender: Option<ActorRef>);
}
```

### ZmqReceiver

```rust
impl ZmqReceiver {
    /// Create a new receiver bound to endpoint
    pub fn new(bind_endpoint: &str, zmq_sender: Arc<ZmqSender>) -> Self;

    /// Register a local actor to receive remote messages
    pub fn register(&self, name: &str, actor_ref: ActorRef);

    /// Start the receiver thread
    pub fn start(&self) -> ZmqReceiverHandle;
}
```

### RemoteActorRef

```rust
impl RemoteActorRef {
    /// Send a message to this remote actor
    pub fn send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>);

    /// Convert to ActorRef enum
    pub fn into_actor_ref(self) -> ActorRef;
}
```

### Message Registration

```rust
/// Register message type for remote serialization/deserialization
pub fn register_remote_message<M: Message + Serialize + DeserializeOwned>(type_name: &str);
```

## Error Handling with Reject Messages

When a remote message cannot be processed (unknown message type, actor not found, deserialization failure), the framework automatically sends a `Reject` message back to the sender.

### The Reject Message

```rust
use actors::Reject;

// Reject contains:
// - message_type: The type name that was rejected (e.g., "UnknownMessage")
// - reason: Why it was rejected (e.g., "Unknown message type: UnknownMessage")
// - rejected_by: The actor that rejected it (e.g., "receiver")
```

### Handling Reject in Your Actor

To receive reject notifications, register the `Reject` message and add a handler:

```rust
use actors::{
    Reject, register_remote_message, handle_messages,
};

fn register_messages() {
    // ... your other messages ...

    // Register Reject for remote communication
    register_remote_message::<Reject>("Reject");
}

struct MyActor {
    // ...
}

handle_messages!(MyActor,
    Start => on_start,
    // ... your other handlers ...
    Reject => on_reject  // Handle rejection notifications
);

impl MyActor {
    fn on_reject(&mut self, msg: &Reject, _ctx: &mut ActorContext) {
        println!("Message rejected!");
        println!("  Type: {}", msg.message_type);
        println!("  Reason: {}", msg.reason);
        println!("  Rejected by: {}", msg.rejected_by);
        // Handle the error appropriately (retry, log, fallback, etc.)
    }
}
```

### Rejection Scenarios

The framework sends a `Reject` message when:

1. **Unknown message type** - The receiver doesn't have a deserializer for the message type
2. **Actor not found** - The target actor name is not registered with the ZmqReceiver
3. **Deserialization failure** - The message data doesn't match the expected structure

### Example: Reject Flow

```
Sender Process                          Receiver Process
      │                                        │
      │  ── UnknownMessage ──────────────▶    │
      │      {"message_type": "Unknown"}       │
      │                                        │
      │                               (Lookup fails:
      │                                "Unknown" not registered)
      │                                        │
      │  ◀─────────────── Reject ────────     │
      │      {"message_type": "Unknown",       │
      │       "reason": "Unknown message...",  │
      │       "rejected_by": "receiver"}       │
      │                                        │
      ▼                                        ▼
 on_reject() called                     (continues normally)
```

### Running the Reject Example

**Terminal 1 - Start Receiver (only knows Ping/Pong):**
```bash
cargo run --example reject_receiver
```

**Terminal 2 - Start Sender (sends UnknownMessage):**
```bash
cargo run --example reject_sender
```

Expected output:
```
# Sender output:
SenderActor: Starting test...
SenderActor: Sending UnknownMessage (should be rejected)...
SenderActor: Sending Ping (should succeed)...
SenderActor: Received Reject!
  - Message type: UnknownMessage
  - Reason: Unknown message type: UnknownMessage
  - Rejected by: receiver
SenderActor: Received Pong 1 - normal message worked!
SenderActor: Test complete!

# Receiver output:
ReceiverActor: Received Ping 1
ReceiverActor: Sent Pong 1 back
```

### Python Interoperability

The `Reject` message is also supported in Python. When Python rejects a message, Rust can receive it (and vice versa). The wire format is:

```json
{
    "message_type": "Reject",
    "message": {
        "message_type": "UnknownMessage",
        "reason": "Unknown message type: UnknownMessage. Did you register it with @register_message?",
        "rejected_by": "receiver"
    }
}
```

Python code to handle Reject:
```python
from actors import Reject

class MyActor(Actor):
    def on_reject(self, msg: Reject, ctx):
        print(f"Rejected: {msg.message_type}")
        print(f"Reason: {msg.reason}")
        print(f"Rejected by: {msg.rejected_by}")

    def receive(self, msg, ctx):
        if isinstance(msg, Reject):
            self.on_reject(msg, ctx)
        # ... other handlers
```

## Examples

| Example | Description | Files |
|---------|-------------|-------|
| `remote_ping` | Rust ping process | `examples/remote_ping_pong/ping.rs` |
| `remote_pong` | Rust pong process | `examples/remote_ping_pong/pong.rs` |
| `rust_ping` | Rust ping (for Python pong) | `examples/remote_rust_python/rust_ping.rs` |
| `rust_pong` | Rust pong (for Python ping) | `examples/remote_rust_python/rust_pong.rs` |
| `reject_sender` | Demonstrates sending unknown messages | `examples/reject_example/sender.rs` |
| `reject_receiver` | Demonstrates automatic rejection | `examples/reject_example/receiver.rs` |
| `timer_example` | Periodic and one-shot timers | `examples/timer_example.rs` |

Run examples:
```bash
cargo run --example remote_pong   # Start first
cargo run --example remote_ping   # Start second
```

Run reject example:
```bash
cargo run --example reject_receiver   # Start first
cargo run --example reject_sender     # Start second
```

## Timers

The actor framework provides a `Timer` for scheduling delayed and periodic messages to actors.

### Timer Types

- **One-shot timer**: Fires once after a specified delay
- **Periodic timer**: Fires repeatedly at fixed intervals

### Using Timers

```rust
use actors::{Timer, Timeout, handle_messages};
use std::time::Duration;

struct MyActor {
    periodic_timer: Option<Timer>,
    countdown_timer: Option<Timer>,
}

handle_messages!(MyActor,
    Start => on_start,
    Timeout => on_timeout
);

impl MyActor {
    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        // Create a periodic timer that fires every 500ms
        self.periodic_timer = Some(Timer::periodic(
            ctx.self_ref().unwrap(),
            Duration::from_millis(500),
            1  // timer_id to distinguish multiple timers
        ));

        // Create a one-shot timer that fires after 2.5 seconds
        self.countdown_timer = Some(Timer::once(
            ctx.self_ref().unwrap(),
            Duration::from_millis(2500),
            2  // different timer_id
        ));
    }

    fn on_timeout(&mut self, msg: &Timeout, _ctx: &mut ActorContext) {
        match msg.id {
            1 => println!("Periodic tick!"),
            2 => println!("Countdown complete!"),
            _ => println!("Unknown timer"),
        }
    }
}
```

### Cancelling Timers

Timers can be cancelled by dropping them or explicitly:

```rust
// Drop cancels the timer automatically
self.periodic_timer = None;

// Or take ownership and let it drop
if let Some(timer) = self.periodic_timer.take() {
    drop(timer);
}
```

### Timer API

```rust
impl Timer {
    /// Create a one-shot timer that fires once after delay
    pub fn once(target: ActorRef, delay: Duration, id: u64) -> Timer;

    /// Create a periodic timer that fires at fixed intervals
    pub fn periodic(target: ActorRef, interval: Duration, id: u64) -> Timer;
}

/// Message sent when timer fires
pub struct Timeout {
    pub id: u64,  // Identifies which timer fired
}
```

### Running the Timer Example

```bash
cargo run --example timer_example
```

Expected output:
```
=== Timer Example ===

TimerActor: Starting timers...
  - Periodic timer (id=1): every 500ms
  - One-shot timer (id=2): fires after 2.5s

TimerActor: Periodic tick #1
TimerActor: Periodic tick #2
TimerActor: Periodic tick #3
TimerActor: Periodic tick #4
TimerActor: *** COUNTDOWN COMPLETE! One-shot timer fired! ***
TimerActor: Periodic tick #5
...
TimerActor: Periodic tick #10
TimerActor: Max ticks reached, cancelling periodic timer
TimerActor: Shutting down...

=== Example Complete ===
```

### Python Timer

The Python framework also has a Timer with the same API:

```python
from actors import Timer, Timeout, next_timer_id

class MyActor(Actor):
    def on_start(self, env):
        # Generate unique timer IDs
        self.periodic_id = next_timer_id()
        self.countdown_id = next_timer_id()

        # Start timers
        self.periodic_timer = Timer.periodic(
            self._actor_ref, interval=0.5, timer_id=self.periodic_id
        )
        self.countdown_timer = Timer.once(
            self._actor_ref, delay=2.5, timer_id=self.countdown_id
        )

    def on_timeout(self, env):
        if env.msg.id == self.periodic_id:
            print("Periodic tick!")
        elif env.msg.id == self.countdown_id:
            print("Countdown complete!")

    def cancel_timer(self):
        if self.periodic_timer:
            self.periodic_timer.cancel()
            self.periodic_timer = None
```

Run the Python timer example:
```bash
cd /home/vm/actors-py
python3 examples/timer_example.py
```
