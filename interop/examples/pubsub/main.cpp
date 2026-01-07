/*
 * Cross-Language Pub/Sub Example
 *
 * Demonstrates C++ subscriber receiving MarketUpdate from Rust publisher.
 *
 * Key feature: Location transparency!
 * The MarketSubscriber doesn't know if the publisher is C++ or Rust.
 *
 * Flow:
 * - C++ MarketSubscriber sends Subscribe("AAPL") to Rust RustPublisher
 * - Rust stores subscription and sends 3 MarketUpdate via reply()
 * - C++ receives updates and prints them
 */

#include <iostream>
#include <cstring>
#include "actors/Actor.hpp"
#include "actors/ActorRef.hpp"
#include "actors/msg/Start.hpp"
#include "InteropMessages.hpp"
#include "InteropManager.hpp"  // Extended Manager for cross-language lookup
#include "CppActorBridge.hpp"
#include <thread>
#include <chrono>

using namespace std;

// Forward declare Rust Manager FFI functions
extern "C" {
    void create_rust_manager();
    void* register_rust_publisher();
    void rust_manager_init();
    void rust_manager_end();
    void rust_actor_init(const void* mgr);
    void rust_actor_shutdown();
    void init_cpp_actor_lookup();  // Register C++ actor lookup for Rust
}

/**
 * MarketSubscriber - subscribes and receives MarketUpdates
 *
 * Note: This actor has NO knowledge that publisher is in Rust.
 * It uses ActorRef from get_ref() - works the same for C++ or Rust targets.
 */
class MarketSubscriber : public actors::Actor {
    actors::ActorRef publisher_ref_;  // ActorRef - works for C++ or Rust!
    actors::Actor* manager_;
    int update_count_ = 0;
    bool publisher_resolved_ = false;

public:
    MarketSubscriber(actors::Actor* mgr)
        : manager_(mgr)
    {
        strncpy(name, "cpp_subscriber", sizeof(name));
        MESSAGE_HANDLER(actors::msg::Start, on_start);
        MESSAGE_HANDLER(msg::MarketUpdate, on_update);
    }

    void on_start(const actors::msg::Start*) noexcept {
        // Get ActorRef by name - location transparent!
        // Works the same whether target is C++ or Rust.
        publisher_ref_ = static_cast<interop::InteropManager*>(manager_)->get_ref("rust_publisher");
        publisher_resolved_ = true;

        cout << "[C++ Subscriber] Starting, subscribing to AAPL via ActorRef..." << endl;

        auto* sub = new msg::Subscribe();
        std::fill(sub->topic.begin(), sub->topic.end(), '\0');
        const char* sym = "AAPL";
        std::copy(sym, sym + 4, sub->topic.begin());

        publisher_ref_.send(sub, this);
    }

    void on_update(const msg::MarketUpdate* m) noexcept {
        std::cerr << "[C++ Subscriber] on_update called" << std::endl;
        update_count_++;

        string symbol(m->symbol.begin(),
            std::find(m->symbol.begin(), m->symbol.end(), '\0'));

        cout << "[C++ Subscriber] Update #" << update_count_
             << ": " << symbol
             << " @ $" << m->price
             << " vol=" << m->volume << endl;

        if (update_count_ >= 3) {
            cout << "[C++ Subscriber] Received all updates, done!" << endl;
            manager_->terminate();
        }
    }
};

/**
 * PubSubManager - uses InteropManager for cross-language actor lookup
 */
class PubSubManager : public interop::InteropManager {
public:
    PubSubManager() {
        auto* sub = new MarketSubscriber(this);
        manage(sub);
    }
};

int main() {
    cout << "=== Cross-Language Pub/Sub Example ===" << endl;
    cout << "C++ Subscriber <--FFI--> Rust Publisher" << endl;
    cout << endl;

    // 1. Create InteropManager
    PubSubManager mgr;

    // 2. Initialize C++ actor bridge
    cpp_actor_init(&mgr);

    // 3. Create Rust Manager
    create_rust_manager();

    // 4. Register rust_publisher actor
    void* rust_mgr = register_rust_publisher();

    // 5. Initialize Rust actor bridge
    rust_actor_init(rust_mgr);

    // 6. Register C++ actor lookup so Rust can find C++ actors
    init_cpp_actor_lookup();

    cout << "[Main] Starting actors..." << endl;
    cout << endl;

    // 6. Start C++ actors
    mgr.init();

    // 7. Start Rust actors
    rust_manager_init();

    // Give time for messages to be processed
    std::this_thread::sleep_for(std::chrono::milliseconds(500));

    // Wait for completion
    mgr.end();

    cout << endl;
    cout << "[Main] Shutting down..." << endl;

    rust_manager_end();
    rust_actor_shutdown();
    cpp_actor_shutdown();

    return 0;
}
