# C++ Subscriber <- Rust Publisher (Pub/Sub)

Demonstrates C++ actor subscribing to a Rust market data publisher via FFI.

## What It Does

1. C++ `MarketSubscriber` receives `Start` and sends `Subscribe("AAPL")` to Rust `RustPublisher`
2. Rust receives Subscribe, sends 3 `MarketUpdate` messages back via `CppActorIF`
3. C++ receives updates and displays prices
4. After 3 updates, C++ signals completion via `manager->terminate()`

## Architecture

```
                C++ Side                          Rust Side
                --------                          ---------

    PubSubManager                            Rust Manager (via FFI)
        |                                           |
        v                                           v
    MarketSubscriber  -- rust_actor_send() -->  RustPublisher
        |                                           |
        |  (via RustActorIF)                        |  (via CppActorIF)
        v                                           v
    receives MarketUpdate  <-- cpp_actor_send() --  sends MarketUpdate
```

## Startup Sequence

The main() function follows this sequence:

1. **Create C++ Manager** - `PubSubManager cpp_mgr;`
   - Creates MarketSubscriber and registers it via `manage(sub)`
   - Actor name is set via `strncpy(name, "cpp_subscriber", ...)`

2. **Initialize C++ bridge** - `cpp_actor_init(&cpp_mgr);`
   - Stores Manager pointer so Rust can find C++ actors via `get_actor_by_name()`

3. **Create Rust Manager** - `create_rust_manager();`
   - Creates a new Rust Manager instance

4. **Register Rust actor** - `register_rust_publisher();`
   - Creates `RustPublisher` with name "rust_publisher"
   - Returns Manager pointer for bridge initialization

5. **Initialize Rust bridge** - `rust_actor_init(rust_mgr);`
   - Stores Rust Manager pointer so C++ can find Rust actors via `get_ref()`

6. **Start C++ actors** - `cpp_mgr.init();`
   - Sends `Start` message to all managed actors
   - MarketSubscriber receives Start, sends Subscribe to Rust

7. **Start Rust actors** - `rust_manager_init();`
   - Sends `Start` message to Rust actors

8. **Wait for completion** - `cpp_mgr.end();`
   - Blocks until `terminate()` is called

9. **Cleanup** - Shutdown Rust Manager and bridges

## Build

```bash
cd ~/actors-interop/examples/pubsub
make
```

## Run

```bash
./pubsub
```

## Expected Output

```
=== Cross-Language Pub/Sub Example ===
C++ Subscriber <--FFI--> Rust Publisher

[Main] Starting actors...

[C++ Subscriber] Starting, subscribing to AAPL...
[Rust Publisher] Subscriber subscribing to 'AAPL'
[Rust Publisher] Sending update: AAPL @ $150.00
[Rust Publisher] Sending update: AAPL @ $150.25
[Rust Publisher] Sending update: AAPL @ $150.50
[Rust Publisher] Started
[C++ Subscriber] Update #1: AAPL @ $150 vol=100
[C++ Subscriber] Update #2: AAPL @ $150.25 vol=200
[C++ Subscriber] Update #3: AAPL @ $150.5 vol=300
[C++ Subscriber] Received all updates, done!

[Main] Shutting down...
```

## Key Files

| File | Description |
|------|-------------|
| `main.cpp` | C++ MarketSubscriber, PubSubManager, and main() with startup sequence |
| `rust_publisher.rs` | Rust RustPublisher using `handle_messages!` macro |
| `Makefile` | Build script linking C++ and Rust code |

## Code Highlights

### C++ Subscriber (main.cpp)

```cpp
class MarketSubscriber : public actors::Actor {
    interop::RustActorIF publisher_;  // Interface to Rust publisher
    actors::Actor* manager_;
    int update_count_ = 0;

public:
    MarketSubscriber(actors::Actor* mgr)
        : publisher_("rust_publisher", "cpp_subscriber")  // target, sender
        , manager_(mgr)
    {
        strncpy(name, "cpp_subscriber", sizeof(name));
        MESSAGE_HANDLER(actors::msg::Start, on_start);
        MESSAGE_HANDLER(msg::MarketUpdate, on_update);
    }

    void on_start(const actors::msg::Start*) noexcept {
        msg::Subscribe sub;
        // ... fill topic ...
        publisher_.send(sub);  // Send to Rust via FFI
    }

    void on_update(const msg::MarketUpdate* m) noexcept {
        update_count_++;
        // ... print update ...
        if (update_count_ >= 3) {
            manager_->terminate();
        }
    }
};
```

### Rust Publisher (rust_publisher.rs)

```rust
pub struct RustPublisher {
    cpp_subscriber: CppActorIF,  // Interface to send to C++
    topics: Vec<String>,
    manager_handle: ManagerHandle,
}

fn on_subscribe(&mut self, msg: &Subscribe, _ctx: &mut ActorContext) {
    let topic = /* extract from msg */;

    // Send 3 updates via CppActorIF to the C++ subscriber
    for i in 0..3 {
        let update = MarketUpdate { /* ... */ };
        self.cpp_subscriber.send(&update);  // Send to C++ via FFI
    }
}

handle_messages!(RustPublisher,
    Start => on_start,
    Subscribe => on_subscribe
);
```

## Key Insight

This example demonstrates the pub/sub pattern across FFI:
- C++ subscriber sends `Subscribe` message to Rust publisher
- Rust publisher sends `MarketUpdate` messages back to C++ subscriber
- Both use the FFI bridge functions (`rust_actor_send()`, `cpp_actor_send()`) to route messages
