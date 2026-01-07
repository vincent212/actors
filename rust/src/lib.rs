/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! # Actors - High-Performance Actor Framework for Rust
//!
//! A lightweight, high-performance actor framework for building concurrent systems.
//!
//! ## Features
//!
//! - **Actor Model**: Independent entities processing messages sequentially
//! - **Message-Driven**: All communication via typed message passing
//! - **Thread-Safe**: Each actor runs in its own thread with isolated state
//! - **Low-Latency**: Designed for high-frequency trading systems
//! - **Simple API**: Easy to learn, minimal boilerplate
//!
//! ## Quick Start
//!
//! ### 1. Define Messages
//!
//! ```rust
//! use actors::{Message, define_message};
//!
//! struct Ping { count: i32 }
//! define_message!(Ping);
//!
//! struct Pong { count: i32 }
//! define_message!(Pong);
//! ```
//!
//! ### 2. Create Actors
//!
//! ```rust
//! use actors::{Actor, ActorContext, Message};
//!
//! struct PongActor;
//!
//! impl Actor for PongActor {
//!     fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
//!         if let Some(ping) = msg.as_any().downcast_ref::<Ping>() {
//!             println!("Received ping {}", ping.count);
//!             ctx.reply(Box::new(Pong { count: ping.count }));
//!         }
//!     }
//! }
//! ```
//!
//! ### 3. Set Up Manager
//!
//! ```rust
//! use actors::{Manager, ThreadConfig};
//!
//! let mut mgr = Manager::new();
//! let pong_ref = mgr.manage("pong", Box::new(PongActor), ThreadConfig::default());
//! let ping_ref = mgr.manage("ping", Box::new(PingActor::new(pong_ref)), ThreadConfig::default());
//!
//! mgr.init();  // Start all actors
//! // ... run ...
//! mgr.end();   // Wait for all actors to finish
//! ```
//!
//! ## Messaging
//!
//! ### Async Send (Fire-and-Forget)
//! ```rust
//! other_actor.send(Box::new(MyMessage { data: 42 }), ctx.self_ref());
//! ```
//!
//! ### Sync Send (RPC-style)
//! ```rust
//! if let Some(reply) = other_actor.fast_send(Box::new(Request { id: 1 }), ctx.self_ref()) {
//!     if let Some(response) = reply.as_any().downcast_ref::<Response>() {
//!         // Handle response
//!     }
//! }
//! ```
//!
//! ### Reply
//! ```rust
//! fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
//!     ctx.reply(Box::new(Response { result: 42 }));
//! }
//! ```

pub mod actor;
pub mod manager;
pub mod message;
pub mod messages;
pub mod registry;
pub mod registry_messages;
pub mod remote;
pub mod serialization;
pub mod timer;

// Re-export commonly used types
pub use actor::{Actor, ActorContext, ActorRef, ActorRuntime, CppActorRef, CppSendFn, Envelope, LocalActorRef};
pub use manager::{register_cpp_lookup, CppActorLookupFn, Manager, ManagerHandle, ThreadConfig};
pub use message::Message;
pub use messages::{Continue, Reject, Shutdown, Start, Timeout};
pub use remote::{ActorRegistry, RemoteActorRef, ZmqReceiver, ZmqReceiverHandle, ZmqSender};
pub use serialization::{
    deserialize_message, get_type_name, register_remote_message,
    serialize_message,
};
pub use timer::{next_timer_id, Timer};
pub use registry::{RegistryClient, RegistryError};
