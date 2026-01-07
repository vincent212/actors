# Actors - High-Performance Actor Framework for Rust

A lightweight, high-performance actor framework for building concurrent systems in Rust.

## Overview

This is part of a **multi-language actor framework** spanning C++, Rust, and Python. All three implementations share a common JSON-over-ZMQ wire protocol, enabling seamless cross-language communication while leveraging each language's unique strengths.

The Rust implementation brings memory safety guarantees while maintaining excellent performance. It's ideal for systems where reliability is paramount but you can't sacrifice too much speed.

**When to use Rust**: Network services, systems programming, anywhere you need C++-like performance with memory safety guarantees, or when building infrastructure that must never crash.

For the full project documentation and blog post, see: https://m2te.ch/blog/opensource/actor-model

**Related repositories:**
- [actors-cpp](https://github.com/anthropics/actors-cpp) - C++ implementation (lowest latency)
- [actors-py](https://github.com/anthropics/actors-py) - Python implementation (rapid prototyping)

## Features

- **Actor Model**: Independent entities processing messages sequentially
- **Message-Driven**: All communication via typed message passing
- **Thread-Safe**: Each actor runs in its own thread with isolated state
- **Low-Latency**: Designed for high-frequency trading systems
- **Simple API**: Easy to learn, minimal boilerplate
- **Thread Pools**: Group multiple lightweight actors on shared threads

## Quick Start

### 1. Define Messages

```rust
use actors::{Message, define_message, Start};

struct Ping { count: i32 }
define_message!(Ping);

struct Pong { count: i32 }
define_message!(Pong);
```

### 2. Create Actors

```rust
use actors::{Actor, ActorContext, ActorRef, handle_messages};

struct PongActor;

impl Actor for PongActor {}

handle_messages!(PongActor,
    Ping => on_ping
);

impl PongActor {
    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        println!("Received ping {}", msg.count);
        ctx.reply(Box::new(Pong { count: msg.count }));
    }
}

struct PingActor {
    pong_ref: ActorRef,
}

impl Actor for PingActor {}

handle_messages!(PingActor,
    Start => on_start,
    Pong => on_pong
);

impl PingActor {
    fn new(pong_ref: ActorRef) -> Self {
        Self { pong_ref }
    }

    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        self.pong_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("Received pong {}", msg.count);
        self.pong_ref.send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
    }
}
```

### 3. Set Up Manager

```rust
use actors::{Manager, ThreadConfig};

fn main() {
    let mut mgr = Manager::new();

    // Register actors
    let pong_ref = mgr.manage("pong", Box::new(PongActor), ThreadConfig::default());
    let ping_ref = mgr.manage("ping", Box::new(PingActor::new(pong_ref)), ThreadConfig::default());

    mgr.init();  // Start all actors
    // ... run ...
    mgr.end();   // Wait for all actors to finish
}
```

## Core Concepts

### Actor
Base trait for all actors. Implement `process_message()` to handle messages.

### Message
All messages implement the `Message` trait. Use the `message!` macro for easy definition.

### Manager
Manages actor lifecycle, thread creation, CPU affinity, and provides a registry for name-based lookup.

### Group
Run multiple lightweight actors in a shared thread pool.

### ActorRef
Cloneable handle for sending messages to an actor. Acts as the actor's "address".

## Messaging

### Async Send (Fire-and-Forget)
```rust
other_actor.send(Box::new(MyMessage { data: 42 }), ctx.self_ref());
```
Message is queued and processed later by the receiver's thread.

### Sync Send (RPC-style)
```rust
if let Some(reply) = other_actor.fast_send(Box::new(Request {}), ctx.self_ref()) {
    if let Some(response) = reply.as_any().downcast_ref::<Response>() {
        // Handle response
    }
}
```
Caller blocks until handler completes and returns a reply.

### Reply
```rust
fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
    ctx.reply(Box::new(Response { result: 42 }));  // Works for both send() and fast_send()
}
```

## CPU Affinity & Priority

```rust
// Pin actor to CPU core 2 with FIFO scheduling, priority 50
let config = ThreadConfig::new(vec![2], 50, libc::SCHED_FIFO);
mgr.manage("my_actor", Box::new(MyActor), config);
```

## Thread Pools with Group

```rust
use actors::Group;

// Create group with 4 worker threads
let mut group = Group::new("workers", 4);

// Add lightweight actors
group.add("worker_1", Box::new(Worker::new()));
group.add("worker_2", Box::new(Worker::new()));
group.add("worker_3", Box::new(Worker::new()));

group.start();
// ... send messages ...
group.end();
```

## Building

```bash
cargo build
```

### Run Examples

```bash
cargo run --example ping_pong
cargo run --example group_workers
```

### Requirements

- Rust 2021 edition
- libc (for pthread affinity/priority)

## Files

```
src/
  lib.rs       - Public API exports
  actor.rs     - Actor trait and ActorRef
  message.rs   - Message trait and macro
  messages.rs  - Built-in messages (Start, Shutdown, Timeout, Continue)
  manager.rs   - Actor lifecycle manager
  group.rs     - Thread pool for lightweight actors
  timer.rs     - Timer utilities

examples/
  ping_pong.rs      - Basic actor communication
  group_workers.rs  - Thread pool with Group
```

## Documentation

See [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) for detailed internals documentation.

## License

MIT License
