//! Rust Ping Actor - initiates ping-pong with C++
//!
//! Demonstrates Rust calling C++ actor and receiving reply.
//! Uses the standard Actor trait with handle_messages! macro.
//!
//! Note: Uses ActorRef for location transparency - actor doesn't know
//! if cpp_pong is local Rust or remote C++.

use actors::{handle_messages, ActorContext, ActorRef, ManagerHandle};
use actors::messages::Start;
use crate::interop_messages::{Ping, Pong};

pub struct PingActor {
    pong_ref: ActorRef,
    manager_handle: ManagerHandle,
}

impl PingActor {
    pub fn new(pong_ref: ActorRef, manager_handle: ManagerHandle) -> Self {
        PingActor { pong_ref, manager_handle }
    }

    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("[Rust Ping] Starting ping-pong...");
        println!("[Rust Ping] Sending Ping #1");
        self.pong_ref.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("[Rust Ping] Received Pong #{}", msg.count);

        if msg.count >= 5 {
            println!("[Rust Ping] Ping-pong complete!");
            self.manager_handle.terminate();
        } else {
            println!("[Rust Ping] Sending Ping #{}", msg.count + 1);
            self.pong_ref.send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
        }
    }
}

// Register message handlers
handle_messages!(PingActor,
    Start => on_start,
    Pong => on_pong
);
