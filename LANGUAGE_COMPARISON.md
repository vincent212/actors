# Actors Framework - Language Comparison

This document explains the key differences between the C++, Rust, Python, and Java implementations of the Actors framework, with detailed reasoning behind each design decision.

## Overview

All four implementations share the same core concepts and wire protocol, but each adapts to its language's idioms, memory model, and performance characteristics.

| Aspect | C++ | Rust | Python | Java |
|--------|-----|------|--------|------|
| **Primary Use** | Ultra-low latency | Safety + performance | Rapid prototyping | JVM ecosystem |
| **Message Wrapper** | None (raw pointer) | Envelope | Envelope | Envelope |
| **Handler Context** | Message fields | ActorContext | Envelope | Envelope |
| **Handler Registration** | Macro in constructor | Macro after impl | Reflection | Reflection |
| **Memory Management** | Manual (new/delete) | Ownership (Box) | GC | GC |

---

## The Envelope Question: Why C++ Doesn't Need One

This is the most significant architectural difference between implementations. Understanding it reveals the fundamental tradeoffs in actor system design.

### What is an Envelope?

An envelope is a wrapper around a message that carries **routing metadata** - information about where the message came from and how to respond. In most actor systems, this includes:

- The message payload itself
- The sender's address (ActorRef)
- Sometimes: timestamps, correlation IDs, priority hints

### The Problem Envelopes Solve

Consider this scenario: Actor A sends a `Ping` message to Actor B. Actor B wants to reply with `Pong`. How does B know where to send the reply?

**Option 1: Embed routing in the message**
```cpp
struct Ping {
    int count;
    Actor* sender;  // Routing info baked into message
};
```

**Option 2: Wrap the message**
```rust
struct Envelope {
    msg: Box<dyn Message>,  // The actual message
    sender: Option<ActorRef>,  // Routing info separate
}
```

### Why C++ Embeds Metadata in Messages

C++ chooses Option 1 - routing metadata lives directly on the `Message` base class:

```cpp
struct Message {
    virtual int get_message_id() const = 0;
    mutable Actor *sender;       // Who sent this message
    mutable Actor *destination;  // Target actor (for routing)
    mutable bool last;           // Shutdown signal flag
};

struct Ping : public actors::Message_N<100> {
    int count;
    Ping(int c) : count(c) {}
    // Inherits sender, destination, last from Message
};
```

**The reasoning:**

1. **Zero additional allocations**: Creating an envelope means allocating another object. In C++, every allocation has a cost (typically 50-200ns for malloc). When you're targeting sub-microsecond latency, one extra allocation per message is significant.

2. **Cache locality**: The message and its metadata are in contiguous memory. When the CPU loads the message, it gets the routing info in the same cache line. An envelope adds a pointer indirection - the CPU must chase a pointer to get the actual message.

3. **No polymorphic wrapper**: Envelopes typically use dynamic dispatch (`Box<dyn Message>` in Rust, `Object` in Java). C++ messages already use vtables for `get_message_id()`, but avoiding a second layer of indirection matters.

4. **Simpler lifetime**: The message IS the unit of work. When processing completes, delete the message - done. With envelopes, you're managing envelope lifetime separately from message lifetime.

**The tradeoff:**

Messages are "impure" - a `Ping` isn't just data, it carries mutable routing state. This is philosophically unclean but pragmatically fast.

```cpp
// In C++, the handler accesses sender via the message
void on_ping(const Ping *m) noexcept {
    // m->sender is the return address
    // reply() internally uses m->sender
    reply(new Pong(m->count));
}

// The Actor::reply() implementation
void Actor::reply(const Message *m) noexcept {
    if (reply_to) {  // reply_to was set from incoming message's sender
        reply_to->send(m, this);
    }
}
```

### Why Rust, Python, and Java Use Envelopes

These languages prioritize different values than raw speed:

#### Rust: Safety Through Separation

```rust
pub struct Envelope {
    pub msg: Box<dyn Message>,
    pub sender: Option<ActorRef>,
}
```

**The reasoning:**

1. **Immutable messages**: Rust's ownership model strongly encourages immutable data. A `Ping { count: 5 }` should be pure data - no mutable routing state. The envelope handles mutability.

2. **Borrow checker compliance**: If routing info were on the message, you'd need `&mut Message` to update sender/destination. But handlers receive `&Message` (immutable borrow). Separating routing into the envelope sidesteps this.

3. **Clear ownership boundaries**: The envelope owns the message (`Box<dyn Message>`). When the envelope is dropped, the message is dropped. Clean and predictable.

4. **Generic over message types**: `Box<dyn Message>` allows any message type in a single queue. The alternative (generic queues per message type) would be complex.

**The cost**: One extra heap allocation per message (the Box). For Rust's target use cases (safety-critical systems, not HFT), this is acceptable.

#### Python & Java: Natural Fit for GC Languages

```python
@dataclass
class Envelope:
    msg: Any
    sender: Optional[ActorRef]
```

```java
public class Envelope {
    private final Object msg;
    private final ActorRef sender;
}
```

**The reasoning:**

1. **GC handles allocation cost**: In garbage-collected languages, object allocation is cheap (bump pointer allocation). The envelope overhead is negligible.

2. **Immutability is idiomatic**: Python dataclasses and Java's `final` fields make envelopes naturally immutable. This matches the languages' best practices.

3. **Dynamic typing needs a wrapper anyway**: Python's `Any` and Java's `Object` already involve runtime type information. An envelope adds structure to this.

4. **No manual memory management**: Unlike C++, there's no concern about "who deletes what". The GC handles everything.

---

## ActorContext: Why Rust Needs It and Others Don't

### The Problem

When a handler runs, it needs access to:
1. The message content (`Ping { count: 5 }`)
2. A way to reply (needs sender's address)
3. This actor's own address (to pass as sender when sending)

Where does this information come from?

### C++ Solution: Everything on the Actor

```cpp
class Actor {
protected:
    Actor *reply_to;  // Set before handler runs, from message->sender

public:
    void reply(const Message *m) noexcept {
        if (reply_to) {
            reply_to->send(m, this);  // 'this' is our address
        }
    }
};

// Handler signature - just the message
void on_ping(const Ping *m) noexcept {
    // 'this' is available (we're a method)
    // reply_to is a member variable
    reply(new Pong(m->count));
}
```

**Why this works in C++:**

- `this` pointer is always available in methods - that's the actor's address
- `reply_to` is set by the framework before invoking the handler
- No extra parameters needed

### Rust Solution: ActorContext Parameter

```rust
pub struct ActorContext {
    self_ref: Option<ActorRef>,   // This actor's mailbox address
    sender: Option<ActorRef>,     // Who sent the current message
}

impl ActorContext {
    pub fn self_ref(&self) -> Option<ActorRef> {
        self.self_ref.clone()
    }

    pub fn reply(&self, msg: Box<dyn Message>) {
        if let Some(sender) = &self.sender {
            sender.send(msg, self.self_ref.clone());
        }
    }
}

// Handler signature - message AND context
fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
    ctx.reply(Box::new(Pong { count: msg.count }));
}
```

**Why Rust needs ActorContext:**

1. **`&mut self` is the actor's STATE, not its ADDRESS**

   In Rust, `self` in a method is the struct instance - `PongActor { some_state: 42 }`. This is the actor's internal data. But to send messages, you need an `ActorRef` - a handle that can be cloned and sent to other threads.

   ```rust
   struct PongActor {
       message_count: u32,  // This is what &mut self gives you
   }

   // But ActorRef is a separate thing:
   struct ActorRef {
       sender: Sender<Envelope>,  // Channel to the actor's queue
       name: String,
   }
   ```

   The actor doesn't "know" its own ActorRef unless someone tells it. That's what ActorContext provides.

2. **Ownership and borrowing constraints**

   The actor can't store its own ActorRef as a field and also hand it out, because:
   - ActorRef must be `Clone` (so others can keep copies)
   - If stored in `self`, you'd need `&self` to access it
   - But handlers take `&mut self` (exclusive borrow)
   - Can't have `&self` and `&mut self` simultaneously

   Passing ActorContext separately sidesteps this.

3. **Separation of concerns**

   - `&mut self` = actor's mutable state (your business logic)
   - `&Ping` = the message (immutable, borrowed)
   - `&mut ActorContext` = execution context (framework-provided)

   This is clean: your actor struct only contains your data, not framework plumbing.

### Python Solution: Envelope Contains Everything

```python
def on_ping(self, env: Envelope):
    # env.msg is the Ping
    # env.sender is the sender's ActorRef
    # self._actor_ref is this actor's address (stored on Actor base class)
    self.reply(env, Pong(count=env.msg.count))
```

**Why Python doesn't need ActorContext:**

1. **No ownership restrictions**: Python's GC means `self._actor_ref` can be stored on the actor and freely accessed. No borrowing rules.

2. **Dynamic typing**: The envelope's `sender` field is just an object reference. No complex generics needed.

3. **Simpler is better**: Python prioritizes readability. One parameter (envelope) is simpler than two (message + context).

### Java Solution: Same as Python

```java
public void on_Ping(Envelope env) {
    Ping ping = (Ping) env.getMsg();
    // env.getSender() is the sender's ActorRef
    // this.actorRef is this actor's address (protected field)
    reply(env, new Pong(ping.count));
}
```

**Why Java doesn't need ActorContext:**

Same reasons as Python - garbage collection eliminates ownership concerns, and the envelope pattern is idiomatic Java (immutable value objects).

---

## Handler Registration: Compile-Time vs Runtime

### C++ and Rust: Compile-Time Registration

Both use macros to wire up handlers at compile time:

**C++ - MESSAGE_HANDLER in constructor:**
```cpp
class PongActor : public actors::Actor {
public:
    PongActor() {
        MESSAGE_HANDLER(Ping, on_ping);
        MESSAGE_HANDLER(actors::msg::Start, on_start);
    }

    void on_ping(const Ping *m) noexcept {
        reply(new Pong(m->count));
    }

    void on_start(const actors::msg::Start *) noexcept {
        std::cout << "Started" << std::endl;
    }
};
```

**What MESSAGE_HANDLER does:**

```cpp
#define MESSAGE_HANDLER(message_type, function_name)                            \
{                                                                               \
    typedef typename std::remove_reference<decltype(*this)>::type ActorT;       \
    actors::register_handler<ActorT, message_type>(this)(&ActorT::function_name);\
}
```

1. Extracts the actor's type from `this`
2. Creates a type-erased function pointer
3. Stores in a map keyed by `std::type_index(typeid(MessageType))`
4. Also caches in a 512-slot array for O(1) lookup by message ID

**Why compile-time registration in C++:**

- Function pointer dispatch is as fast as virtual calls
- Type safety at compile time (wrong handler signature won't compile)
- The 512-slot cache means most dispatches are a single array lookup
- No runtime reflection overhead

**Rust - handle_messages! macro:**
```rust
impl PongActor {
    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        ctx.reply(Box::new(Pong { count: msg.count }));
    }

    fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
        println!("Started");
    }
}

handle_messages!(PongActor,
    Ping => on_ping,
    Start => on_start
);
```

**What handle_messages! generates:**

```rust
impl Actor for PongActor {
    fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
        if let Some(m) = msg.as_any().downcast_ref::<Ping>() {
            self.on_ping(m, ctx);
        } else if let Some(m) = msg.as_any().downcast_ref::<Start>() {
            self.on_start(m, ctx);
        }
        // ... else unhandled
    }
}
```

**Why compile-time registration in Rust:**

- Match-based dispatch is predictable (no hash lookups)
- Type safety enforced at compile time
- No runtime reflection (Rust has limited reflection)
- Downcasting is explicit and safe

### Python and Java: Runtime Reflection

**Python - naming convention discovery:**
```python
class PongActor(Actor):
    def on_ping(self, env: Envelope):
        self.reply(env, Pong(count=env.msg.count))

    def on_start(self, env: Envelope):
        print("Started")
```

**How dispatch works:**

```python
def _dispatch(self, env: Envelope):
    msg = env.msg
    msg_type = type(msg).__name__.lower()  # "ping"
    method_name = f"on_{msg_type}"          # "on_ping"

    # Check cache first
    handler = self._handler_cache.get(method_name)
    if handler is None:
        handler = getattr(self, method_name, None)
        self._handler_cache[method_name] = handler

    if handler:
        handler(env)
```

**Java - reflection with caching:**
```java
class PongActor extends Actor {
    public void on_Ping(Envelope env) {
        Ping ping = (Ping) env.getMsg();
        reply(env, new Pong(ping.count));
    }

    public void on_Start(Envelope env) {
        System.out.println("Started");
    }
}
```

**How dispatch works:**

```java
protected void processMessage(Envelope env) {
    Object msg = env.getMsg();
    String msgType = msg.getClass().getSimpleName();  // "Ping"
    String methodName = "on_" + msgType;               // "on_Ping"

    Method method = handlerCache.computeIfAbsent(msgType, k -> {
        try {
            return this.getClass().getMethod(methodName, Envelope.class);
        } catch (NoSuchMethodException e) {
            return null;
        }
    });

    if (method != null) {
        method.invoke(this, env);
    }
}
```

**Why runtime reflection in Python/Java:**

1. **No registration boilerplate**: Just name your method correctly, it works
2. **Dynamic languages expect reflection**: It's idiomatic
3. **Caching mitigates overhead**: First call is slow, subsequent calls are fast
4. **Development speed matters more**: In Python/Java use cases, the few microseconds of reflection overhead are irrelevant

---

## Memory Management Deep Dive

### C++ - Manual Ownership Transfer

```cpp
// Caller allocates
other->send(new Ping(1), this);
// IMPORTANT: Do NOT delete - ownership transferred to framework

// Inside Actor::operator() - the main loop
void Actor::operator()() noexcept {
    while (!terminated) {
        auto [m, valid] = msgq->pop();
        if (valid) {
            reply_to = m->sender;
            call_handler(m);
            delete m;  // Framework deletes after processing
        }
    }
}
```

**The contract:**

1. Caller allocates with `new`
2. `send()` takes ownership
3. Receiver processes message
4. Receiver deletes message

**Why this works:**

- Single owner at any time (caller → queue → receiver)
- Deterministic deallocation (immediately after processing)
- No reference counting overhead

**The danger:**

```cpp
Ping* p = new Ping(1);
other->send(p, this);
// BUG: p is now owned by the queue
p->count = 2;  // Undefined behavior - might already be deleted!
```

### Rust - Explicit Ownership with Box

```rust
// Box<T> = heap-allocated, single owner
other_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
// Ownership moved to send(), compiler enforces this

// Cannot use the Box after sending:
let msg = Box::new(Ping { count: 1 });
other_ref.send(msg, ctx.self_ref());
// msg.count = 2;  // COMPILE ERROR: value moved
```

**The guarantee:**

The compiler proves at compile time that:
- Only one owner exists at any moment
- The message isn't used after being sent
- The message will be dropped exactly once

**Where is the message deleted?**

In Rust, there's no explicit `delete` - memory is freed when the owner goes out of scope. Here's the lifecycle:

```rust
// 1. Sender creates message, Box owns it
let msg = Box::new(Ping { count: 1 });  // Box allocates on heap

// 2. send() takes ownership - Box moves into Envelope
other_ref.send(msg, ctx.self_ref());
// msg is now INVALID - ownership transferred

// 3. Inside send(), the Envelope is created and queued
pub fn send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) {
    let envelope = Envelope { msg, sender };  // Box moves into Envelope
    self.sender.send(envelope);               // Envelope moves into channel
}

// 4. Receiver's run loop takes ownership from channel
fn run(&mut self) {
    while let Ok(envelope) = self.receiver.recv() {
        // envelope now owns the Box<dyn Message>
        self.process_message(&*envelope.msg, &mut ctx);
        // After processing, envelope goes out of scope HERE
        // Rust automatically:
        //   - Drops envelope
        //   - Which drops envelope.msg (the Box)
        //   - Which frees the heap memory
    }
}
```

**The key insight**: Rust's `Drop` trait handles cleanup automatically. When `envelope` goes out of scope at the end of the loop iteration, Rust calls `drop()` on it, which recursively drops the `Box<dyn Message>`, freeing the heap allocation.

There's no `delete envelope.msg` - the drop happens implicitly when the scope ends. This is why Rust is "zero-cost" - you get deterministic cleanup without writing cleanup code.

**Comparison to C++:**

| Aspect | C++ | Rust |
|--------|-----|------|
| Allocation | `new Ping(1)` | `Box::new(Ping { count: 1 })` |
| Ownership transfer | Implicit (convention) | Explicit (move semantics, compiler-enforced) |
| Deallocation | Explicit `delete m` | Implicit when owner goes out of scope |
| Use-after-free | Possible (runtime bug) | Impossible (compile error) |
| Double-free | Possible (runtime bug) | Impossible (compile error) |

### Python & Java - Garbage Collection

```python
other_ref.send(Ping(count=1), self._actor_ref)
# GC tracks all references
# When no more references exist, object is collected
```

**The tradeoff:**

- **Pro**: No manual memory management, no leaks, no use-after-free
- **Con**: GC pauses (unpredictable latency), memory overhead (object headers, reference tracking)

For Python/Java use cases (not HFT), this is the right tradeoff.

---

## Thread Configuration

### C++ and Rust: Full Control

Both provide CPU affinity and thread priority control for real-time systems:

**C++:**
```cpp
// Pin actor to CPU core 2, SCHED_FIFO at max priority
int prio = sched_get_priority_max(SCHED_FIFO);
manage(actor, {2}, prio, SCHED_FIFO);
```

**Rust:**
```rust
let config = ThreadConfig::new(
    vec![2],           // CPU cores
    50,                // Priority
    libc::SCHED_FIFO   // Scheduler policy
);
mgr.manage("actor", Box::new(actor), config);
```

**Why this matters:**

1. **CPU affinity**: Keeps the actor's data in one CPU's cache. Migrating between CPUs flushes cache (expensive).

2. **SCHED_FIFO**: Real-time scheduler that won't preempt for lower-priority processes. Critical for latency-sensitive work.

3. **Isolated cores**: Combined with kernel boot parameters (`isolcpus`), you can dedicate cores to actors with no OS interference.

### Python & Java: Platform Threads Only

```python
# Python - standard threading
mgr.manage("actor", MyActor())  # No affinity/priority options
```

```java
// Java - standard threads
manager.manage("actor", new MyActor());  // No affinity/priority options
```

**Why no thread control:**

1. **GIL (Python)**: Python's Global Interpreter Lock means true parallelism is limited anyway. CPU affinity provides less benefit.

2. **JVM abstraction**: Java's philosophy is "write once, run anywhere". Platform-specific thread tuning conflicts with this.

3. **Target use cases**: Python and Java actors are for distributed systems, ML pipelines, enterprise apps - not sub-microsecond trading.

---

## Remote Communication

All four languages share the same wire protocol for cross-language interop:

```json
{
    "message_type": "Ping",
    "receiver": "pong",
    "sender_actor": "ping",
    "sender_endpoint": "tcp://localhost:5001",
    "message": {"count": 1}
}
```

### Registration for Remote Messages

| Language | Registration |
|----------|-------------|
| C++ | `register_remote_message<Ping>("Ping")` |
| Rust | `register_remote_message::<Ping>("Ping")` |
| Python | `@register_message` decorator |
| Java | `@RegisterMessage` annotation + `Serialization.registerMessage(Ping.class)` |

### Why JSON?

- Human readable (debugging)
- Language agnostic (all languages have JSON libraries)
- Schema-flexible (easy to evolve messages)

For ultra-low-latency local communication, use the native implementations. Reserve JSON-over-ZMQ for cross-language and cross-process scenarios.

---

## Summary: When to Use Each

| Language | Best For | Typical Latency |
|----------|----------|-----------------|
| **C++** | Ultra-low latency, trading systems, real-time control | < 1μs |
| **Rust** | Safety-critical systems, when you want C++ speed with memory safety | 1-10μs |
| **Python** | Prototyping, data science, AI/ML pipelines, glue code | 100μs-1ms |
| **Java** | Enterprise systems, JVM ecosystem (Kafka, Spark, etc.) | 10-100μs |

### Mixing Languages

All four can communicate seamlessly via ZMQ. A common pattern:

```
[Python ML Model] --ZMQ--> [Rust Risk Engine] --ZMQ--> [C++ Order Router]
                                    ^
                                    |
                            [Java Monitoring Dashboard]
```

Each component uses the language best suited to its requirements.

---

## Quick Reference: Handler Signatures

| Language | Handler Signature |
|----------|-------------------|
| C++ | `void on_ping(const Ping *m) noexcept` |
| Rust | `fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext)` |
| Python | `def on_ping(self, env: Envelope)` |
| Java | `public void on_Ping(Envelope env)` |

## Quick Reference: Sending Messages

| Language | Send Syntax |
|----------|-------------|
| C++ | `other->send(new Ping(1), this);` |
| Rust | `other_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());` |
| Python | `other_ref.send(Ping(count=1), self._actor_ref)` |
| Java | `otherRef.send(new Ping(1), actorRef);` |

## Quick Reference: Replying

| Language | Reply Syntax |
|----------|--------------|
| C++ | `reply(new Pong(m->count));` |
| Rust | `ctx.reply(Box::new(Pong { count: msg.count }));` |
| Python | `self.reply(env, Pong(count=env.msg.count))` |
| Java | `reply(env, new Pong(ping.count));` |
