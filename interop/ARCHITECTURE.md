# actors-interop Architecture Guide

This document explains the actors-interop FFI layer for AI systems and developers.

## Overview

actors-interop enables **location-transparent** communication between C++ and Rust actors via FFI (Foreign Function Interface). Actors use `ActorRef` to send messages without knowing whether the target is local, cross-language (FFI), or remote (ZMQ).

## Core Principle: Location Transparency

**Actors MUST NOT know where other actors live.** This is the fundamental design principle.

```cpp
// CORRECT - Actor uses ActorRef, doesn't know target language
ActorRef publisher = manager->get_ref("rust_publisher");
publisher.send(new msg::Subscribe{...}, this);

// WRONG - Actor explicitly uses FFI interface (violates transparency)
RustActorIF rust_pub{"rust_publisher", "my_actor"};  // DON'T DO THIS
rust_pub.send(msg);
```

```rust
// CORRECT - Actor uses ActorRef from get_actor_ref()
let publisher = get_actor_ref("cpp_publisher", "my_actor");
publisher.send(Box::new(Subscribe{...}), None);

// WRONG - Actor explicitly uses CppActorIF (violates transparency)
let cpp_pub = CppActorIF::new("cpp_publisher", Some("my_actor"));  // DON'T DO THIS
```

## Directory Structure

```
actors-interop/
├── messages/
│   └── interop_messages.h      # C struct message definitions (source of truth)
├── codegen/
│   └── generate.py             # Generates C++/Rust code from messages
├── generated/
│   ├── cpp/
│   │   ├── InteropMessages.hpp # C++ message classes with to/from_c_struct()
│   │   ├── CppActorBridge.cpp  # FFI bridge: Rust -> C++
│   │   └── InteropManager.hpp  # Extended Manager with get_ref() for Rust lookup
│   └── rust/
│       ├── interop_messages.rs # Rust message structs with to/from_c_struct()
│       └── rust_actor_bridge.rs # FFI bridge: C++ -> Rust
├── cpp/
│   ├── include/interop/
│   │   └── RustActorIF.hpp     # Low-level FFI wrapper (used by RustActorRef)
│   └── src/
│       └── RustActorRef.cpp    # RustActorRef::send() implementation
├── rust/
│   └── src/
│       ├── lib.rs              # Crate root
│       ├── cpp_actor_if.rs     # Low-level FFI wrapper (used by ActorRef::Cpp)
│       └── rust_manager_ffi.rs # get_actor_ref(), cpp_send_fn, init_cpp_actor_lookup
└── examples/
    ├── ping_pong/              # C++ initiates -> Rust responds
    ├── pubsub/                 # C++ subscribes -> Rust publishes
    ├── rust_ping_cpp_pong/     # Rust initiates -> C++ responds
    └── rust_subscribes_cpp_publisher/  # Rust subscribes -> C++ publishes
```

## ActorRef Variants

### C++ ActorRef (actors-cpp)

```cpp
// File: /home/vm/actors-cpp/include/actors/ActorRef.hpp

class ActorRef {
    std::variant<LocalActorRef, RemoteActorRef, RustActorRef> ref_;
public:
    void send(const Message* m, Actor* sender = nullptr) {
        std::visit([&](auto& r) { r.send(m, sender); }, ref_);
    }
};
```

- `LocalActorRef` - Target is a C++ actor in same process
- `RemoteActorRef` - Target is remote via ZMQ
- `RustActorRef` - Target is a Rust actor (FFI)

### Rust ActorRef (actors-rust)

```rust
// File: /home/vm/actors-rust/src/actor_ref.rs

pub enum ActorRef {
    Local { sender: Sender<...>, name: String },
    Remote { ... },
    Cpp { target: String, sender: String, send_fn: CppSendFn },
}

impl ActorRef {
    pub fn send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) {
        match self {
            ActorRef::Local { sender, .. } => sender.send(msg),
            ActorRef::Cpp { target, sender, send_fn } => {
                send_fn(target, sender, msg.as_ref());  // FFI call
            }
            ...
        }
    }
}
```

## Actor Lookup Functions

### C++ Side: InteropManager::get_ref()

```cpp
// File: /home/vm/actors-interop/generated/cpp/InteropManager.hpp

class InteropManager : public actors::Manager {
public:
    actors::ActorRef get_ref(const std::string& name) {
        // 1. Check local C++ actors
        if (Actor* a = get_actor_by_name(name)) {
            return ActorRef(a);  // LocalActorRef
        }
        // 2. Check Rust actors via FFI
        if (rust_actor_exists(name.c_str())) {
            return ActorRef(RustActorRef(name, ""));  // RustActorRef
        }
        throw std::runtime_error("Actor not found: " + name);
    }
};
```

### Rust Side: get_actor_ref()

```rust
// File: /home/vm/actors-interop/rust/src/rust_manager_ffi.rs

pub fn get_actor_ref(name: &str, sender: &str) -> Option<ActorRef> {
    // 1. Check local Rust actors via Manager
    if let Some(ref_) = manager.get_ref(name) {
        return Some(ref_);
    }
    // 2. Check C++ actors via FFI
    if cpp_actor_exists(name) {
        return Some(ActorRef::cpp(name, sender, cpp_send_fn));
    }
    None
}
```

## FFI Bridge Functions

### C++ -> Rust: rust_actor_send()

```cpp
// Declared in generated/rust (implemented in Rust)
extern "C" int32_t rust_actor_send(
    const char* actor_name,    // Target Rust actor
    const char* sender_name,   // Sender name (for reply routing)
    int32_t msg_type,          // Message ID (e.g., 1000 for Ping)
    const void* msg_data       // Pointer to C struct
);
```

Called by `RustActorRef::send()` in C++.

### Rust -> C++: cpp_actor_send()

```rust
// Declared in rust/src/rust_manager_ffi.rs (implemented in C++)
extern "C" {
    fn cpp_actor_send(
        actor_name: *const c_char,
        sender_name: *const c_char,
        msg_type: i32,
        msg_data: *const c_void
    ) -> i32;
}
```

Called by `cpp_send_fn()` which is stored in `ActorRef::Cpp`.

## Message Flow Examples

### C++ Actor Sends to Rust Actor

```
1. C++ Actor calls: publisher_ref_.send(new msg::Subscribe{...}, this)
2. ActorRef dispatches to RustActorRef::send()
3. RustActorRef::send():
   a. Converts msg::Subscribe to C struct (to_c_struct())
   b. Calls rust_actor_send("rust_publisher", "cpp_subscriber", 1010, &c_struct)
4. Rust rust_actor_send():
   a. Looks up "rust_publisher" in Rust Manager
   b. Converts C struct to Rust Subscribe (from_c_struct())
   c. Sends to actor's channel
5. Rust Actor receives Subscribe in handle_messages! macro
```

### Rust Actor Sends to C++ Actor

```
1. Rust Actor calls: publisher.send(Box::new(Subscribe{...}), None)
2. ActorRef::Cpp dispatches to cpp_send_fn()
3. cpp_send_fn():
   a. Gets message_id from msg.message_id()
   b. Downcasts to concrete type, converts to C struct
   c. Calls cpp_actor_send("cpp_publisher", "rust_subscriber", 1010, &c_struct)
4. C++ cpp_actor_send():
   a. Looks up "cpp_publisher" in Manager
   b. Converts C struct to msg::Subscribe (from_c_struct())
   c. Calls actor->send(cpp_msg, sender_proxy)
5. C++ Actor receives Subscribe in MESSAGE_HANDLER
```

## Initialization Sequence

Every interop example follows this pattern:

```cpp
// main.cpp

int main() {
    // 1. Create C++ Manager (InteropManager for cross-language lookup)
    PubSubManager cpp_mgr;

    // 2. Initialize C++ actor bridge (stores Manager pointer)
    cpp_actor_init(&cpp_mgr);

    // 3. Create Rust Manager
    create_rust_manager();

    // 4. Register Rust actors
    void* rust_mgr = register_rust_publisher();  // Example-specific

    // 5. Initialize Rust actor bridge
    rust_actor_init(rust_mgr);

    // 6. Register C++ actor lookup for Rust (CRITICAL!)
    init_cpp_actor_lookup();

    // 7. Start both managers
    cpp_mgr.init();       // Sends Start to C++ actors
    rust_manager_init();  // Sends Start to Rust actors

    // ... run ...

    // Shutdown
    cpp_mgr.end();
    rust_manager_end();
    rust_actor_shutdown();
    cpp_actor_shutdown();
}
```

## Message Definition

Messages are defined in C header format:

```c
// File: messages/interop_messages.h

INTEROP_MESSAGE(Subscribe, 1010)
typedef struct {
    char topic[32];          // Fixed-size char array
} Subscribe;

INTEROP_MESSAGE(MarketUpdate, 1012)
typedef struct {
    char symbol[8];
    double price;
    int64_t timestamp;
    int32_t volume;
} MarketUpdate;
```

Code generator produces:

**C++ (InteropMessages.hpp):**
```cpp
namespace msg {
class Subscribe : public actors::Message_N<1010> {
public:
    std::array<char, 32> topic;

    ::Subscribe to_c_struct() const;
    static Subscribe from_c_struct(const ::Subscribe& c);
};
}
```

**Rust (interop_messages.rs):**
```rust
pub const MSG_SUBSCRIBE: i32 = 1010;

#[repr(C)]
pub struct Subscribe {
    pub topic: [u8; 32],
}

impl Subscribe {
    pub fn to_c_struct(&self) -> CSubscribe { ... }
    pub fn from_c_struct(c: &CSubscribe) -> Self { ... }
}
```

## Key Files Reference

| File | Purpose |
|------|---------|
| `messages/interop_messages.h` | Message definitions (edit to add new messages) |
| `codegen/generate.py` | Code generator |
| `generated/cpp/InteropMessages.hpp` | Generated C++ message classes |
| `generated/cpp/CppActorBridge.cpp` | FFI entry point for Rust->C++ |
| `generated/cpp/InteropManager.hpp` | Manager with get_ref() for Rust lookup |
| `generated/rust/interop_messages.rs` | Generated Rust message structs |
| `generated/rust/rust_actor_bridge.rs` | FFI entry point for C++->Rust |
| `rust/src/rust_manager_ffi.rs` | get_actor_ref(), cpp_send_fn, lookup functions |
| `cpp/src/RustActorRef.cpp` | RustActorRef::send() implementation |

## Common Patterns

### Reply to Cross-Language Message

C++ receiving from Rust can use `reply()` naturally:

```cpp
void on_ping(const msg::Ping* m) noexcept {
    auto* pong = new msg::Pong{m->count};
    reply(pong);  // Works! Uses RustSenderProxy
}
```

The FFI bridge creates a `RustSenderProxy` as the sender, which forwards `reply()` calls back to Rust.

### Lazy Actor Lookup

Rust actors look up targets on first use:

```rust
fn get_publisher(&mut self) -> Option<ActorRef> {
    if self.publisher.is_none() {
        self.publisher = get_actor_ref("cpp_publisher", "rust_subscriber");
    }
    self.publisher.clone()
}
```

### Pub/Sub with Mixed Languages

Publisher stores ActorRefs from subscribers:

```cpp
// C++ Publisher
std::unordered_map<std::string, ActorRef> subscribers_;

void on_subscribe(const msg::Subscribe* m) noexcept {
    std::string sender_name = "rust_subscriber";  // From message context
    subscribers_[sender_name] = manager_->get_ref(sender_name);
}

void publish(const msg::MarketUpdate& update) {
    for (auto& [name, ref] : subscribers_) {
        ref.send(new msg::MarketUpdate(update), this);  // Works for C++ or Rust!
    }
}
```

## Debugging Tips

1. **Actor not found**: Ensure `init_cpp_actor_lookup()` is called after `rust_actor_init()`
2. **Message not delivered**: Check message ID matches in both C++ and Rust
3. **Segfault in handler**: Verify `ACTOR_HANDLER_CACHE_SIZE` >= max message ID (default 2048)
4. **Reply not received**: Ensure sender name is passed correctly through FFI

## Adding New Examples

1. Create directory under `examples/`
2. Add Makefile (copy from existing example)
3. Create C++ main.cpp following initialization sequence
4. Create Rust actor in `rust/src/` with FFI exports
5. Update `rust/src/lib.rs` to include new module
