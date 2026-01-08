/*
 * Rust Ping -> C++ Pong Example
 *
 * Demonstrates:
 * 1. Rust actor initiating ping-pong
 * 2. C++ actor receiving and responding
 * 3. Bidirectional FFI communication
 *
 * Flow:
 * - Rust sends Ping #1 to C++
 * - C++ sends Pong #1 back to Rust
 * - Rust sends Ping #2, etc.
 * - After 3 rounds, Rust signals done
 */

#include <iostream>
#include <thread>
#include <chrono>
#include "actors/Actor.hpp"
#include "actors/act/Manager.hpp"
#include "actors/msg/Start.hpp"
#include "InteropMessages.hpp"
#include "CppActorBridge.hpp"

using namespace std;

// Forward declare Rust Manager FFI functions
extern "C" {
    // Rust Manager management
    void create_rust_manager();
    void* register_rust_ping_actor();  // Returns Manager pointer
    void* get_rust_manager();
    void rust_manager_init();
    void rust_manager_end();

    // Rust actor bridge (from generated code)
    void rust_actor_init(const void* mgr);
    void rust_actor_shutdown();
    void init_cpp_actor_lookup();  // Register C++ actor lookup for Rust
}

// C++ Pong Actor - receives Ping, uses reply() to send Pong back
// Note: No need for RustActorIF - the FFI bridge creates a proxy actor
// that allows reply() to work naturally across language boundaries!
class CppPongActor : public actors::Actor {
public:
    CppPongActor() {
        strncpy(name, "cpp_pong", sizeof(name));
        MESSAGE_HANDLER(msg::Ping, on_ping);
    }

    void on_ping(const msg::Ping* m) noexcept {
        cout << "[C++ Pong] Received Ping #" << m->count << endl;

        auto* pong = new msg::Pong();
        pong->count = m->count;

        cout << "[C++ Pong] Using reply() to send Pong #" << pong->count << endl;
        reply(pong);  // Uses RustSenderProxy automatically!
    }
};

class PongManager : public actors::Manager {
public:
    PongManager() {
        auto* pong = new CppPongActor();
        manage(pong);  // Actor is found via Manager's get_actor_by_name()
    }
};

int main() {
    cout << "=== Rust Ping -> C++ Pong Example ===" << endl;
    cout << "Rust initiates, C++ responds" << endl;
    cout << endl;

    // 1. Create C++ Manager and pong actor
    PongManager cpp_mgr;

    // 2. Initialize C++ actor bridge with Manager pointer
    cpp_actor_init(&cpp_mgr);

    // 3. Create Rust Manager
    create_rust_manager();

    // 4. Register C++ actor lookup so Rust can find C++ actors by name
    init_cpp_actor_lookup();

    // 5. Register RustPingActor (needs C++ lookup to find cpp_pong)
    void* rust_mgr = register_rust_ping_actor();

    // 6. Initialize Rust actor bridge with Rust Manager pointer
    rust_actor_init(rust_mgr);

    // 7. Start both Managers (sends Start message to actors)
    cout << "[Main] Starting actors..." << endl << endl;
    cpp_mgr.init();       // C++ actors receive Start
    rust_manager_init();  // Rust actors receive Start

    // Wait for ping-pong to complete (5 rounds)
    this_thread::sleep_for(chrono::milliseconds(500));

    // Shutdown
    rust_manager_end();
    cpp_mgr.end();

    cout << endl << "[Main] Done!" << endl;

    rust_actor_shutdown();
    cpp_actor_shutdown();

    return 0;
}
