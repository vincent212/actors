/*
Reject Example: Sender Process

This example demonstrates error handling when a remote message type is unknown.
The sender sends a message that the receiver doesn't know about, and receives
a Reject message back.

Run receiver first, then run this:
    cargo run --example reject_sender

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use actors::{
    ActorContext, ActorRef, Manager, ManagerHandle, Reject, Start, ThreadConfig,
    ZmqReceiver, ZmqSender, register_remote_message, define_message, handle_messages,
};

// Define a message that the receiver DOES NOT know about
#[derive(Serialize, Deserialize, Default)]
struct UnknownMessage {
    data: i32,
}
define_message!(UnknownMessage);

// Define Ping/Pong for normal communication
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
    // Register all messages we might send or receive
    register_remote_message::<UnknownMessage>("UnknownMessage");
    register_remote_message::<Ping>("Ping");
    register_remote_message::<Pong>("Pong");
    register_remote_message::<Reject>("Reject");
}

/// SenderActor sends messages and handles Reject responses.
struct SenderActor {
    receiver_ref: ActorRef,
    manager_handle: ManagerHandle,
    rejected_count: i32,
}

handle_messages!(SenderActor,
    Start => on_start,
    Pong => on_pong,
    Reject => on_reject
);

impl SenderActor {
    fn new(receiver_ref: ActorRef, manager_handle: ManagerHandle) -> Self {
        SenderActor {
            receiver_ref,
            manager_handle,
            rejected_count: 0,
        }
    }

    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("SenderActor: Starting test...");

        // First, send an UnknownMessage (receiver doesn't know this type)
        println!("SenderActor: Sending UnknownMessage (should be rejected)...");
        self.receiver_ref.send(Box::new(UnknownMessage { data: 42 }), ctx.self_ref());

        // Also send a valid Ping to show normal operation continues
        println!("SenderActor: Sending Ping (should succeed)...");
        self.receiver_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    fn on_pong(&mut self, msg: &Pong, _ctx: &mut ActorContext) {
        println!("SenderActor: Received Pong {} - normal message worked!", msg.count);

        // Wait a bit then terminate
        println!("SenderActor: Test complete!");
        self.manager_handle.terminate();
    }

    fn on_reject(&mut self, msg: &Reject, _ctx: &mut ActorContext) {
        self.rejected_count += 1;
        println!("SenderActor: Received Reject!");
        println!("  - Message type: {}", msg.message_type);
        println!("  - Reason: {}", msg.reason);
        println!("  - Rejected by: {}", msg.rejected_by);
    }
}

fn main() {
    println!("=== Reject Example: Sender (port 5002) -> Receiver (port 5001) ===");

    // Register messages for remote serialization
    register_messages();

    let local_endpoint = "tcp://0.0.0.0:5002";
    let remote_receiver_endpoint = "tcp://localhost:5001";

    // Create ZMQ sender
    let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5002"));

    // Create remote ref to receiver process
    let remote_receiver = zmq_sender.remote_ref("receiver", remote_receiver_endpoint);

    // Create manager and actors
    let mut mgr = Manager::new();
    let handle = mgr.get_handle();

    let sender_ref = mgr.manage(
        "sender",
        Box::new(SenderActor::new(remote_receiver.into_actor_ref(), handle)),
        ThreadConfig::default()
    );

    // Create ZMQ receiver and register local actors
    let zmq_receiver = ZmqReceiver::new(local_endpoint, Arc::clone(&zmq_sender));
    zmq_receiver.register("sender", sender_ref);

    // Start the receiver
    let mut receiver_handle = zmq_receiver.start();

    // Small delay to ensure receiver is ready
    thread::sleep(Duration::from_millis(100));

    mgr.init();
    println!("Sender process starting...");

    mgr.run();

    // Cleanup
    receiver_handle.stop();
    mgr.end();

    println!("=== Sender Process Complete ===");
}
