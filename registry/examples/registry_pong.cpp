/*
Registry Pong - Server that registers with GlobalRegistry

Run GlobalRegistry first (python -m actors.registry), then run this,
then run registry_ping in another terminal.

Usage:
    cd ~/actors/cpp && make
    cd ~/actors/registry/examples && make
    ./registry_pong

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

#include <iostream>
#include <memory>
#include <csignal>
#include "actors/Actor.hpp"
#include "actors/ActorRef.hpp"
#include "actors/act/Manager.hpp"
#include "actors/msg/Start.hpp"
#include "actors/remote/Serialization.hpp"
#include "actors/remote/ZmqSender.hpp"
#include "actors/remote/ZmqReceiver.hpp"

using namespace actors;
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

/**
 * PongActor - Receives Ping, sends Pong back
 */
class PongActor : public Actor {
public:
    PongActor() {
        strncpy(name, "pong", sizeof(name));
        MESSAGE_HANDLER(msg::Start, on_start);
        MESSAGE_HANDLER(Ping, on_ping);
    }

private:
    void on_start(const msg::Start*) noexcept {
        cout << "PongActor: Ready to receive pings..." << endl;
    }

    void on_ping(const Ping* msg) noexcept {
        cout << "PongActor: Received ping " << msg->count << " from remote" << endl;
        reply(new Pong(msg->count));
    }
};

// Global manager pointer for signal handler
static Manager* g_manager = nullptr;

void signal_handler(int) {
    if (g_manager) {
        g_manager->terminate();
    }
}

/**
 * PongManager - Sets up pong actor, auto-registers with GlobalRegistry via Manager
 */
class PongManager : public Manager {
    shared_ptr<ZmqSender> zmq_sender_;

public:
    PongManager(const string& registry_endpoint) {
        const string local_endpoint = "tcp://0.0.0.0:5001";

        // Create ZMQ sender for replies
        zmq_sender_ = make_shared<ZmqSender>("tcp://localhost:5001");
        manage(zmq_sender_.get());

        // Connect to GlobalRegistry - this enables auto-registration
        // and allows get_actor_by_name() to lookup remote actors
        set_registry(registry_endpoint, local_endpoint, zmq_sender_);

        // Create pong actor - it will be auto-registered with GlobalRegistry
        auto* pong_actor = new PongActor();
        manage(pong_actor);

        // Create ZMQ receiver and register local actors for incoming messages
        auto* zmq_receiver = new ZmqReceiver(local_endpoint, zmq_sender_);
        zmq_receiver->register_actor("pong", pong_actor);
        manage(zmq_receiver);
    }
};

int main(int argc, char* argv[]) {
    string registry_endpoint = "tcp://localhost:5555";
    if (argc > 1) {
        registry_endpoint = argv[1];
    }

    cout << "=== Registry Pong Process (port 5001) ===" << endl;
    cout << "Registry: " << registry_endpoint << endl;

    // Set up signal handler
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);

    PongManager mgr(registry_endpoint);
    g_manager = &mgr;

    mgr.init();

    cout << "Pong process ready, 'pong' actor auto-registered with GlobalRegistry" << endl;
    cout << "Press Ctrl+C to stop" << endl;

    mgr.end();

    cout << "=== Registry Pong Process Complete ===" << endl;
    return 0;
}
