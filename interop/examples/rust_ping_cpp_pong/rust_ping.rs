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
use crate::rust_manager_ffi::get_actor_ref;

pub struct RustPingActor {
    /// ActorRef to C++ pong actor - location transparent!
    cpp_pong: Option<ActorRef>,
    manager_handle: ManagerHandle,
}

impl RustPingActor {
    pub fn new(manager_handle: ManagerHandle) -> Self {
        RustPingActor {
            // Will be looked up via get_actor_ref() on first use
            cpp_pong: None,
            manager_handle,
        }
    }

    /// Get the pong ActorRef, looking it up if needed
    fn get_pong(&mut self) -> Option<ActorRef> {
        if self.cpp_pong.is_none() {
            self.cpp_pong = get_actor_ref("cpp_pong", "rust_ping");
        }
        self.cpp_pong.clone()
    }

    fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
        println!("[Rust Ping] Starting ping-pong...");
        println!("[Rust Ping] Sending Ping #1");
        let ping = Ping { count: 1 };
        if let Some(pong) = self.get_pong() {
            pong.send(Box::new(ping), None);
        }
    }

    fn on_pong(&mut self, msg: &Pong, _ctx: &mut ActorContext) {
        println!("[Rust Ping] Received Pong #{}", msg.count);

        if msg.count < 3 {
            let next_count = msg.count + 1;
            println!("[Rust Ping] Sending Ping #{}", next_count);
            let ping = Ping { count: next_count };
            if let Some(pong) = self.get_pong() {
                pong.send(Box::new(ping), None);
            }
        } else {
            println!("[Rust Ping] Ping-pong complete!");
            self.manager_handle.terminate();
        }
    }
}

// Register message handlers
handle_messages!(RustPingActor,
    Start => on_start,
    Pong => on_pong
);
