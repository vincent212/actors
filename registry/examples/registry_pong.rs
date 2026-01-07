/*
Registry Pong - Server that registers with GlobalRegistry

Run GlobalRegistry first (python -m actors.registry), then run this,
then run registry_ping in another terminal.

Usage:
    cd ~/actors/rust
    cargo run --example registry_pong

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use actors::{
    ActorContext, Manager, ThreadConfig, ManagerHandle,
    ZmqReceiver, ZmqSender, register_remote_message, define_message, handle_messages,
    RegistryClient, RegistryError,
};

// Define messages
#[derive(Serialize, Deserialize, Default)]
struct Ping {
    count: i32,
}
define_message!(Ping);

#[derive(Serialize, Deserialize, Default)]
struct Pong {
    count: i32,
}
define_message!(Pong);

/// PongActor receives Ping from remote, sends Pong back.
struct PongActor;

handle_messages!(PongActor,
    Ping => on_ping
);

impl PongActor {
    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        println!("PongActor: Received ping {} from remote", msg.count);
        ctx.reply(Box::new(Pong { count: msg.count }));
    }
}

fn main() {
    let registry_endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tcp://localhost:5555".to_string());

    let endpoint = "tcp://0.0.0.0:5001";
    let local_endpoint = "tcp://localhost:5001";

    println!("=== Registry Pong Process (port 5001) ===");
    println!("Registry: {}", registry_endpoint);

    // Register messages for remote serialization
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");

    // Create registry client and start heartbeat
    let registry_client = RegistryClient::new("PongManager", &registry_endpoint);
    registry_client.start_heartbeat();

    // Create ZMQ sender for replies
    let zmq_sender = Arc::new(ZmqSender::new(local_endpoint));

    // Create manager and actors
    let mut mgr = Manager::new();
    let handle = mgr.get_handle();
    let pong_ref = mgr.manage("pong", Box::new(PongActor), ThreadConfig::default());

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("pong", pong_ref);

    // Register pong actor with GlobalRegistry
    match registry_client.register("pong", local_endpoint) {
        Ok(_) => println!("Registered 'pong' with GlobalRegistry at {}", local_endpoint),
        Err(e) => {
            eprintln!("Failed to register with registry: {}", e);
            return;
        }
    }

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    // Set up signal handler
    let handle_clone = handle.clone();
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        handle_clone.terminate();
    }).expect("Error setting Ctrl-C handler");

    mgr.init();
    println!("Pong process ready, 'pong' actor registered with GlobalRegistry");
    println!("Press Ctrl+C to stop");

    // Wait until terminated
    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Registry Pong Process Complete ===");
}
