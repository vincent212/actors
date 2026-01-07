# In-Process FFI: Why Cross-Language Actor Communication Beats IPC for Performance-Critical Systems

In my [previous post](/blog/opensource/actor-model), I introduced our open-source actor model implementations in C++, Rust, and Python. Today, I want to dive into a critical piece that makes multi-language actor systems practical for high-performance applications: **in-process FFI (Foreign Function Interface) communication**.

The question isn't whether you *can* connect actors across languages—ZeroMQ, gRPC, and shared memory all work. The question is whether you can do it **fast enough** that the language boundary becomes invisible.

## The Problem: Language Boundaries Kill Latency

Consider a trading system where a C++ order router needs to consult a Rust risk engine before sending orders. Using traditional IPC:

```
C++ Actor                    Rust Actor
    │                            │
    │ serialize order ─────────> │
    │ (protobuf: ~500ns)         │
    │                            │
    │ write to socket ─────────> │
    │ (syscall: ~1-5μs)          │
    │                            │
    │ context switch ──────────> │
    │ (kernel: ~1-10μs)          │
    │                            │
    │ <─────────── deserialize   │
    │              (~500ns)      │
    │                            │
    │        [process]           │
    │                            │
    │ <─────────── response      │
    │     (same overhead)        │
    └────────────────────────────┘

    Total: 10-30μs round-trip (optimistic)
```

For a system processing 100,000 messages per second, you've just burned 1-3 seconds of latency *per second* on serialization overhead alone. That's before you've done any actual work.

## The Solution: In-Process FFI

What if both actors lived in the same process, communicating through direct function calls?

```
C++ Actor                    Rust Actor (same process)
    │                            │
    │ rust_actor_send() ───────> │
    │ (function call: ~10ns)     │
    │                            │
    │ memcpy C struct ─────────> │
    │ (~20ns for small msg)      │
    │                            │
    │        [process]           │
    │                            │
    │ <─────────── cpp_actor_send()
    │              (~30ns)       │
    └────────────────────────────┘

    Total: ~100ns round-trip
```

That's **100-300x faster** than IPC. At 100,000 messages per second, you're spending 10ms total instead of 2 seconds.

## The Engineering Challenge: Location Transparency

Speed is necessary but not sufficient. The harder problem is **abstraction**: actors shouldn't know where other actors live.

Why does this matter? Because the moment you write this:

```cpp
// BAD: Actor knows it's talking to Rust
RustActorIF rust_risk_engine{"risk_engine", "order_router"};
rust_risk_engine.send(order);
```

You've coupled your architecture to deployment topology. Want to move the risk engine to a separate process for isolation? Rewrite the actor. Want to run both in Rust for a new product? Rewrite again.

The correct abstraction looks identical regardless of where the target lives:

```cpp
// GOOD: Actor doesn't know (or care) where risk_engine lives
ActorRef risk_engine = manager->get_ref("risk_engine");
risk_engine.send(new msg::RiskCheck{order}, this);
```

This is **location transparency**—the fundamental property that makes actor systems composable.

## Implementation: The ActorRef Variant

Our solution extends the `ActorRef` type to handle local, remote, and cross-language targets uniformly:

**C++ ActorRef:**
```cpp
class ActorRef {
    std::variant<LocalActorRef, RemoteActorRef, RustActorRef> ref_;
public:
    void send(const Message* m, Actor* sender = nullptr) {
        std::visit([&](auto& r) { r.send(m, sender); }, ref_);
    }
};
```

**Rust ActorRef:**
```rust
pub enum ActorRef {
    Local { sender: Sender<Box<dyn Message>>, name: String },
    Remote { /* ZMQ socket */ },
    Cpp { target: String, sender: String, send_fn: CppSendFn },
}
```

The `std::visit` in C++ and `match` in Rust dispatch to the appropriate transport. The actor code is identical:

```cpp
// This code works whether risk_engine is C++, Rust, or remote
void on_new_order(const msg::NewOrder* order) {
    risk_engine_.send(new msg::RiskCheck{*order}, this);
}
```

## The FFI Bridge

The magic happens in two bridge functions that translate between languages:

**C++ calling Rust:**
```cpp
// Implemented in Rust, called from C++
extern "C" int32_t rust_actor_send(
    const char* actor_name,
    const char* sender_name,
    int32_t msg_type,
    const void* msg_data  // Pointer to C struct
);
```

**Rust calling C++:**
```rust
// Implemented in C++, called from Rust
extern "C" {
    fn cpp_actor_send(
        actor_name: *const c_char,
        sender_name: *const c_char,
        msg_type: i32,
        msg_data: *const c_void
    ) -> i32;
}
```

Both sides use `extern "C"` linkage with C-compatible structs. No serialization—just `memcpy` of fixed-layout data.

## Message Definition: One Source of Truth

Messages are defined once in a C header:

```c
// messages/interop_messages.h

INTEROP_MESSAGE(RiskCheck, 1020)
typedef struct {
    int64_t order_id;
    char symbol[8];
    double quantity;
    double price;
    int32_t side;  // 0=buy, 1=sell
} RiskCheck;

INTEROP_MESSAGE(RiskResult, 1021)
typedef struct {
    int64_t order_id;
    int32_t approved;
    char reject_reason[64];
} RiskResult;
```

A code generator produces C++ and Rust bindings:

```cpp
// Generated C++
namespace msg {
class RiskCheck : public actors::Message_N<1020> {
public:
    int64_t order_id;
    std::array<char, 8> symbol;
    double quantity;
    double price;
    int32_t side;

    ::RiskCheck to_c_struct() const;
    static RiskCheck from_c_struct(const ::RiskCheck& c);
};
}
```

```rust
// Generated Rust
#[repr(C)]
pub struct RiskCheck {
    pub order_id: i64,
    pub symbol: [u8; 8],
    pub quantity: f64,
    pub price: f64,
    pub side: i32,
}
```

The `#[repr(C)]` attribute ensures Rust uses C-compatible memory layout. Both sides interpret the same bytes identically.

## Actor Lookup: The Registry Problem

For location transparency to work, actors need to find each other by name without knowing the target's language. We solve this with bidirectional lookup:

**C++ looking up actors:**
```cpp
ActorRef InteropManager::get_ref(const std::string& name) {
    // 1. Check local C++ actors
    if (Actor* a = get_actor_by_name(name)) {
        return ActorRef(LocalActorRef(a));
    }
    // 2. Check Rust actors via FFI
    if (rust_actor_exists(name.c_str())) {
        return ActorRef(RustActorRef(name, ""));
    }
    throw std::runtime_error("Actor not found: " + name);
}
```

**Rust looking up actors:**
```rust
pub fn get_actor_ref(name: &str, sender: &str) -> Option<ActorRef> {
    // 1. Check local Rust actors
    if let Some(actor_ref) = manager.get_ref(name) {
        return Some(actor_ref);
    }
    // 2. Check C++ actors via FFI
    if cpp_actor_exists(name) {
        return Some(ActorRef::cpp(name, sender, cpp_send_fn));
    }
    None
}
```

The lookup is lazy—actors resolve references on first use, after both language runtimes have initialized their registries.

## Reply Routing Across Languages

A subtle problem: when a Rust actor sends to C++, how does the C++ actor `reply()`?

The FFI bridge creates a **proxy actor** as the sender:

```cpp
class RustSenderProxy : public actors::Actor {
    std::string rust_actor_name_;
public:
    void send(const Message* m, Actor* sender) noexcept override {
        // Forward to Rust via FFI
        rust_actor_send(rust_actor_name_.c_str(), ..., m->to_c_struct());
    }
};
```

When C++ receives a message from Rust, the bridge passes a `RustSenderProxy` as the sender. The C++ actor calls `reply()` naturally—the proxy intercepts it and routes back to Rust.

```cpp
void on_risk_check(const msg::RiskCheck* m) noexcept {
    bool approved = check_limits(m);
    auto* result = new msg::RiskResult{m->order_id, approved, ""};
    reply(result);  // Works! Uses RustSenderProxy
}
```

## Benchmarks: The Numbers

On a modern Intel Xeon (3.5GHz), typical latencies:

| Transport | Round-trip Latency | Messages/sec |
|-----------|-------------------|--------------|
| In-process FFI | 80-150ns | 7-12M |
| Unix domain socket | 2-5μs | 200-500K |
| TCP localhost | 10-30μs | 30-100K |
| gRPC (protobuf) | 50-200μs | 5-20K |

The FFI path is **50-100x faster** than the next best option. For latency-sensitive systems, this isn't optimization—it's a requirement.

## When to Use FFI vs. IPC

**Use in-process FFI when:**
- Latency matters (sub-microsecond requirements)
- Messages are small and fixed-size
- You control both codebases
- Components share a failure domain anyway

**Use IPC/RPC when:**
- You need process isolation for fault tolerance
- Components have different deployment lifecycles
- Messages are large or variable-sized
- You need language-independent protocols

The beauty of location-transparent `ActorRef` is that you can start with FFI and migrate to IPC later without changing actor code.

## Why C as the Common ABI?

You might ask: why not use a higher-level FFI like SWIG or cxx?

1. **Predictable layout**: C structs have guaranteed memory layout. No hidden vtables, no string allocators, no exceptions across boundaries.

2. **Zero-copy potential**: With careful design, you can pass pointers to shared memory regions without copying.

3. **Minimal runtime**: No garbage collector coordination, no reference counting across boundaries.

4. **Universal support**: Every language can call C functions. Adding Python, Go, or Java becomes straightforward.

The trade-off is manual memory management at the boundary—but actors already manage message lifetimes explicitly.

## Integration with AI Code Generation

A recurring theme in this series: **AI assistants write better code when APIs are uniform**. Location-transparent `ActorRef` means an AI can generate actor code without knowing deployment topology:

```
Human: "Add a rate limiter before the order router"

AI generates:
class RateLimiter : public actors::Actor {
    ActorRef order_router_;
public:
    void on_order(const msg::NewOrder* m) {
        if (check_rate_limit(m->trader_id)) {
            order_router_.send(new msg::NewOrder(*m), this);
        }
    }
};
```

The AI doesn't need to know if `order_router_` is C++, Rust, or remote. The code is correct regardless.

## Conclusion

In-process FFI isn't about avoiding the "cost" of separate processes—sometimes isolation is worth the latency. It's about having the **option** to co-locate components when performance demands it, without sacrificing architectural cleanliness.

The combination of:
- **Location-transparent ActorRef**
- **C ABI for zero-copy messaging**
- **Generated bindings from single source**
- **Bidirectional actor lookup**

...creates a system where language boundaries are deployment decisions, not architectural constraints.

The code is [open source](https://github.com/user/actors-interop) under MIT license. The `ARCHITECTURE.md` file provides detailed documentation for integrating with your own actor systems.


