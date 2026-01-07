# C++ Ping -> Rust Pong Example

Demonstrates C++ actor initiating ping-pong communication with a Rust actor via FFI.

## What It Does

1. C++ `PingActor` receives `Start` message and sends `Ping(1)` to Rust `RustPongActor`
2. Rust receives Ping, sends `Pong(1)` back to C++ via `CppActorIF`
3. C++ receives Pong, increments count, sends `Ping(2)`
4. Continues until count reaches 5
5. C++ signals completion via `manager->terminate()`

## Architecture

```
                C++ Side                          Rust Side
                --------                          ---------

    PingManager                              Rust Manager (via FFI)
        |                                           |
        v                                           v
    PingActor  -- rust_actor_send() -->  RustPongActor
        |                                           |
        |  (via RustActorIF)                        |  (via CppActorIF)
        v                                           v
    receives Pong  <-- cpp_actor_send() --  sends Pong
```

## Startup Sequence

The main() function follows this sequence:

1. **Create C++ Manager** - `PingManager cpp_mgr;`
   - Creates PingActor and registers it via `manage(ping)`
   - Actor name is set via `strncpy(name, "cpp_ping", ...)`

2. **Initialize C++ bridge** - `cpp_actor_init(&cpp_mgr);`
   - Stores Manager pointer so Rust can find C++ actors via `get_actor_by_name()`

3. **Create Rust Manager** - `create_rust_manager();`
   - Creates a new Rust Manager instance

4. **Register Rust actor** - `register_rust_pong_actor();`
   - Creates `RustPongActor` with name "rust_pong"
   - Returns Manager pointer for bridge initialization

5. **Initialize Rust bridge** - `rust_actor_init(rust_mgr);`
   - Stores Rust Manager pointer so C++ can find Rust actors via `get_ref()`

6. **Start C++ actors** - `cpp_mgr.init();`
   - Sends `Start` message to all managed actors
   - PingActor receives Start, sends first Ping to Rust

7. **Start Rust actors** - `rust_manager_init();`
   - Sends `Start` message to Rust actors
   - (RustPongActor doesn't need Start to respond to Ping)

8. **Wait for completion** - `cpp_mgr.end();`
   - Blocks until `terminate()` is called

9. **Cleanup** - Shutdown Rust Manager and bridges

## Build

```bash
cd ~/actors-interop/examples/ping_pong
make
```

## Run

```bash
./ping_pong
```

## Expected Output

```
=== Cross-Language Ping-Pong Example ===
C++ Ping <--FFI--> Rust Pong

[Main] Starting actors...

[C++ Ping] Starting cross-language ping-pong!
[C++ Ping] Sending Ping(1) to Rust...
[Rust Pong] Received Ping #1
[Rust Pong] Sending Pong #1 back to C++...
[C++ Ping] Received Pong(1) from Rust
[C++ Ping] Sending Ping(2) to Rust...
[Rust Pong] Received Ping #2
[Rust Pong] Sending Pong #2 back to C++...
[C++ Ping] Received Pong(2) from Rust
[C++ Ping] Sending Ping(3) to Rust...
[Rust Pong] Received Ping #3
[Rust Pong] Sending Pong #3 back to C++...
[C++ Ping] Received Pong(3) from Rust
[C++ Ping] Sending Ping(4) to Rust...
[Rust Pong] Received Ping #4
[Rust Pong] Sending Pong #4 back to C++...
[C++ Ping] Received Pong(4) from Rust
[C++ Ping] Sending Ping(5) to Rust...
[Rust Pong] Received Ping #5
[Rust Pong] Sending Pong #5 back to C++...
[C++ Ping] Received Pong(5) from Rust
[C++ Ping] Done! Reached max count 5

[Main] Shutting down...
```

## Key Files

| File | Description |
|------|-------------|
| `main.cpp` | C++ PingActor, PingManager, and main() with startup sequence |
| `rust_pong.rs` | Rust RustPongActor using `handle_messages!` macro |
| `Makefile` | Build script linking C++ and Rust code |

## Code Highlights

### C++ Actor (main.cpp)

```cpp
class PingActor : public actors::Actor {
    interop::RustActorIF pong_actor_;  // Interface to Rust actor
    actors::Actor* manager_;
    int max_count_;

public:
    PingActor(actors::Actor* mgr, int max = 5)
        : pong_actor_("rust_pong", "cpp_ping")  // target, sender
        , manager_(mgr)
        , max_count_(max)
    {
        strncpy(name, "cpp_ping", sizeof(name));
        MESSAGE_HANDLER(actors::msg::Start, on_start);
        MESSAGE_HANDLER(msg::Pong, on_pong);
    }

    void on_start(const actors::msg::Start*) noexcept {
        pong_actor_.send(msg::Ping{1});  // Send to Rust via FFI
    }

    void on_pong(const msg::Pong* m) noexcept {
        if (m->count >= max_count_) {
            manager_->terminate();  // Signal completion
        } else {
            pong_actor_.send(msg::Ping{m->count + 1});
        }
    }
};
```

### Rust Actor (rust_pong.rs)

```rust
pub struct RustPongActor {
    cpp_ping: CppActorIF,           // Interface to send to C++
    manager_handle: ManagerHandle,
}

fn on_ping(&mut self, msg: &Ping, _ctx: &mut ActorContext) {
    let pong = Pong { count: msg.count };
    self.cpp_ping.send(&pong);  // Send to C++ via FFI
}

handle_messages!(RustPongActor,
    Ping => on_ping
);
```

## Key Insight

This example demonstrates explicit cross-language communication using:
- `RustActorIF` - C++ class for sending to Rust actors
- `CppActorIF` - Rust struct for sending to C++ actors

Both use the FFI bridge functions (`rust_actor_send()`, `cpp_actor_send()`) to route
messages across the language boundary.
