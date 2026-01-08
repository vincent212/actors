# Actors Developer Guide

This guide explains the internals of the Actors framework for Rust.

## Architecture Overview

The actors framework is built around these core concepts:

1. **Message** - Data sent between actors
2. **Actor** - An entity that processes messages in its own thread
3. **ActorRef** - A cloneable handle for sending messages to an actor
4. **Manager** - Orchestrates actor lifecycle and provides a registry
5. **Group** - Thread pool for running lightweight actors

## Messaging System

### Channels (std::sync::mpsc)

Each actor has a channel pair:
- **Sender** - Cloneable, used to send messages TO the actor
- **Receiver** - Owned by the actor, used to receive messages

```rust
use std::sync::mpsc::{channel, Sender, Receiver};

// Create a channel
let (sender, receiver) = channel::<Envelope>();

// Sender can be cloned - each clone is another "address" to the actor
let sender2 = sender.clone();

// Both senders can send to the same receiver
sender.send(envelope1);
sender2.send(envelope2);

// Receiver blocks until a message arrives
let msg = receiver.recv();  // Blocks if queue empty
```

### ActorRef - The Actor's Mailbox Address

`ActorRef` is the actor's **mailbox address** - a cloneable handle that others use to send messages to the actor. It wraps a channel `Sender`:

```rust
#[derive(Clone)]
pub struct ActorRef {
    sender: Sender<Envelope>,
    name: String,
}
```

When you clone an `ActorRef`, you get another handle to the same actor's mailbox.
All clones send to the same channel receiver.

**Key distinction:**
- `&mut self` in a handler = the actor's internal **state** (data, fields)
- `ctx.self_ref()` = the actor's **mailbox address** (how others reach you)

### Envelope - Message + Metadata

Messages are wrapped in `Envelope` to carry routing metadata:

```rust
pub struct Envelope {
    pub msg: Box<dyn Message>,      // The actual message
    pub sender: Option<ActorRef>,    // Who sent it (for replies)
}
```

This keeps the message immutable while allowing metadata to be managed separately.

## Sending Messages

### send() - Async (Fire-and-Forget)

```rust
// Message is queued, returns immediately
// Handler runs in RECEIVER's thread
actor_ref.send(Box::new(MyMessage { data: 42 }), ctx.self_ref());
```

Flow:
1. Caller creates message, wraps in Envelope
2. Envelope sent through channel (non-blocking)
3. Receiver's thread picks up envelope from queue
4. Handler runs in receiver's thread

### reply() - Respond to Messages

```rust
impl Actor for MyActor {
    fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
        if let Some(req) = msg.as_any().downcast_ref::<Request>() {
            ctx.reply(Box::new(Response { result: 42 }));
        }
    }
}
```

Reply goes through the sender's ActorRef (if provided).

## ActorContext - Why It's Separate from Message

`ActorContext` provides the execution context for message handlers. It's passed separately from the message for several reasons:

### What ActorContext Contains

```rust
pub struct ActorContext {
    self_ref: Option<ActorRef>,           // This actor's mailbox address
    sender: Option<ActorRef>,             // Who sent the current message
}
```

### Why Not Part of the Message?

1. **Messages are immutable data** - A message like `Ping { count: 5 }` is pure data. It shouldn't carry execution state or mutable references.

2. **Messages are reusable** - The same message struct could theoretically be sent multiple times. Each delivery has different context (different sender, different reply channel).

3. **Separation of concerns**:
   - Message = "what data was sent"
   - Context = "how to respond" and "who am I"

4. **Thread safety** - Messages must be `Send`. If context (with its channels and mutability) were embedded in messages, it would complicate ownership.

### How to Use ActorContext

```rust
fn on_request(&mut self, msg: &Request, ctx: &mut ActorContext) {
    // Get this actor's "mailbox address" - an ActorRef others can use to send messages here
    // Note: &mut self is the actor's STATE, ctx.self_ref() is the actor's ADDRESS
    let my_address = ctx.self_ref();

    // Send a reply to the sender
    ctx.reply(Box::new(Response { result: 42 }));
}
```

### Comparison to C++

In C++, the equivalent is `destination_ptr` on the message:
```cpp
msg->destination_ptr  // equivalent to ctx.self_ref()
msg->sender_ptr       // passed in envelope.sender
```

In Rust, we keep context separate to maintain message immutability and cleaner ownership.

## Message Definition

Use the `define_message!` macro to implement the Message trait:

```rust
use actors::{Message, define_message};

struct Ping {
    count: i32,
}
define_message!(Ping);

struct Pong {
    count: i32,
}
define_message!(Pong);
```

Built-in messages:
- `Start` - Sent when actor starts
- `Shutdown` - Sent when shutting down
- `Timeout` - Sent by Timer
- `Continue` - For self-continuation
- `Reject` - Sent when remote message delivery fails (see Remote Error Handling)

## Actor Definition

### Using handle_messages! Macro (Recommended)

The `handle_messages!` macro provides type-safe message dispatch:

```rust
use actors::{Actor, ActorContext, define_message, handle_messages, Start};

struct Ping { count: i32 }
define_message!(Ping);

struct Pong { count: i32 }
define_message!(Pong);

struct MyActor {
    state: i32,
}

impl MyActor {
    fn new() -> Self {
        MyActor { state: 0 }
    }

    // Handler for Start message
    fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
        println!("Actor starting, state = {}", self.state);
    }

    // Handler for Ping message
    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        self.state += msg.count;
        ctx.reply(Box::new(Pong { count: self.state }));
    }
}

// Register handlers - this implements Actor trait automatically
handle_messages!(MyActor,
    Start => on_start,
    Ping => on_ping
);
```

The macro generates:
1. A `dispatch_message()` method that tries each handler in order
2. An `Actor` impl that calls `dispatch_message()` from `process_message()`

### Manual Implementation (Alternative)

For more control, implement Actor trait directly:

```rust
use actors::{Actor, ActorContext, Message};

struct MyActor {
    state: i32,
}

impl Actor for MyActor {
    fn init(&mut self) {
        // Called once at startup, before processing messages
        println!("Actor starting");
    }

    fn end(&mut self) {
        // Called once at shutdown, after message processing stops
        println!("Actor shutting down");
    }

    fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
        // Handle messages by downcasting
        if let Some(ping) = msg.as_any().downcast_ref::<Ping>() {
            self.state += ping.count;
            ctx.reply(Box::new(Pong { count: self.state }));
        }
    }
}
```

## Manager - Actor Registry

The Manager tracks all actors by name and handles lifecycle:

```rust
use actors::{Manager, ThreadConfig};

let mut mgr = Manager::new();

// Register actors - returns ActorRef
let actor1_ref = mgr.manage("actor1", Box::new(MyActor::new()), ThreadConfig::default());
let actor2_ref = mgr.manage("actor2", Box::new(OtherActor::new(actor1_ref)), ThreadConfig::default());

// Look up actor by name
if let Some(actor_ref) = mgr.get_ref("actor1") {
    actor_ref.send(Box::new(Ping { count: 1 }), None);
}

// Start all actors (sends Start message, spawns threads)
mgr.init();

// ... run ...

// Shutdown (sends Shutdown, waits for threads)
mgr.end();
```

### ManagerHandle - Signaling Termination

Actors can signal the manager to terminate using `ManagerHandle`:

```rust
use actors::{Manager, ManagerHandle, ThreadConfig};

struct MyActor {
    manager_handle: ManagerHandle,
}

impl MyActor {
    fn on_done(&mut self, _msg: &DoneMessage, _ctx: &mut ActorContext) {
        // Signal the manager to terminate
        self.manager_handle.terminate();
    }
}

fn main() {
    let mut mgr = Manager::new();
    let handle = mgr.get_handle();  // Get handle before creating actors

    // Pass handle to actors that need to signal termination
    let actor = MyActor { manager_handle: handle };
    mgr.manage("my_actor", Box::new(actor), ThreadConfig::default());

    mgr.init();

    // Blocks until an actor calls manager_handle.terminate()
    mgr.run();

    // Shutdown all actors
    mgr.end();
}
```

`ManagerHandle` is `Clone`, so you can share it with multiple actors.

### Thread Configuration

```rust
// Default (no affinity, normal priority)
ThreadConfig::default()

// Pin to specific CPU cores
ThreadConfig::with_affinity(vec![0, 1])

// Real-time priority (requires CAP_SYS_NICE)
ThreadConfig::with_priority(50, libc::SCHED_FIFO)

// Both
ThreadConfig::new(vec![2], 50, libc::SCHED_FIFO)
```

## Group - Thread Pool

For many lightweight actors that don't need dedicated threads:

```rust
use actors::Group;

// Create group with 4 worker threads
let mut group = Group::new("my_group", 4);

// Add actors - they share the thread pool
let ref1 = group.add("light_actor_1", Box::new(LightActor::new()));
let ref2 = group.add("light_actor_2", Box::new(LightActor::new()));
let ref3 = group.add("light_actor_3", Box::new(LightActor::new()));
// ... add more actors

// Start the group
group.start();

// Send messages normally
ref1.send(Box::new(ComputeMessage { input: 42 }), None);

// Shutdown
group.end();
```

### How Group Works

1. All actors in a group share a single message queue
2. N worker threads pull messages from the queue
3. Each message includes the destination actor name
4. Workers look up the actor and acquire its Mutex before processing
5. Per-actor Mutex prevents concurrent access to actor state

```
                    ┌─────────────┐
                    │ Shared Queue│
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
   ┌─────────┐        ┌─────────┐        ┌─────────┐
   │Worker 1 │        │Worker 2 │        │Worker 3 │
   └────┬────┘        └────┬────┘        └────┬────┘
        │                  │                  │
        ▼                  ▼                  ▼
   ┌─────────┐        ┌─────────┐        ┌─────────┐
   │Actor A  │        │Actor B  │        │Actor C  │
   │(Mutex)  │        │(Mutex)  │        │(Mutex)  │
   └─────────┘        └─────────┘        └─────────┘
```

## Timer

Schedule delayed or periodic messages:

```rust
use actors::{Timer, next_timer_id};
use std::time::Duration;

// One-shot timer (sends Timeout after 5 seconds)
let timer = Timer::once(actor_ref.clone(), Duration::from_secs(5), next_timer_id());

// Periodic timer (sends Timeout every 100ms)
let timer = Timer::periodic(actor_ref.clone(), Duration::from_millis(100), 1);

// Cancel when done
timer.cancel();
```

Handle timeouts in your actor:

```rust
use actors::Timeout;

fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
    if let Some(timeout) = msg.as_any().downcast_ref::<Timeout>() {
        match timeout.id {
            1 => println!("Periodic timer fired"),
            _ => println!("Timer {} fired", timeout.id),
        }
    }
}
```

## Memory Management

### Box<dyn Message>

Messages are heap-allocated trait objects:

```rust
// Box allocates on heap, dyn allows polymorphism
let msg: Box<dyn Message> = Box::new(Ping { count: 1 });
```

Why Box?
- Messages need to be stored in channels (fixed size required)
- Different message types have different sizes
- Box provides uniform pointer size

### Option<T>

Rust's null replacement:

```rust
// Either contains a value or nothing
let sender: Option<ActorRef> = Some(actor_ref);
let sender: Option<ActorRef> = None;

// Use pattern matching
if let Some(ref sender) = envelope.sender {
    sender.send(reply, None);
}

// Or methods
sender.as_ref().map(|s| s.name());
sender.unwrap_or_default();
```

### Arc<Mutex<T>>

Thread-safe shared ownership with interior mutability:

```rust
use std::sync::{Arc, Mutex};

// Arc = Atomic Reference Count (shared ownership across threads)
// Mutex = Mutual Exclusion (only one thread accesses at a time)
let shared_actor: Arc<Mutex<Box<dyn Actor>>> = Arc::new(Mutex::new(actor));

// Clone Arc to share (doesn't clone the actor, just the pointer)
let shared2 = Arc::clone(&shared_actor);

// Lock to access
{
    let mut actor = shared_actor.lock().unwrap();
    actor.process_message(msg, ctx);
}  // Lock released when guard goes out of scope
```

Used in Group for per-actor locks.

## Best Practices

1. **Keep actors lightweight** - Heavy computation blocks the message queue
2. **Use Groups for many small actors** - Reduces thread overhead
3. **Handle Shutdown gracefully** - Clean up resources in end()
4. **Use Timer for periodic work** - Don't block in actors

## Error Handling

Channel operations can fail:

```rust
// send() silently ignores errors (actor might be shut down)
actor_ref.send(msg, None);
```

## Thread Safety

- Each standalone actor runs in its own thread (no sharing)
- Group actors share threads but have per-actor Mutexes
- ActorRef is Clone + Send (safe to pass between threads)
- Messages must be Send + 'static (no borrowed data)

## Remote Error Handling

When sending messages to remote actors (via ZMQ), delivery can fail if:
- The message type is not registered on the receiver
- The target actor doesn't exist
- Deserialization fails

In these cases, the receiver automatically sends a `Reject` message back:

```rust
use actors::{Reject, handle_messages, register_remote_message};

// Register Reject for remote communication
register_remote_message::<Reject>("Reject");

struct MyActor { /* ... */ }

handle_messages!(MyActor,
    Start => on_start,
    Reject => on_reject  // Handle rejection notifications
);

impl MyActor {
    fn on_reject(&mut self, msg: &Reject, _ctx: &mut ActorContext) {
        eprintln!("Message '{}' rejected by '{}': {}",
            msg.message_type, msg.rejected_by, msg.reason);
    }
}
```

See [REMOTE_ACTORS_GUIDE.md](REMOTE_ACTORS_GUIDE.md) for complete remote actor documentation.
