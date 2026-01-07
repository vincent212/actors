//! Rust PongActor for cross-language ping-pong
//!
//! Receives Ping from C++ PingActor, sends Pong back via FFI.
//! Uses the standard Actor trait with handle_messages! macro.

use actors::{handle_messages, ActorContext, ManagerHandle};
use crate::interop_messages::{Ping, Pong};
use crate::cpp_actor_if::CppActorIF;

/// Rust Pong Actor - receives Ping, sends Pong back
pub struct RustPongActor {
    /// Interface to send back to C++ actors
    cpp_ping: CppActorIF,
    #[allow(dead_code)]
    manager_handle: ManagerHandle,
}

impl RustPongActor {
    pub fn new(manager_handle: ManagerHandle) -> Self {
        RustPongActor {
            cpp_ping: CppActorIF::new("cpp_ping", Some("rust_pong")),
            manager_handle,
        }
    }

    fn on_ping(&mut self, msg: &Ping, _ctx: &mut ActorContext) {
        println!("[Rust Pong] Received Ping #{}", msg.count);

        // Send Pong back to C++
        let pong = Pong { count: msg.count };
        println!("[Rust Pong] Sending Pong #{} back to C++...", pong.count);
        self.cpp_ping.send(&pong);
    }
}

// Register message handlers
handle_messages!(RustPongActor,
    Ping => on_ping
);
