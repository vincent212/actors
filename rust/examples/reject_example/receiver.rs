/*
Reject Example: Receiver Process

This example demonstrates error handling when a remote message type is unknown.
The receiver only knows about Ping/Pong, not UnknownMessage.

Run this first, then run sender:
    cargo run --example reject_receiver

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use actors::{
    ActorContext, Manager, Reject, ThreadConfig,
    ZmqReceiver, ZmqSender, register_remote_message, define_message, handle_messages,
};

// Receiver only knows about Ping/Pong - NOT UnknownMessage
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
    // Only register messages we know about
    // Note: UnknownMessage is NOT registered here!
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");
    register_remote_message::<Reject>("Reject");
}

/// ReceiverActor receives Ping, sends Pong back.
/// Unknown messages will trigger automatic Reject responses.
struct ReceiverActor;

handle_messages!(ReceiverActor,
    Ping => on_ping
);

impl ReceiverActor {
    fn new() -> Self {
        ReceiverActor
    }

    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        println!("ReceiverActor: Received Ping {}", msg.count);
        ctx.reply(Box::new(Pong { count: msg.count }));
        println!("ReceiverActor: Sent Pong {} back", msg.count);
    }
}

fn main() {
    println!("=== Reject Example: Receiver (port 5001) ===");
    println!("This receiver only knows Ping/Pong - UnknownMessage will be rejected");

    // Register messages for remote serialization
    register_messages();

    let local_endpoint = "tcp://0.0.0.0:5001";

    // Create ZMQ sender (for replies)
    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5001"));

    // Create manager and actors
    let mut mgr = Manager::new();

    let receiver_ref = mgr.manage(
        "receiver",
        Box::new(ReceiverActor::new()),
        ThreadConfig::default()
    );

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(local_endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("receiver", receiver_ref);

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    // Small delay to ensure receiver is ready
    thread::sleep(Duration::from_millis(100));

    mgr.init();
    println!("Receiver process ready, waiting for messages...");
    println!("Press Ctrl+C to stop");

    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Receiver Process Complete ===");
}
