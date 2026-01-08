# Python Actors Developer Guide

This guide explains the internals of the Python Actors framework.

## Architecture Overview

The framework is built around these core concepts:

1. **Message** - Plain Python class sent between actors
2. **Actor** - Processes messages in its own thread
3. **ActorRef** - Mailbox address (LocalActorRef or RemoteActorRef)
4. **Manager** - Orchestrates actor lifecycle and registry
5. **Envelope** - Message wrapper with sender info

## Threading Model

Each actor runs in its own thread with its own queue:

```
┌─────────────────┐     ┌─────────────────┐
│   PingActor     │     │   PongActor     │
│   [Queue]       │     │   [Queue]       │
│   [Thread]      │     │   [Thread]      │
└────────┬────────┘     └────────┬────────┘
         │                       │
    blocks on              blocks on
    queue.get()            queue.get()
```

**Why threading works despite GIL:**
- GIL only prevents parallel Python bytecode execution
- `queue.get()` releases the GIL while waiting
- I/O-bound actors work fine in parallel

## ActorRef - The Mailbox Address

`ActorRef` is polymorphic - same interface for local and remote actors:

```python
# Base class
class ActorRef(ABC):
    def send(self, msg, sender=None): ...

# Local: uses Queue
class LocalActorRef(ActorRef):
    def send(self, msg, sender=None):
        self._queue.put(Envelope(msg, sender))

# Remote: uses ZMQ (see REMOTE_ACTORS_GUIDE.md)
class RemoteActorRef(ActorRef):
    def send(self, msg, sender=None):
        self._zmq_sender.send_to(...)
```

**Key distinction:**
- `self` = actor's internal state
- `self._actor_ref` = actor's mailbox address

## Message Dispatch

Handlers are methods named `on_<classname>` (lowercase):

```python
class MyActor(Actor):
    def on_ping(self, env: Envelope):
        print(f"Got ping: {env.msg.count}")
        self.reply(env, Pong(env.msg.count))

    def on_pong(self, env: Envelope):
        print(f"Got pong: {env.msg.count}")
```

The framework uses reflection to find handlers:

```python
def process_message(self, envelope):
    handler_name = f"on_{type(envelope.msg).__name__.lower()}"
    handler = getattr(self, handler_name, None)
    if handler:
        handler(envelope)
```

## Envelope - Message + Metadata

```python
@dataclass
class Envelope:
    msg: Any                    # The message
    sender: Optional[ActorRef]  # Who sent it (for replies)
```

The envelope carries:
- The message itself
- The sender's ActorRef (for `reply()`)

## Sending Messages

### send() - Async (Fire-and-Forget)

```python
# Message is queued, returns immediately
actor_ref.send(MyMessage(data=42), self._actor_ref)
```

### reply() - Respond to Messages

```python
def on_request(self, env: Envelope):
    self.reply(env, Response(result=42))
```

Reply goes to `env.sender` mailbox.

## Manager - Actor Registry

```python
mgr = Manager()
handle = mgr.get_handle()

# Register actors
pong_ref = mgr.manage("pong", PongActor())
mgr.manage("ping", PingActor(pong_ref, handle))

# Start all actors (spawns threads, sends Start)
mgr.init()

# Wait until handle.terminate() called
mgr.run()

# Stop all actors
mgr.end()
```

### ManagerHandle - Termination

```python
class MyActor(Actor):
    def __init__(self, manager_handle):
        self.manager_handle = manager_handle

    def on_done(self, env):
        self.manager_handle.terminate()
```

## Complete Example

```python
from actors import Actor, Envelope, Manager, Start

class Ping:
    def __init__(self, count):
        self.count = count

class Pong:
    def __init__(self, count):
        self.count = count

class PingActor(Actor):
    def __init__(self, pong_ref, manager_handle):
        self.pong_ref = pong_ref
        self.manager_handle = manager_handle

    def on_start(self, env):
        self.pong_ref.send(Ping(1), self._actor_ref)

    def on_pong(self, env):
        if env.msg.count >= 5:
            self.manager_handle.terminate()
        else:
            self.pong_ref.send(Ping(env.msg.count + 1), self._actor_ref)

class PongActor(Actor):
    def on_ping(self, env):
        self.reply(env, Pong(env.msg.count))

# Main
mgr = Manager()
handle = mgr.get_handle()
pong_ref = mgr.manage("pong", PongActor())
mgr.manage("ping", PingActor(pong_ref, handle))
mgr.init()
mgr.run()
mgr.end()
```

## Best Practices

1. **Keep handlers fast** - Long handlers block the actor's message queue
2. **Use `self._actor_ref`** - Pass it as sender so others can reply
3. **Handle Shutdown** - Implement `on_shutdown()` for cleanup
4. **One actor per concern** - Don't overload actors with multiple responsibilities

## Remote Error Handling

When sending messages to remote actors (via ZMQ), delivery can fail if:
- The message type is not registered with `@register_message`
- The target actor doesn't exist
- Deserialization fails

In these cases, the receiver automatically sends a `Reject` message back:

```python
from actors import Actor, Reject

class MyActor(Actor):
    def on_reject(self, env):
        msg = env.msg
        print(f"Message '{msg.message_type}' rejected by '{msg.rejected_by}': {msg.reason}")

    def receive(self, msg, ctx):
        if isinstance(msg, Reject):
            self.on_reject(msg, ctx)
        # ... other handlers
```

The `Reject` message contains:
- `message_type` - The type name that was rejected
- `reason` - Why it was rejected
- `rejected_by` - The actor that rejected it

See [REMOTE_ACTORS_GUIDE.md](REMOTE_ACTORS_GUIDE.md) for complete remote actor documentation.
