/*
Registry Ping - Client that looks up pong via GlobalRegistry

Run GlobalRegistry first (python -m actors.registry), then registry_pong,
then run this.

Usage:
    cd ~/actors/rust
    cargo run --example registry_ping

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use actors::{
    Actor, ActorContext, ActorRef, Manager, ThreadConfig, ManagerHandle,
    ZmqReceiver, ZmqSender, RemoteActorRef, register_remote_message, define_message, handle_messages,
    Start,
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

/// PingActor - Sends Ping to pong (looked up via registry), receives Pong back
struct PingActor {
    manager_handle: ManagerHandle,
    registry_client: Arc<RegistryClient>,
    zmq_sender: Arc<ZmqSender>,
    pong_ref: Option<ActorRef>,
}

impl PingActor {
    fn new(
        manager_handle: ManagerHandle,
        registry_client: Arc<RegistryClient>,
        zmq_sender: Arc<ZmqSender>,
    ) -> Self {
        PingActor {
            manager_handle,
            registry_client,
            zmq_sender,
            pong_ref: None,
        }
    }
}

handle_messages!(PingActor,
    Start => on_start,
    Pong => on_pong
);

impl PingActor {
    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("PingActor: Starting ping-pong...");

        // Look up pong actor via registry
        match self.registry_client.lookup("pong") {
            Ok(endpoint) => {
                println!("PingActor: Found 'pong' at {}", endpoint);

                // Create remote ref for pong
                let pong_ref = RemoteActorRef::new("pong", &endpoint, Arc::clone(&self.zmq_sender));
                self.pong_ref = Some(pong_ref.into_actor_ref());

                // Send first ping
                println!("PingActor: Sending first ping");
                if let Some(ref pong) = self.pong_ref {
                    pong.send(Box::new(Ping { count: 1 }), ctx.self_ref());
                }
            }
            Err(RegistryError::NotFound(_)) => {
                eprintln!("PingActor: Failed to find 'pong'");
                eprintln!("Make sure registry_pong is running first!");
                self.manager_handle.terminate();
            }
            Err(RegistryError::Offline(_)) => {
                eprintln!("PingActor: 'pong' is offline");
                self.manager_handle.terminate();
            }
            Err(e) => {
                eprintln!("PingActor: Registry error: {}", e);
                self.manager_handle.terminate();
            }
        }
    }

    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("PingActor: Received pong {} from remote", msg.count);

        if msg.count >= 5 {
            println!("PingActor: Done!");
            self.manager_handle.terminate();
        } else {
            // Send next ping
            if let Some(ref pong) = self.pong_ref {
                pong.send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
            }
        }
    }
}

fn main() {
    let registry_endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tcp://localhost:5555".to_string());

    let endpoint = "tcp://0.0.0.0:5002";
    let local_endpoint = "tcp://localhost:5002";

    println!("=== Registry Ping Process (port 5002) ===");
    println!("Registry: {}", registry_endpoint);

    // Register messages for remote serialization
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");

    // Create registry client
    let registry_client = Arc::new(RegistryClient::new("PingManager", &registry_endpoint));
    registry_client.start_heartbeat();

    // Create ZMQ sender for remote communication
    let zmq_sender = Arc::new(ZmqSender::new(local_endpoint));

    // Create manager
    let mut mgr = Manager::new();
    let handle = mgr.get_handle();

    // Create ping actor - it will lookup pong via registry in on_start
    let ping_actor = PingActor::new(
        handle.clone(),
        Arc::clone(&registry_client),
        Arc::clone(&zmq_sender),
    );
    let ping_ref = mgr.manage("ping", Box::new(ping_actor), ThreadConfig::default());

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("ping", ping_ref);

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    // Set up signal handler
    let handle_clone = handle.clone();
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        handle_clone.terminate();
    }).expect("Error setting Ctrl-C handler");

    mgr.init();
    println!("Ping process starting...");

    // Wait until terminated
    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Registry Ping Process Complete ===");
}
