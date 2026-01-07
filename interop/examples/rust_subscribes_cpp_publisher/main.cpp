/*
 * Rust Subscribes to C++ Publisher Example
 *
 * Demonstrates Rust subscriber receiving MarketUpdate from C++ publisher.
 *
 * Key feature: Location transparency!
 * The CppPriceFeed doesn't know if subscribers are C++ or Rust.
 *
 * Flow:
 * - Rust RustSubscriber sends Subscribe to C++ CppPriceFeed on Start
 * - C++ stores subscriber as ActorRef and sends MarketUpdates
 * - Rust receives updates and prints them
 *
 * Startup sequence:
 * 1. Create InteropManager
 * 2. Initialize C++ actor bridge with cpp_actor_init(&mgr)
 * 3. Create Rust Manager with create_rust_manager()
 * 4. Register rust_price_monitor actor
 * 5. Initialize Rust actor bridge with rust_actor_init(rust_mgr_ptr)
 * 6. Start C++ actors with mgr.init()
 * 7. Start Rust actors with rust_manager_init()
 */

#include <iostream>
#include <iomanip>
#include <string>
#include <vector>
#include <unordered_map>
#include <cstring>
#include <ctime>
#include <cstdlib>
#include <thread>
#include <chrono>
#include "actors/Actor.hpp"
#include "actors/ActorRef.hpp"
#include "actors/msg/Start.hpp"
#include "InteropMessages.hpp"
#include "InteropManager.hpp"
#include "CppActorBridge.hpp"

using namespace std;

// Forward declare Rust Manager FFI functions
extern "C" {
    // Rust Manager management
    void create_rust_manager();
    void* register_rust_subscriber();  // Returns Manager pointer
    void* get_rust_manager();
    void rust_manager_init();
    void rust_manager_end();

    // Rust actor bridge (from generated code)
    void rust_actor_init(const void* mgr);
    void rust_actor_shutdown();
    void init_cpp_actor_lookup();  // Register C++ actor lookup for Rust
}

// Subscriber info - now uses ActorRef for location transparency
struct SubscriberInfo {
    string name;
    vector<string> topics;
    actors::ActorRef ref;  // ActorRef works for C++ or Rust actors!
};

// C++ Price Feed Publisher
class CppPriceFeed : public actors::Actor {
    unordered_map<string, SubscriberInfo> subscribers_;
    unordered_map<string, double> prices_;
    int update_count_ = 0;
    interop::InteropManager* manager_;  // Need InteropManager for get_ref()

public:
    CppPriceFeed(interop::InteropManager* mgr) : manager_(mgr) {
        strncpy(name, "cpp_price_feed", sizeof(name));
        MESSAGE_HANDLER(msg::Subscribe, on_subscribe);
        MESSAGE_HANDLER(msg::Unsubscribe, on_unsubscribe);

        // Initialize prices
        prices_["AAPL"] = 150.0;
        prices_["GOOG"] = 2800.0;
        prices_["MSFT"] = 380.0;

        srand(time(nullptr));
        cout << "[C++ Publisher] Created PriceFeed" << endl;
    }

    void publish_updates() {
        // Simulate price changes
        for (auto& [symbol, price] : prices_) {
            double change = (static_cast<double>(rand()) / RAND_MAX - 0.5) * 2.0;
            price += change;
        }

        // Send to all subscribers
        for (auto& [sub_name, sub_info] : subscribers_) {
            for (const auto& topic : sub_info.topics) {
                auto it = prices_.find(topic);
                if (it != prices_.end()) {
                    send_update(sub_info, topic, it->second);
                }
            }
        }

        update_count_++;
        if (update_count_ >= 3) {
            cout << "[C++ Publisher] Sent 3 update rounds, stopping." << endl;
            manager_->terminate();
        }
    }

private:
    void on_subscribe(const msg::Subscribe* m) noexcept {
        string topic(m->topic.begin(),
            find(m->topic.begin(), m->topic.end(), '\0'));

        // Sender name comes from the message routing
        string sender_name = "rust_price_monitor";

        cout << "[C++ Publisher] " << sender_name
             << " subscribing to '" << topic << "'" << endl;

        auto& subscriber = subscribers_[sender_name];
        if (!subscriber.ref) {  // Check if ref is valid
            subscriber.name = sender_name;
            // Get ActorRef from manager - works for C++ or Rust actors!
            subscriber.ref = manager_->get_ref(sender_name);
        }

        auto& topics = subscriber.topics;
        if (find(topics.begin(), topics.end(), topic) == topics.end()) {
            topics.push_back(topic);
        }

        // Send initial price
        auto it = prices_.find(topic);
        if (it != prices_.end()) {
            send_update(subscriber, topic, it->second);
        }
    }

    void on_unsubscribe(const msg::Unsubscribe* m) noexcept {
        string topic(m->topic.begin(),
            find(m->topic.begin(), m->topic.end(), '\0'));

        string sender_name = "rust_price_monitor";

        cout << "[C++ Publisher] " << sender_name
             << " unsubscribing from '" << topic << "'" << endl;

        auto it = subscribers_.find(sender_name);
        if (it != subscribers_.end()) {
            auto& topics = it->second.topics;
            topics.erase(remove(topics.begin(), topics.end(), topic), topics.end());
        }
    }

    void send_update(SubscriberInfo& sub, const string& symbol, double price) {
        auto* update = new msg::MarketUpdate;
        fill(update->symbol.begin(), update->symbol.end(), '\0');
        copy(symbol.begin(),
             symbol.begin() + min(symbol.size(), update->symbol.size() - 1),
             update->symbol.begin());

        update->price = price;
        update->timestamp = static_cast<int64_t>(time(nullptr)) * 1000;
        update->volume = rand() % 10000;

        cout << "[C++ Publisher] Sending " << symbol << " @ $"
             << fixed << setprecision(2) << price << endl;

        // Use ActorRef::send() - works for C++ or Rust actors!
        sub.ref.send(update, this);
    }
};

class PubManager : public interop::InteropManager {
    CppPriceFeed* publisher_;
public:
    PubManager() {
        publisher_ = new CppPriceFeed(this);
        manage(publisher_);  // Actor is found via Manager's get_actor_by_name()
    }

    void publish() {
        publisher_->publish_updates();
    }
};

int main() {
    cout << "=== Rust Subscribes to C++ Publisher ===" << endl;
    cout << "Rust Subscriber <--FFI--> C++ Publisher" << endl;
    cout << endl;

    // 1. Create C++ Manager and publisher actor
    PubManager cpp_mgr;

    // 2. Initialize C++ actor bridge with Manager pointer
    cpp_actor_init(&cpp_mgr);

    // 3. Create Rust Manager
    create_rust_manager();

    // 4. Register rust_price_monitor actor with Rust Manager
    void* rust_mgr = register_rust_subscriber();

    // 5. Initialize Rust actor bridge with Manager pointer
    rust_actor_init(rust_mgr);

    // 6. Register C++ actor lookup so Rust can find C++ actors
    init_cpp_actor_lookup();

    cout << "[Main] Starting actors..." << endl;
    cout << endl;

    // 6. Start C++ actors (sends Start message)
    cpp_mgr.init();

    // 7. Start Rust actors (sends Start message)
    //    RustSubscriber receives Start, sends Subscribe to C++
    rust_manager_init();

    // Give Rust time to subscribe
    this_thread::sleep_for(chrono::milliseconds(100));

    // Publish 3 rounds of updates
    for (int i = 0; i < 3; i++) {
        cout << endl << "[Main] Publishing update round #" << (i + 1) << "..." << endl;
        cpp_mgr.publish();
        this_thread::sleep_for(chrono::milliseconds(100));
    }

    cpp_mgr.end();

    cout << endl;
    cout << "[Main] Shutting down..." << endl;

    // Shutdown Rust Manager
    rust_manager_end();

    // Cleanup
    rust_actor_shutdown();
    cpp_actor_shutdown();

    return 0;
}
