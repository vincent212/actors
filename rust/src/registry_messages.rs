/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech

*/

//! Registry protocol messages.
//!
//! These messages are used for communication between Managers and GlobalRegistry.

use crate::define_message;

/// RegisterActor - Manager registers an actor with GlobalRegistry
///
/// Sent during Manager::manage() to register actor name -> endpoint mapping.
/// GlobalRegistry replies with RegistrationOk or RegistrationFailed.
pub struct RegisterActor {
    pub manager_id: String,
    pub actor_name: String,
    pub actor_endpoint: String,  // ZMQ endpoint for reaching this actor
}
define_message!(RegisterActor);

/// UnregisterActor - Remove an actor from the registry
///
/// Sent when an actor is stopped or Manager shuts down.
pub struct UnregisterActor {
    pub actor_name: String,
}
define_message!(UnregisterActor);

/// RegistrationOk - Confirms successful actor registration
pub struct RegistrationOk {
    pub actor_name: String,
}
define_message!(RegistrationOk);

/// RegistrationFailed - Registration was rejected
///
/// Common reasons: name already registered, invalid ActorRef
pub struct RegistrationFailed {
    pub actor_name: String,
    pub reason: String,
}
define_message!(RegistrationFailed);

/// LookupActor - Request ActorRef for a named actor
///
/// Manager sends this when local lookup fails.
/// GlobalRegistry replies with LookupResult via standard reply() mechanism.
pub struct LookupActor {
    pub actor_name: String,
}
define_message!(LookupActor);

/// LookupResult - Response to LookupActor
///
/// Contains the endpoint if found, and online status.
/// If endpoint is None, the actor was not found.
/// If online is false, the actor's Manager has missed heartbeats.
pub struct LookupResult {
    pub actor_name: String,
    pub endpoint: Option<String>,  // ZMQ endpoint for reaching this actor
    pub online: bool,
}
define_message!(LookupResult);

/// Heartbeat - Manager health check
///
/// Managers send this every 2 seconds.
/// GlobalRegistry marks Manager offline after 6 seconds without heartbeat.
pub struct Heartbeat {
    pub manager_id: String,
    pub timestamp_ms: u64,
}
define_message!(Heartbeat);

impl Heartbeat {
    pub fn new(manager_id: String) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Heartbeat { manager_id, timestamp_ms }
    }
}

/// HeartbeatAck - Acknowledgement of heartbeat
pub struct HeartbeatAck;
define_message!(HeartbeatAck);
