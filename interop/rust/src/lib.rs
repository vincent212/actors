//! actors-interop - FFI interop layer between actors-cpp and actors-rust
//!
//! This crate provides:
//! - `interop_messages` - Message definitions matching the C header
//! - `rust_actor_bridge` - extern "C" functions for C++ to call Rust actors
//! - `cpp_actor_if` - CppActorIF for Rust to call C++ actors
//! - `rust_manager_ffi` - FFI functions for C++ to manage Rust Manager
//!
//! Uses Manager's actor registry instead of separate registries.

// Include generated code
#[path = "../../generated/rust/interop_messages.rs"]
pub mod interop_messages;

#[path = "../../generated/rust/rust_actor_bridge.rs"]
pub mod rust_actor_bridge;

#[path = "../../generated/rust/cpp_actor_if.rs"]
pub mod cpp_actor_if;

// FFI for Rust Manager management
pub mod rust_manager_ffi;

// Re-export commonly used items
pub use interop_messages::*;
pub use cpp_actor_if::{CppActorIF, InteropMessage};

// Example actors - included in the library so they can be called from C++
#[path = "../../examples/ping_pong/rust_pong.rs"]
pub mod ping_pong;

#[path = "../../examples/pubsub/rust_publisher.rs"]
pub mod pubsub;

#[path = "../../examples/rust_ping_cpp_pong/rust_ping.rs"]
pub mod rust_ping;

#[path = "../../examples/rust_subscribes_cpp_publisher/rust_subscriber.rs"]
pub mod rust_subscriber;
