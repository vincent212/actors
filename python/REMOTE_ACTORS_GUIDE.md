# Remote Actors Guide

This guide explains how to use actors across multiple processes via ZeroMQ.

## Overview

Remote communication uses:
- **ZmqSender** - Sends messages to remote processes
- **ZmqReceiver** - Receives messages and routes to local actors
- **RemoteActorRef** - ActorRef that sends via ZMQ instead of local queue

## Architecture

```
Process A (port 5001)                    Process B (port 5002)
┌─────────────────────┐                  ┌─────────────────────┐
│  ┌───────────────┐  │                  │  ┌───────────────┐  │
│  │  ZmqReceiver  │◄─┼──── ZMQ ────────┼──│   ZmqSender   │  │
│  └───────────────┘  │                  │  └───────────────┘  │
│  ┌───────────────┐  │                  │  ┌───────────────┐  │
│  │   ZmqSender   │──┼──── ZMQ ────────┼─►│  ZmqReceiver  │  │
│  └───────────────┘  │                  │  └───────────────┘  │
│  ┌───────────────┐  │                  │  ┌───────────────┐  │
│  │   PongActor   │  │                  │  │   PingActor   │  │
│  └───────────────┘  │                  │  └───────────────┘  │
└─────────────────────┘                  └─────────────────────┘
```

## Wire Format

Messages are serialized as JSON:

```json
{
    "sender_actor": "ping",
    "sender_endpoint": "tcp://localhost:5002",
    "receiver": "pong",
    "message_type": "Ping",
    "message": {"count": 1}
}
```

- `sender_actor` - Name of sending actor
- `sender_endpoint` - ZMQ endpoint for replies
- `receiver` - Name of target actor
- `message_type` - Class name for deserialization
- `message` - The message data (dict)

## Message Registration

Messages must be registered for serialization/deserialization:

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

### How @register_message Works

The decorator adds the class to a global registry:

```python
MESSAGE_REGISTRY = {}

def register_message(cls):
    MESSAGE_REGISTRY[cls.__name__] = cls  # "Ping" -> Ping class
    return cls
```

### Serialization (Sending)

When you send a message remotely, it's converted to JSON:

```python
# Your message
msg = Ping(count=5)

# Becomes JSON
{
    "message_type": "Ping",      # class.__name__
    "message": {"count": 5}      # msg.__dict__
}
```

### Deserialization (Receiving)

When receiving, the class is looked up by name and reconstructed:

```python
# Incoming JSON
data = {"message_type": "Ping", "message": {"count": 5}}

# Lookup class by name
cls = MESSAGE_REGISTRY["Ping"]  # -> Ping class

# Reconstruct using kwargs
msg = cls(**data["message"])    # -> Ping(count=5)
```

### Requirements

1. **Register on both sides** - Sender and receiver must both register the same message class
2. **Matching `__init__` params** - Constructor parameter names must match field names:
   ```python
   # GOOD - param name matches field
   class Ping:
       def __init__(self, count: int):
           self.count = count

   # BAD - param name doesn't match what's serialized
   class Ping:
       def __init__(self, c: int):
           self.count = c  # Fails: receives {"count": 5}, expects "c"
   ```
3. **JSON-serializable fields** - Use basic types (int, str, float, bool, list, dict)

## Setting Up Remote Communication

### 1. Create ZmqSender and ZmqReceiver

```python
from actors import Manager, ZmqSender, ZmqReceiver

ENDPOINT = "tcp://*:5001"

mgr = Manager(endpoint=ENDPOINT)
zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5001")
zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)

mgr.manage("zmq_receiver", zmq_receiver)
```

### 2. Create RemoteActorRef

```python
from actors import RemoteActorRef

# Reference to actor on another process
remote_pong = RemoteActorRef(
    name="pong",
    endpoint="tcp://localhost:5001",  # Where pong lives
    zmq_sender=zmq_sender
)
```

### 3. Use Normally

```python
# Send works the same as local
remote_pong.send(Ping(1), self._actor_ref)
```

## Reply Routing

When ZmqReceiver receives a message, it creates a RemoteActorRef for the sender:

```python
# ZmqReceiver does this internally:
sender_ref = RemoteActorRef(
    name=data["sender_actor"],
    endpoint=data["sender_endpoint"],
    zmq_sender=self._zmq_sender
)
local_ref.send(msg, sender=sender_ref)
```

When the local actor calls `reply()`, it uses this RemoteActorRef:

```python
def reply(self, envelope, response):
    if envelope.sender:
        # Polymorphic! Works for Local or Remote
        envelope.sender.send(response, self._actor_ref)
```

The reply is automatically routed back to the correct process.

## Multiple Senders

Multiple processes can send to the same actor. Replies route correctly because each message carries its sender's endpoint:

```
Process A (port 5002)      Process C (port 5001)      Process B (port 5003)
    Ping1 ──────────────────► Pong ◄─────────────────── Ping2
      ▲                                                    ▲
      │                                                    │
      └────── reply routes to 5002 ────┐  ┌── reply routes to 5003 ──┘
                                       │  │
                                    (based on sender_endpoint)
```

## Complete Example: Two Processes

### pong_process.py (port 5001)

```python
from actors import Actor, Envelope, Manager, ZmqSender, ZmqReceiver, register_message

@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count

@register_message
class Pong:
    def __init__(self, count: int):
        self.count = count

class PongActor(Actor):
    def on_ping(self, env: Envelope):
        print(f"Got ping {env.msg.count} from {env.sender.name}")
        self.reply(env, Pong(env.msg.count))

ENDPOINT = "tcp://*:5001"
mgr = Manager(endpoint=ENDPOINT)
zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5001")
zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)

mgr.manage("zmq_receiver", zmq_receiver)
mgr.manage("pong", PongActor())

mgr.init()
mgr.run()
mgr.end()
```

### ping_process.py (port 5002)

```python
from actors import (
    Actor, Envelope, Manager, ManagerHandle, Start,
    RemoteActorRef, ZmqSender, ZmqReceiver, register_message
)

@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count

@register_message
class Pong:
    def __init__(self, count: int):
        self.count = count

class PingActor(Actor):
    def __init__(self, pong_ref, manager_handle):
        self.pong_ref = pong_ref
        self.manager_handle = manager_handle

    def on_start(self, env: Envelope):
        self.pong_ref.send(Ping(1), self._actor_ref)

    def on_pong(self, env: Envelope):
        if env.msg.count >= 5:
            self.manager_handle.terminate()
        else:
            self.pong_ref.send(Ping(env.msg.count + 1), self._actor_ref)

LOCAL_ENDPOINT = "tcp://*:5002"
REMOTE_PONG = "tcp://localhost:5001"

mgr = Manager(endpoint=LOCAL_ENDPOINT)
handle = mgr.get_handle()
zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5002")
zmq_receiver = ZmqReceiver(LOCAL_ENDPOINT, mgr, zmq_sender)

remote_pong = RemoteActorRef("pong", REMOTE_PONG, zmq_sender)

mgr.manage("zmq_receiver", zmq_receiver)
mgr.manage("ping", PingActor(remote_pong, handle))

mgr.init()
mgr.run()
mgr.end()
```

## Rust Interoperability

Python actors can communicate with Rust actors using the same wire protocol.

### Message Name Matching (Critical)

**The `message_type` field must match exactly between Python and Rust.** This is case-sensitive and must be identical on both sides.

When a message is sent over the wire, it includes the type name:
```json
{"message_type": "Ping", "message": {"count": 1}, ...}
```

The receiving process uses `message_type` to look up the correct deserializer. If names don't match:
- **Python**: `Unknown message type: Ping`
- **Rust**: `Message type 'Ping' not registered` panic

Example of matching registration:

| Python | Rust |
|--------|------|
| `@register_message class Ping:` | `register_remote_message::<Ping>("Ping")` |
| `@register_message class Pong:` | `register_remote_message::<Pong>("Pong")` |

### Field Names Must Match

Message field names must be identical:

**Python:**
```python
@register_message
class Ping:
    def __init__(self, count: int):  # Field: "count"
        self.count = count
```

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
struct Ping {
    count: i32,  // Field: "count" - must match!
}
```

### Running Python with Rust

**Python Pong + Rust Ping:**
```bash
# Terminal 1 - Python pong
cd /home/vm/actors-py/examples/remote_ping_pong
python3 pong_process.py

# Terminal 2 - Rust ping
cd /home/vm/actors-rust
cargo run --example rust_ping
```

**Rust Pong + Python Ping:**
```bash
# Terminal 1 - Rust pong
cd /home/vm/actors-rust
cargo run --example rust_pong

# Terminal 2 - Python ping
cd /home/vm/actors-py/examples/remote_ping_pong
python3 ping_process.py
```

## Error Handling with Reject Messages

When a remote message cannot be processed (unknown message type, actor not found, deserialization failure), the framework automatically sends a `Reject` message back to the sender.

### The Reject Message

```python
from actors import Reject

# Reject contains:
# - message_type: The type name that was rejected (e.g., "UnknownMessage")
# - reason: Why it was rejected (e.g., "Unknown message type: UnknownMessage")
# - rejected_by: The actor that rejected it (e.g., "receiver")
```

### Handling Reject in Your Actor

To receive reject notifications, handle the `Reject` message type:

```python
from actors import Actor, Reject

class MyActor(Actor):
    def on_reject(self, msg: Reject, ctx):
        print(f"Message rejected!")
        print(f"  Type: {msg.message_type}")
        print(f"  Reason: {msg.reason}")
        print(f"  Rejected by: {msg.rejected_by}")
        # Handle the error appropriately (retry, log, fallback, etc.)

    def receive(self, msg, ctx):
        if isinstance(msg, Reject):
            self.on_reject(msg, ctx)
        # ... other handlers
```

### Rejection Scenarios

The framework sends a `Reject` message when:

1. **Unknown message type** - The receiver doesn't have the message class registered with `@register_message`
2. **Actor not found** - The target actor name is not registered with the Manager
3. **Deserialization failure** - The message data doesn't match the expected structure

### Example: Reject Flow

```
Sender Process                          Receiver Process
      │                                        │
      │  ── UnknownMessage ──────────────▶    │
      │      {"message_type": "Unknown"}       │
      │                                        │
      │                               (Lookup fails:
      │                                "Unknown" not in MESSAGE_REGISTRY)
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
cd /home/vm/actors-py
python3 examples/reject_example/receiver.py
```

**Terminal 2 - Start Sender (sends UnknownMessage):**
```bash
cd /home/vm/actors-py
python3 examples/reject_example/sender.py
```

Expected output:
```
# Sender output:
SenderActor: Starting test...
SenderActor: Sending UnknownMessage (should be rejected)...
SenderActor: Sending Ping (should succeed)...
SenderActor: Received Reject!
  - Message type: UnknownMessage
  - Reason: Unknown message type: UnknownMessage. Did you register it with @register_message?
  - Rejected by: receiver
SenderActor: Received Pong 1 - normal message worked!
SenderActor: Test complete!

# Receiver output:
ReceiverActor: Received Ping 1
ReceiverActor: Sent Pong 1 back
```

### Rust Interoperability

The `Reject` message is also supported in Rust. When Rust rejects a message, Python can receive it (and vice versa). The wire format is:

```json
{
    "message_type": "Reject",
    "message": {
        "message_type": "UnknownMessage",
        "reason": "Unknown message type: UnknownMessage",
        "rejected_by": "receiver"
    }
}
```

## Limitations

1. **fast_send() not supported remotely** - Would need RPC mechanism
2. **Messages must be registered** - Use `@register_message` decorator
3. **Message fields must be JSON-serializable** - Use basic types

## Best Practices

1. **Register all message types** - Both sender and receiver must register
2. **Use unique ports** - Each process needs its own ZMQ endpoint
3. **Handle network failures** - ZMQ will queue messages, but consider timeouts
4. **Keep messages simple** - Stick to JSON-serializable types
5. **Match message names exactly** - Case-sensitive between Python and Rust
6. **Handle Reject messages** - Implement on_reject() to handle failed deliveries gracefully
