/*
Remote Ping-Pong: Pong Process

Run this first, then run ping in another terminal.

Usage:
    cargo run --example remote_pong

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
    println!("=== Pong Process (port 5001) ===");

    // Register messages for remote serialization (use class names for interop)
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");

    let endpoint = "tcp://0.0.0.0:5001";

    // Create ZMQ sender for replies
    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5001"));

    // Create manager and actors
    let mut mgr = Manager::new();
    let pong_ref = mgr.manage("pong", Box::new(PongActor), ThreadConfig::default());

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("pong", pong_ref);

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    mgr.init();
    println!("Pong process ready, waiting for pings...");
    println!("Press Ctrl+C to stop");

    // Wait for interrupt
    loop {
        thread::sleep(Duration::from_secs(1));
    }

    // Cleanup (unreachable in this example, but shows proper cleanup)
    // receiver_handle.stop();
    // mgr.end();
    // println!("=== Pong Process Complete ===");
}
