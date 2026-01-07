/*
Remote Rust-to-Python: Rust Pong Process

Run this first, then run python ping_process.py.

Usage:
    cargo run --example rust_pong

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use actors::{
    ActorContext, Manager, ThreadConfig,
    ZmqReceiver, ZmqSender, register_remote_message, define_message, handle_messages,
};

// Define messages - must match Python message registration
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

fn register_messages() {
    // Register messages with their type names (must match Python)
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");
}

/// PongActor receives Ping from Python ping, sends Pong back.
struct PongActor;

handle_messages!(PongActor,
    Ping => on_ping
);

impl PongActor {
    fn new() -> Self {
        PongActor
    }

    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        println!("RustPongActor: Received ping {} from Python", msg.count);
        ctx.reply(Box::new(Pong { count: msg.count }));
    }
}

fn main() {
    println!("=== Rust Pong Process (port 5001) <- Python Ping (port 5002) ===");

    // Register messages for remote serialization
    register_messages();

    let local_endpoint = "tcp://0.0.0.0:5001";

    // Create ZMQ sender (for replies)
    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5001"));

    // Create manager and actors
    let mut mgr = Manager::new();

    let pong_ref = mgr.manage(
        "pong",
        Box::new(PongActor::new()),
        ThreadConfig::default()
    );

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(local_endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("pong", pong_ref);

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    // Small delay to ensure receiver is ready
    thread::sleep(Duration::from_millis(100));

    mgr.init();
    println!("Rust pong process ready, waiting for pings...");
    println!("Press Ctrl+C to stop");

    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Rust Pong Process Complete ===");
}
