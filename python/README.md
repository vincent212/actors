# Actors - Actor Framework for Python

A lightweight actor framework for building concurrent systems in Python.

## Overview

This is part of a **multi-language actor framework** spanning C++, Rust, and Python. All three implementations share a common JSON-over-ZMQ wire protocol, enabling seamless cross-language communication while leveraging each language's unique strengths.

The Python implementation prioritizes developer productivity and integration with Python's rich ecosystem of numerical and AI libraries.

**Key strengths of the Python implementation:**
- **Reflection-Based Dispatch**: No macros needed—just name your methods `on_<messagetype>`
- **Simple Message Classes**: Messages are plain Python classes with a `@register_message` decorator for remote communication
- **NumPy/Pandas Integration**: Direct access to Python's numerical computing stack
- **Quick Prototyping**: Rapidly test actor topologies before implementing in a compiled language

**When to use Python**: Data science pipelines, AI/ML systems, prototyping actor designs, or any system where you need to integrate with Python libraries (TensorFlow, PyTorch, pandas, etc.).

For the full project documentation and blog post, see: https://m2te.ch/blog/opensource/actor-model

**Related repositories:**
- [actors-cpp](https://github.com/anthropics/actors-cpp) - C++ implementation (lowest latency)
- [actors-rust](https://github.com/anthropics/actors-rust) - Rust implementation (safety + performance)

## Features

- **Actor Model**: Independent entities processing messages sequentially
- **Message-Driven**: All communication via message passing
- **Thread-Safe**: Each actor runs in its own thread with isolated state
- **Remote Actors**: ZeroMQ-based communication with other processes/languages
- **Timers**: Schedule delayed and periodic messages
- **Simple API**: Pythonic design, minimal boilerplate

## Quick Start

### 1. Define Messages

```python
# Messages are plain Python classes
class Ping:
    def __init__(self, count: int):
        self.count = count

class Pong:
    def __init__(self, count: int):
        self.count = count
```

### 2. Create Actors

```python
from actors import Actor, Envelope

class PongActor(Actor):
    """Receives Ping, sends Pong."""

    def on_ping(self, env: Envelope) -> None:
        print(f"Received ping {env.msg.count}")
        self.reply(env, Pong(env.msg.count))
```

Handler methods are named `on_<classname>` (lowercase). The framework uses reflection to dispatch messages automatically.

### 3. Set Up Manager

```python
from actors import Manager

mgr = Manager()
handle = mgr.get_handle()

# Create actors
pong_ref = mgr.manage("pong", PongActor())
ping_ref = mgr.manage("ping", PingActor(pong_ref, handle))

mgr.init()   # Start all actors, send Start message
mgr.run()    # Block until terminated
mgr.end()    # Send Shutdown, wait for threads
```

## Core Concepts

### Actor
Base class for all actors. Implement `on_<messagetype>` methods to handle messages.

### Message
Any Python class can be a message. For remote communication, use `@register_message`.

### Manager
Manages actor lifecycle, thread creation, and provides a registry for name-based lookup.

### ActorRef
Handle for sending messages to an actor. Can be `LocalActorRef` or `RemoteActorRef`.

### Envelope
Wraps a message with sender info, enabling replies.

## Messaging

### Async Send (Fire-and-Forget)
```python
other_ref.send(MyMessage(data=42), self._actor_ref)
```
Message is queued and processed later by the receiver's thread.

### Sync Send (RPC-style)
```python
response = other_ref.fast_send(Request(), self._actor_ref)
# Caller blocks until handler completes
```

### Reply
```python
def on_request(self, env: Envelope) -> None:
    self.reply(env, Response(result=42))  # Works for both send() and fast_send()
```

## Timers

```python
from actors import Timer, Timeout

class MyActor(Actor):
    def on_start(self, env: Envelope) -> None:
        # One-shot timer (fires once after 5 seconds)
        self.timer = Timer.once(self._actor_ref, 5.0, timer_id=1)

        # Periodic timer (fires every 0.5 seconds)
        self.periodic = Timer.periodic(self._actor_ref, 0.5, timer_id=2)

    def on_timeout(self, env: Envelope) -> None:
        print(f"Timer {env.msg.id} fired!")
        if env.msg.id == 2:
            self.periodic.cancel()  # Stop periodic timer
```

## Remote Actors (Cross-Language)

### Setup Remote Communication

```python
from actors import Manager, ZmqSender, ZmqReceiver, RemoteActorRef
from actors.serialization import register_message

# Register messages for remote serialization
@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count

# Setup
mgr = Manager(endpoint="tcp://*:5001")
zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5001")
zmq_receiver = ZmqReceiver("tcp://*:5001", mgr, zmq_sender)

# Create reference to remote actor (e.g., Rust or C++ actor)
remote_pong = RemoteActorRef("pong", "tcp://localhost:5002", zmq_sender)

# Send to remote actor
remote_pong.send(Ping(count=1), self._actor_ref)
```

### Wire Protocol

All implementations use JSON-over-ZMQ:

```json
{
  "message_type": "Ping",
  "receiver": "pong_actor",
  "sender_actor": "ping_actor",
  "sender_endpoint": "tcp://localhost:5001",
  "message": {"count": 42}
}
```

## Built-in Messages

- **Start**: Sent to all actors when `mgr.init()` is called
- **Shutdown**: Sent to all actors when `mgr.end()` is called
- **Timeout**: Sent by timers when they fire
- **Reject**: Sent back when a remote message cannot be delivered

## Installation

```bash
pip install pyzmq
```

Then add the `actors` directory to your Python path or install as a package.

## Examples

### Run Examples

```bash
# Local ping-pong
python examples/ping_pong.py

# Timer example
python examples/timer_example.py

# Remote ping-pong (run in separate terminals)
python examples/remote_ping_pong/pong.py
python examples/remote_ping_pong/ping.py
```

## Files

```
actors/
  __init__.py      - Public API exports
  actor.py         - Actor, ActorRef, LocalActorRef, Envelope
  manager.py       - Manager and ManagerHandle
  messages.py      - Built-in messages (Start, Shutdown, Reject)
  remote.py        - RemoteActorRef, ZmqSender, ZmqReceiver
  serialization.py - Message serialization for remote
  timer.py         - Timer utilities

examples/
  ping_pong.py           - Basic local actor communication
  timer_example.py       - Timer usage
  remote_ping_pong/      - Cross-process communication
  remote_two_pings/      - Multiple remote actors
  reject_example/        - Handling rejected messages
```

## Documentation

See [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) for detailed internals documentation.

See [REMOTE_ACTORS_GUIDE.md](REMOTE_ACTORS_GUIDE.md) for cross-language communication.

## License

MIT License

Copyright 2025 Vincent Maciejewski & M2 Tech
