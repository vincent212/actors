# Rust Ping -> C++ Pong Example

Demonstrates Rust actor initiating ping-pong communication with a C++ actor via FFI.
This is the reverse direction of the `ping_pong` example.

## What It Does

1. Rust `RustPingActor` receives `Start` message and sends `Ping(1)` to C++ `CppPongActor`
2. C++ receives Ping, uses `reply()` to send `Pong(1)` back to Rust
3. Rust receives Pong, increments count, sends `Ping(2)`
4. Continues until count reaches 3
5. Rust signals completion via `manager_handle.terminate()`

## Architecture

```
                    C++ Side                          Rust Side
                    --------                          ---------

    PongManager                              Rust Manager (via FFI)
        |                                           |
        v                                           v
    CppPongActor  <-- cpp_actor_init() --  RustPingActor
        |                                           |
        |  (reply via RustSenderProxy)              |  (send via CppActorIF)
        v                                           v
    RustSenderProxy  ---- rust_actor_send() ---->  receives Pong
```

## How reply() Works Across FFI

When Rust sends a message to C++, the FFI bridge creates a `RustSenderProxy` actor:

```
Rust PingActor --[Ping]--> CppActorIF.send()
                               |
                               v
                      cpp_actor_send() --> CppActorBridge
                               |
                               v
                      RustSenderProxy (created as sender)
                               |
                               v
                      C++ PongActor.on_ping()
                               |
                          reply(Pong)  <-- uses sender = RustSenderProxy
                               |
                               v
                      RustSenderProxy.send() intercepts
                               |
                               v
                      rust_actor_send() --[Pong]--> Rust PingActor
```

The proxy intercepts `reply()` calls and routes them back to Rust via FFI.

## Startup Sequence

The main() function follows this sequence:

1. **Create C++ Manager** - `PongManager cpp_mgr;`
2. **Initialize C++ bridge** - `cpp_actor_init(&cpp_mgr);` allows Rust to find C++ actors
3. **Create Rust Manager** - `create_rust_manager();`
4. **Register Rust actor** - `register_rust_ping_actor();` returns Manager pointer
5. **Initialize Rust bridge** - `rust_actor_init(rust_mgr);` allows C++ to find Rust actors
6. **Start C++ actors** - `cpp_mgr.init();` sends Start to CppPongActor
7. **Start Rust actors** - `rust_manager_init();` sends Start to RustPingActor (triggers first Ping)

## Build

```bash
cd ~/actors-interop/examples/rust_ping_cpp_pong
make
```

## Run

```bash
./rust_ping_cpp_pong
```

## Expected Output

```
=== Rust Ping -> C++ Pong Example ===
Rust initiates, C++ responds

[Main] Starting actors...

[Rust Ping] Starting ping-pong...
[Rust Ping] Sending Ping #1
[C++ Pong] Received Ping #1
[C++ Pong] Using reply() to send Pong #1
[Rust Ping] Received Pong #1
[Rust Ping] Sending Ping #2
[C++ Pong] Received Ping #2
[C++ Pong] Using reply() to send Pong #2
[Rust Ping] Received Pong #2
[Rust Ping] Sending Ping #3
[C++ Pong] Received Ping #3
[C++ Pong] Using reply() to send Pong #3
[Rust Ping] Received Pong #3
[Rust Ping] Ping-pong complete!

[Main] Done!
```

## Key Files

| File | Description |
|------|-------------|
| `main.cpp` | C++ CppPongActor, PongManager, and main() with startup sequence |
| `rust_ping.rs` | Rust RustPingActor using `handle_messages!` macro |
| `Makefile` | Build script linking C++ and Rust code |

## Code Highlights

### Rust Actor (rust_ping.rs)

```rust
pub struct RustPingActor {
    cpp_pong: CppActorIF,           // Interface to send to C++
    manager_handle: ManagerHandle,   // For termination
}

fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
    let ping = Ping { count: 1 };
    self.cpp_pong.send(&ping);  // Send to C++ via FFI
}

fn on_pong(&mut self, msg: &Pong, _ctx: &mut ActorContext) {
    if msg.count < 3 {
        self.cpp_pong.send(&Ping { count: msg.count + 1 });
    } else {
        self.manager_handle.terminate();  // Signal completion
    }
}

handle_messages!(RustPingActor,
    Start => on_start,
    Pong => on_pong
);
```

### C++ Actor (main.cpp)

```cpp
class CppPongActor : public actors::Actor {
public:
    CppPongActor() {
        strncpy(name, "cpp_pong", sizeof(name));
        MESSAGE_HANDLER(msg::Ping, on_ping);
    }

    void on_ping(const msg::Ping* m) noexcept {
        auto* pong = new msg::Pong();
        pong->count = m->count;
        reply(pong);  // reply() works across FFI via RustSenderProxy!
    }
};
```

## Key Insight

C++ actors don't need `RustActorIF` to reply to Rust actors. The standard `reply()`
mechanism works transparently across the FFI boundary thanks to `RustSenderProxy`.
This makes cross-language communication feel natural - actors just use `reply()`
as they normally would.
