/*
Registry Ping - Client that looks up pong via GlobalRegistry

Run GlobalRegistry first (python -m actors.registry), then registry_pong,
then run this.

Usage:
    cd ~/actors/cpp && make
    cd ~/actors/registry/examples && make
    ./registry_ping

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

#include <iostream>
#include <memory>
#include "actors/Actor.hpp"
#include "actors/ActorRef.hpp"
#include "actors/act/Manager.hpp"
#include "actors/msg/Start.hpp"
#include "actors/remote/Serialization.hpp"
#include "actors/remote/ZmqSender.hpp"
#include "actors/remote/ZmqReceiver.hpp"
#include "actors/registry/RegistryClient.hpp"  // For exception types

using namespace actors;
using namespace actors::registry;
using namespace std;

// Define Ping message (ID=100)
class Ping : public Message_N<100> {
public:
    int count;
    Ping(int c = 0) : count(c) {}
};

// Define Pong message (ID=101)
class Pong : public Message_N<101> {
public:
    int count;
    Pong(int c = 0) : count(c) {}
};

// Register messages for remote serialization
REGISTER_REMOTE_MESSAGE_1(Ping, count, int)
REGISTER_REMOTE_MESSAGE_1(Pong, count, int)

// Forward declaration
class PingManager;

/**
 * PingActor - Sends Ping to pong (looked up via registry), receives Pong back
 */
class PingActor : public Actor {
    Manager* manager_;

public:
    explicit PingActor(Manager* manager)
        : manager_(manager) {
        strncpy(name, "ping", sizeof(name));
        MESSAGE_HANDLER(msg::Start, on_start);
        MESSAGE_HANDLER(Pong, on_pong);
    }

private:
    void on_start(const msg::Start*) noexcept {
        cout << "PingActor: Starting ping-pong..." << endl;

        try {
            // Look up pong actor via Manager (uses GlobalRegistry internally)
            ActorRef pong_ref = manager_->get_actor_by_name("pong");
            cout << "PingActor: Found 'pong' via manager, sending first ping" << endl;
            pong_ref.send(new Ping(1), this);
        } catch (const ActorNotFoundError& e) {
            cerr << "PingActor: Failed to find 'pong': " << e.what() << endl;
            cerr << "Make sure registry_pong is running first!" << endl;
            manager_->terminate();
        } catch (const ActorOfflineError& e) {
            cerr << "PingActor: 'pong' is offline: " << e.what() << endl;
            manager_->terminate();
        }
    }

    void on_pong(const Pong* msg) noexcept {
        cout << "PingActor: Received pong " << msg->count << " from remote" << endl;

        if (msg->count >= 5) {
            cout << "PingActor: Done!" << endl;
            manager_->terminate();
        } else {
            try {
                // Look up pong again (could cache, but this shows the API)
                ActorRef pong_ref = manager_->get_actor_by_name("pong");
                pong_ref.send(new Ping(msg->count + 1), this);
            } catch (const RegistryError& e) {
                cerr << "PingActor: Failed to reach 'pong': " << e.what() << endl;
                manager_->terminate();
            }
        }
    }
};

/**
 * PingManager - Connects to registry, ping actor looks up pong via get_actor_by_name()
 */
class PingManager : public Manager {
    shared_ptr<ZmqSender> zmq_sender_;

public:
    PingManager(const string& registry_endpoint) {
        const string local_endpoint = "tcp://0.0.0.0:5002";

        // Create ZMQ sender
        zmq_sender_ = make_shared<ZmqSender>("tcp://localhost:5002");
        manage(zmq_sender_.get());

        // Connect to GlobalRegistry
        // This allows get_actor_by_name() to lookup remote actors
        set_registry(registry_endpoint, local_endpoint, zmq_sender_);

        // Create ping actor - it will lookup pong via get_actor_by_name() in on_start
        auto* ping_actor = new PingActor(this);
        manage(ping_actor);

        // Create ZMQ receiver for incoming Pong messages
        auto* zmq_receiver = new ZmqReceiver(local_endpoint, zmq_sender_);
        zmq_receiver->register_actor("ping", ping_actor);
        manage(zmq_receiver);
    }
};

int main(int argc, char* argv[]) {
    string registry_endpoint = "tcp://localhost:5555";
    if (argc > 1) {
        registry_endpoint = argv[1];
    }

    cout << "=== Registry Ping Process (port 5002) ===" << endl;
    cout << "Registry: " << registry_endpoint << endl;

    try {
        PingManager mgr(registry_endpoint);
        mgr.init();

        cout << "Ping process starting..." << endl;

        mgr.end();
    } catch (const exception& e) {
        cerr << "Error: " << e.what() << endl;
        return 1;
    }

    cout << "=== Registry Ping Process Complete ===" << endl;
    return 0;
}
