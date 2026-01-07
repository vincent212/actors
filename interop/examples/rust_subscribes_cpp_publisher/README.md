# Rust Subscriber <- C++ Publisher

Demonstrates Rust actor subscribing to a C++ market data publisher via FFI.
This is the reverse direction of the `pubsub` example.

## What It Does

1. Rust `RustSubscriber` receives `Start` and sends `Subscribe("AAPL")` and `Subscribe("GOOG")` to C++ `CppPriceFeed`
2. C++ stores subscribers and sends initial prices
3. C++ publishes 3 rounds of `MarketUpdate` messages with price changes
4. Rust receives and displays all updates
5. C++ signals completion via `manager->terminate()`

## Architecture

```
                C++ Side                          Rust Side
                --------                          ---------

    PubManager                               Rust Manager (via FFI)
        |                                           |
        v                                           v
    CppPriceFeed  <-- cpp_actor_send() --  RustSubscriber
        |                                           |
        |  (via RustActorIF)                        |  (via CppActorIF)
        v                                           v
    sends MarketUpdate  -- rust_actor_send() -->  receives MarketUpdate
```

## Startup Sequence

The main() function follows this sequence:

1. **Create C++ Manager** - `PubManager cpp_mgr;`
   - Creates CppPriceFeed and registers it via `manage(publisher)`
   - Actor name is set via `strncpy(name, "cpp_price_feed", ...)`

2. **Initialize C++ bridge** - `cpp_actor_init(&cpp_mgr);`
   - Stores Manager pointer so Rust can find C++ actors via `get_actor_by_name()`

3. **Create Rust Manager** - `create_rust_manager();`
   - Creates a new Rust Manager instance

4. **Register Rust actor** - `register_rust_subscriber();`
   - Creates `RustSubscriber` with name "rust_price_monitor"
   - Returns Manager pointer for bridge initialization

5. **Initialize Rust bridge** - `rust_actor_init(rust_mgr);`
   - Stores Rust Manager pointer so C++ can find Rust actors via `get_ref()`

6. **Start C++ actors** - `cpp_mgr.init();`
   - Sends `Start` message to all managed actors

7. **Start Rust actors** - `rust_manager_init();`
   - Sends `Start` message to Rust actors
   - RustSubscriber receives Start, sends Subscribe to C++

8. **Publish updates** - `cpp_mgr.publish();` (called 3 times)
   - C++ publishes price updates to all subscribers

9. **Wait for completion** - `cpp_mgr.end();`
   - Blocks until `terminate()` is called

10. **Cleanup** - Shutdown Rust Manager and bridges

## Build

```bash
cd ~/actors-interop/examples/rust_subscribes_cpp_publisher
make
```

## Run

```bash
./rust_subscribes_cpp_publisher
```

## Expected Output

```
=== Rust Subscribes to C++ Publisher ===
Rust Subscriber <--FFI--> C++ Publisher

[C++ Publisher] Created PriceFeed
[Main] Starting actors...

[Rust Subscriber] Started, subscribing to AAPL and GOOG...
[Rust Subscriber] Subscribing to AAPL
[Rust Subscriber] Subscribing to GOOG
[C++ Publisher] rust_price_monitor subscribing to 'AAPL'
[C++ Publisher] Sending AAPL @ $150.00
[C++ Publisher] rust_price_monitor subscribing to 'GOOG'
[C++ Publisher] Sending GOOG @ $2800.00
[Rust Subscriber] Update #1: AAPL @ 150.00 vol=...
[Rust Subscriber] Update #2: GOOG @ 2800.00 vol=...

[Main] Publishing update round #1...
[C++ Publisher] Sending AAPL @ $149.xx
[C++ Publisher] Sending GOOG @ $2800.xx
[Rust Subscriber] Update #3: AAPL @ 149.xx vol=...
[Rust Subscriber] Update #4: GOOG @ 2800.xx vol=...

[Main] Publishing update round #2...
...
[Main] Publishing update round #3...
[C++ Publisher] Sent 3 update rounds, stopping.
...

[Main] Shutting down...
```

## Key Files

| File | Description |
|------|-------------|
| `main.cpp` | C++ CppPriceFeed publisher, PubManager, and main() with startup sequence |
| `rust_subscriber.rs` | Rust RustSubscriber using `handle_messages!` macro |
| `Makefile` | Build script linking C++ and Rust code |

## Code Highlights

### C++ Publisher (main.cpp)

```cpp
class CppPriceFeed : public actors::Actor {
    unordered_map<string, SubscriberInfo> subscribers_;
    unordered_map<string, double> prices_;

public:
    CppPriceFeed(actors::Actor* mgr) {
        strncpy(name, "cpp_price_feed", sizeof(name));
        MESSAGE_HANDLER(msg::Subscribe, on_subscribe);
        MESSAGE_HANDLER(msg::Unsubscribe, on_unsubscribe);
    }

    void on_subscribe(const msg::Subscribe* m) noexcept {
        string topic = /* extract from m */;
        string sender_name = "rust_price_monitor";

        // Create RustActorIF to send updates to the subscriber
        subscriber.rust_if = new interop::RustActorIF(sender_name, "cpp_price_feed");

        // Send initial price
        send_update(subscriber, topic, prices_[topic]);
    }

    void publish_updates() {
        // Simulate price changes and send to all subscribers
        for (auto& [sub_name, sub_info] : subscribers_) {
            for (const auto& topic : sub_info.topics) {
                sub_info.rust_if->send(update);  // Send to Rust via FFI
            }
        }
    }
};
```

### Rust Subscriber (rust_subscriber.rs)

```rust
pub struct RustSubscriber {
    publisher: CppActorIF,           // Interface to C++ publisher
    update_count: i32,
    subscribed_topics: Vec<String>,
    manager_handle: ManagerHandle,
}

fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
    self.subscribe("AAPL");
    self.subscribe("GOOG");
}

fn subscribe(&mut self, symbol: &str) {
    let sub = Subscribe { topic: /* ... */ };
    self.publisher.send(&sub);  // Send to C++ via FFI
}

fn on_market_update(&mut self, msg: &MarketUpdate, _ctx: &mut ActorContext) {
    self.update_count += 1;
    println!("Update #{}: {} @ {:.2}", self.update_count, symbol, msg.price);
}

handle_messages!(RustSubscriber,
    Start => on_start,
    MarketUpdate => on_market_update,
    MarketDepth => on_market_depth
);
```

## Key Insight

This example demonstrates the reverse pub/sub pattern:
- Rust subscriber sends `Subscribe` message to C++ publisher
- C++ publisher sends `MarketUpdate` messages to Rust subscriber
- C++ uses `RustActorIF` to route messages to Rust via FFI
- Rust uses `CppActorIF` to route messages to C++ via FFI
