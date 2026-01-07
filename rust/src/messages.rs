/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Built-in message types for actor lifecycle and utilities.
//!
//! These messages are used internally by the framework for:
//! - Actor lifecycle (Start, Shutdown)
//! - Timer callbacks (Timeout)
//! - Self-continuation (Continue)

use crate::define_message;

/// Sent to actors when the Manager starts them.
///
/// Actors can handle this message to perform initialization that requires
/// messaging other actors (which isn't possible in `init()`).
#[derive(Debug, Clone)]
pub struct Start;
define_message!(Start);

/// Sent to actors when the Manager shuts down.
///
/// Actors should clean up resources and stop processing when they receive this.
#[derive(Debug, Clone)]
pub struct Shutdown;
define_message!(Shutdown);

/// Sent by the Timer when a timeout expires.
///
/// The `id` field allows actors to distinguish between multiple timers.
#[derive(Debug, Clone)]
pub struct Timeout {
    /// User-defined timer identifier
    pub id: u64,
}
define_message!(Timeout);

impl Timeout {
    pub fn new(id: u64) -> Self {
        Timeout { id }
    }
}

/// Sent by an actor to itself for self-continuation.
///
/// Useful for breaking up long-running work into smaller chunks
/// to allow other messages to be processed.
#[derive(Debug, Clone)]
pub struct Continue {
    /// User-defined continuation state
    pub state: u64,
}
define_message!(Continue);

impl Continue {
    pub fn new(state: u64) -> Self {
        Continue { state }
    }
}

/// Sent when a remote message cannot be processed.
///
/// This is sent back to the sender when:
/// - The message type is not registered
/// - The target actor is not found
/// - Deserialization fails
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Reject {
    /// The name of the message type that was rejected
    pub message_type: String,
    /// The reason for rejection
    pub reason: String,
    /// The name of the actor that rejected the message
    pub rejected_by: String,
}
define_message!(Reject);

impl Reject {
    pub fn new(message_type: &str, reason: &str, rejected_by: &str) -> Self {
        Reject {
            message_type: message_type.to_string(),
            reason: reason.to_string(),
            rejected_by: rejected_by.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_new() {
        let t = Timeout::new(42);
        assert_eq!(t.id, 42);
    }

    #[test]
    fn test_continue_new() {
        let c = Continue::new(100);
        assert_eq!(c.state, 100);
    }
}
