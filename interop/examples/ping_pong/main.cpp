/*
 * Cross-Language Ping-Pong Example
 *
 * C++ PingActor sends Ping to Rust PongActor via FFI.
 * Rust PongActor sends Pong back to C++ via FFI.
 *
 * Key feature: Location transparency with ActorRef!
 * The PingActor doesn't know or care that PongActor is written in Rust.
 * It just uses manager->get_ref() to get an ActorRef by name.
 * The ActorRef routes to Rust via FFI transparently.
 *
 * Startup sequence:
 * 1. Create InteropManager (extended Manager with Rust lookup)
 * 2. Initialize C++ actor bridge with cpp_actor_init(&mgr)
 * 3. Create Rust Manager with create_rust_manager()
 * 4. Register rust_pong actor with register_rust_pong_actor()
 * 5. Initialize Rust actor bridge with rust_actor_init(rust_mgr_ptr)
 * 6. Start actors with mgr.init()
 * 7. Start Rust actors with rust_manager_init()
 */

#include <iostream>
#include "actors/Actor.hpp"
#include "actors/ActorRef.hpp"
#include "actors/msg/Start.hpp"
#include "InteropMessages.hpp"
#include "InteropManager.hpp"  // Extended Manager for cross-language lookup
#include "CppActorBridge.hpp"

using namespace std;

// Forward declare Rust Manager FFI functions
extern "C" {
    void create_rust_manager();
    void* register_rust_pong_actor();
    void rust_manager_init();
    void rust_manager_end();
    void rust_actor_init(const void* mgr);
    void rust_actor_shutdown();
}

/**
 * PingActor - sends Ping messages and receives Pong replies
 *
 * Note: This actor has NO knowledge that PongActor is in Rust.
 * It just asks the manager for "rust_pong" by name.
 * The returned ActorRef handles routing transparently.
 */
class PingActor : public actors::Actor {
    actors::ActorRef pong_ref_;  // ActorRef - works for C++ or Rust actors!
    actors::Actor* manager_;
    int max_count_;
    bool pong_resolved_ = false;

public:
    PingActor(actors::Actor* mgr, int max = 5)
        : manager_(mgr)
        , max_count_(max)
    {
        strncpy(name, "cpp_ping", sizeof(name));
        MESSAGE_HANDLER(actors::msg::Start, on_start);
        MESSAGE_HANDLER(msg::Pong, on_pong);
    }

    void on_start(const actors::msg::Start*) noexcept {
        // Get an ActorRef by name - location transparent!
        // Works the same whether target is C++ or Rust.
        pong_ref_ = static_cast<interop::InteropManager*>(manager_)->get_ref("rust_pong");
        pong_resolved_ = true;

        cout << "[C++ Ping] Starting cross-language ping-pong!" << endl;
        cout << "[C++ Ping] Sending Ping(1) via ActorRef..." << endl;
        pong_ref_.send(new msg::Ping{1}, this);
    }

    void on_pong(const msg::Pong* m) noexcept {
        cout << "[C++ Ping] Received Pong(" << m->count << ")" << endl;
        if (m->count >= max_count_) {
            cout << "[C++ Ping] Done! Reached max count " << max_count_ << endl;
            manager_->terminate();
        } else {
            cout << "[C++ Ping] Sending Ping(" << m->count + 1 << ") via ActorRef..." << endl;
            pong_ref_.send(new msg::Ping{m->count + 1}, this);
        }
    }
};

/**
 * PingManager - uses InteropManager for cross-language actor lookup
 */
class PingManager : public interop::InteropManager {
public:
    PingManager() {
        auto* ping = new PingActor(this, 5);
        manage(ping);
    }
};

int main() {
    cout << "=== Cross-Language Ping-Pong Example ===" << endl;
    cout << "C++ Ping <--FFI--> Rust Pong" << endl;
    cout << endl;

    // 1. Create InteropManager (extended Manager with Rust lookup)
    PingManager mgr;

    // 2. Initialize C++ actor bridge
    cpp_actor_init(&mgr);

    // 3. Create Rust Manager
    create_rust_manager();

    // 4. Register rust_pong actor
    void* rust_mgr = register_rust_pong_actor();

    // 5. Initialize Rust actor bridge
    rust_actor_init(rust_mgr);

    cout << "[Main] Starting actors..." << endl;
    cout << endl;

    // 6. Start C++ actors
    mgr.init();

    // 7. Start Rust actors
    rust_manager_init();

    // Wait for completion
    mgr.end();

    cout << endl;
    cout << "[Main] Shutting down..." << endl;

    rust_manager_end();
    rust_actor_shutdown();
    cpp_actor_shutdown();

    return 0;
}
