/*
Remote Ping-Pong: Ping Process

Run pong first, then run this.

Usage:
    cargo run --example remote_ping

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

/// PingActor sends Ping to remote pong, receives Pong back.
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
        println!("PingActor: Starting ping-pong with remote pong");
        self.pong_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("PingActor: Received pong {} from remote", msg.count);
        if msg.count >= 5 {
            println!("PingActor: Done!");
            self.manager_handle.terminate();
        } else {
            self.pong_ref.send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
        }
    }
}

fn main() {
    println!("=== Ping Process (port 5002) ===");

    // Register messages for remote serialization (use class names for interop)
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");

    let local_endpoint = "tcp://0.0.0.0:5002";
    let remote_pong_endpoint = "tcp://localhost:5001";

    // Create ZMQ sender
    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5002"));

    // Create remote ref to pong on other process
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
    println!("Ping process starting...");

    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Ping Process Complete ===");
}
