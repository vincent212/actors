/*
Remote Rust-to-Python: Rust Ping Process

Run python_pong.py first, then run this.

Usage:
    cargo run --example rust_ping

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use actors::{
    ActorContext, ActorRef, Manager, ManagerHandle, Start, ThreadConfig,
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

/// PingActor sends Ping to Python pong, receives Pong back.
struct PingActor {
    pong_ref: ActorRef,
    manager_handle: ManagerHandle,
}

handle_messages!(PingActor,
    Start => on_start,
    Pong => on_pong
);

impl PingActor {
    fn new(pong_ref: ActorRef, manager_handle: ManagerHandle) -> Self {
        PingActor { pong_ref, manager_handle }
    }

    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("RustPingActor: Starting ping-pong with Python pong");
        self.pong_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("RustPingActor: Received pong {} from Python", msg.count);
        if msg.count >= 5 {
            println!("RustPingActor: Done!");
            self.manager_handle.terminate();
        } else {
            self.pong_ref.send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
        }
    }
}

fn main() {
    println!("=== Rust Ping Process (port 5002) -> Python Pong (port 5001) ===");

    // Register messages for remote serialization
    register_messages();

    let local_endpoint = "tcp://0.0.0.0:5002";
    let remote_pong_endpoint = "tcp://localhost:5001";

    // Create ZMQ sender
    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5002"));

    // Create remote ref to Python pong process
    let remote_pong = zmq_sender.remote_ref("pong", remote_pong_endpoint);

    // Create manager and actors
    let mut mgr = Manager::new();
    let handle = mgr.get_handle();

    let ping_ref = mgr.manage(
        "ping",
        Box::new(PingActor::new(remote_pong.into_actor_ref(), handle)),
        ThreadConfig::default()
    );

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(local_endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("ping", ping_ref);

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    // Small delay to ensure receiver is ready
    thread::sleep(Duration::from_millis(100));

    mgr.init();
    println!("Rust ping process starting...");

    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Rust Ping Process Complete ===");
}
