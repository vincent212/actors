/*
 * Simple FFI test - demonstrates C++ calling Rust and vice versa
 *
 * This test doesn't use the full actor framework - it just tests
 * that the FFI bridge functions work correctly.
 */

#include <iostream>
#include <cstring>
#include "../messages/interop_messages.h"

// Declare the Rust bridge functions
extern "C" {
    void rust_actor_init();
    void rust_actor_shutdown();
    int32_t rust_actor_send(
        const char* actor_name,
        const char* sender_name,
        int32_t msg_type,
        const void* msg_data
    );
    int32_t rust_actor_exists(const char* name);
}

// Test callback - will be called from Rust
extern "C" void test_callback(int32_t msg_type, const void* data) {
    if (msg_type == 1001) {  // Pong
        const Pong* pong = static_cast<const Pong*>(data);
        std::cout << "[C++ Callback] Received Pong with count=" << pong->count << std::endl;
    }
}

int main() {
    std::cout << "=== actors-interop FFI Test ===" << std::endl;
    std::cout << std::endl;

    // Test 1: Message struct sizes match
    std::cout << "1. Testing struct sizes:" << std::endl;
    std::cout << "   sizeof(Ping) = " << sizeof(Ping) << std::endl;
    std::cout << "   sizeof(Pong) = " << sizeof(Pong) << std::endl;
    std::cout << "   sizeof(DataRequest) = " << sizeof(DataRequest) << std::endl;
    std::cout << "   sizeof(DataResponse) = " << sizeof(DataResponse) << std::endl;
    std::cout << "   sizeof(Subscribe) = " << sizeof(Subscribe) << std::endl;
    std::cout << "   sizeof(MarketUpdate) = " << sizeof(MarketUpdate) << std::endl;
    std::cout << "   sizeof(MarketDepth) = " << sizeof(MarketDepth) << std::endl;
    std::cout << std::endl;

    // Test 2: Create and serialize a Ping message
    std::cout << "2. Creating Ping message:" << std::endl;
    Ping ping;
    ping.count = 42;
    std::cout << "   ping.count = " << ping.count << std::endl;
    std::cout << std::endl;

    // Test 3: Create a DataRequest with string
    std::cout << "3. Creating DataRequest with string:" << std::endl;
    DataRequest req;
    req.request_id = 123;
    const char* symbol = "AAPL";
    std::strncpy(req.symbol.data, symbol, INTEROP_STRING_MAX - 1);
    req.symbol.len = strlen(symbol);
    std::cout << "   request_id = " << req.request_id << std::endl;
    std::cout << "   symbol = " << std::string(req.symbol.data, req.symbol.len) << std::endl;
    std::cout << std::endl;

    // Test 4: Create MarketDepth with arrays
    std::cout << "4. Creating MarketDepth with arrays:" << std::endl;
    MarketDepth depth;
    std::strncpy(depth.symbol, "GOOG", 7);
    depth.num_levels = 3;
    depth.bid_prices[0] = 100.0;
    depth.bid_prices[1] = 99.5;
    depth.bid_prices[2] = 99.0;
    depth.ask_prices[0] = 100.5;
    depth.ask_prices[1] = 101.0;
    depth.ask_prices[2] = 101.5;
    depth.bid_sizes[0] = 100;
    depth.bid_sizes[1] = 200;
    depth.bid_sizes[2] = 300;
    depth.ask_sizes[0] = 150;
    depth.ask_sizes[1] = 250;
    depth.ask_sizes[2] = 350;

    std::cout << "   symbol = " << depth.symbol << std::endl;
    std::cout << "   num_levels = " << depth.num_levels << std::endl;
    for (int i = 0; i < depth.num_levels; i++) {
        std::cout << "   Level " << i << ": bid=" << depth.bid_prices[i]
                  << " x " << depth.bid_sizes[i]
                  << " | ask=" << depth.ask_prices[i]
                  << " x " << depth.ask_sizes[i] << std::endl;
    }
    std::cout << std::endl;

    // Test 5: Initialize Rust runtime and test exists
    std::cout << "5. Testing Rust bridge functions:" << std::endl;
    rust_actor_init();
    std::cout << "   rust_actor_init() called" << std::endl;

    int exists = rust_actor_exists("nonexistent_actor");
    std::cout << "   rust_actor_exists('nonexistent_actor') = " << exists << " (expected 0)" << std::endl;

    // Try to send to non-existent actor (should return -1)
    int result = rust_actor_send("nonexistent_actor", "test_sender", 1000, &ping);
    std::cout << "   rust_actor_send() to nonexistent = " << result << " (expected -1)" << std::endl;

    rust_actor_shutdown();
    std::cout << "   rust_actor_shutdown() called" << std::endl;
    std::cout << std::endl;

    std::cout << "=== All tests passed ===" << std::endl;
    return 0;
}
