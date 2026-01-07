/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech

*/

//! Registry client for communicating with GlobalRegistry via ZMQ.

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::json;
use tokio::runtime::Runtime;
use zeromq::{ReqSocket, Socket, SocketRecv, SocketSend};

/// Error types for registry operations.
#[derive(Debug, Clone)]
pub enum RegistryError {
    /// Actor name not found in registry.
    NotFound(String),
    /// Actor's manager is offline (missed heartbeats).
    Offline(String),
    /// Registration failed (e.g., name already taken).
    RegistrationFailed { actor_name: String, reason: String },
    /// Communication error with registry.
    ConnectionError(String),
    /// Operation timed out.
    Timeout(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::NotFound(name) => write!(f, "Actor not found: {}", name),
            RegistryError::Offline(name) => write!(f, "Actor offline: {}", name),
            RegistryError::RegistrationFailed { actor_name, reason } => {
                write!(f, "Registration failed for '{}': {}", actor_name, reason)
            }
            RegistryError::ConnectionError(msg) => write!(f, "Registry connection error: {}", msg),
            RegistryError::Timeout(msg) => write!(f, "Registry timeout: {}", msg),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Client for communicating with the GlobalRegistry via ZMQ.
///
/// The RegistryClient:
/// - Sends heartbeats every 2 seconds in a background thread
/// - Provides sync lookup for actors by name
/// - Handles registration of local actors
///
/// # Example
/// ```ignore
/// let client = RegistryClient::new("MyManager", "tcp://localhost:5555");
/// client.start_heartbeat();
///
/// // Register an actor
/// client.register("MyActor", "tcp://localhost:5001")?;
///
/// // Lookup a remote actor
/// let endpoint = client.lookup("OtherActor")?;
///
/// client.stop_heartbeat();
/// ```
pub struct RegistryClient {
    manager_id: String,
    registry_endpoint: String,
    socket: Arc<Mutex<Option<ReqSocket>>>,
    runtime: Arc<Runtime>,
    heartbeat_handle: Mutex<Option<JoinHandle<()>>>,
    running: Arc<Mutex<bool>>,
}

impl RegistryClient {
    /// Create a new registry client.
    ///
    /// # Arguments
    /// * `manager_id` - Unique identifier for this manager
    /// * `registry_endpoint` - ZMQ endpoint of the GlobalRegistry (e.g., "tcp://localhost:5555")
    pub fn new(manager_id: &str, registry_endpoint: &str) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Failed to create runtime"));

        RegistryClient {
            manager_id: manager_id.to_string(),
            registry_endpoint: registry_endpoint.to_string(),
            socket: Arc::new(Mutex::new(None)),
            runtime,
            heartbeat_handle: Mutex::new(None),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Connect the ZMQ socket (lazy initialization).
    fn ensure_connected(&self) -> Result<(), RegistryError> {
        let mut socket_guard = self.socket.lock().unwrap();
        if socket_guard.is_none() {
            let endpoint = self.registry_endpoint.clone();
            let socket = self.runtime.block_on(async {
                let mut socket = ReqSocket::new();
                socket.connect(&endpoint).await.map_err(|e| {
                    RegistryError::ConnectionError(format!("Failed to connect: {}", e))
                })?;
                // Small delay to let connection establish
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok::<ReqSocket, RegistryError>(socket)
            })?;
            *socket_guard = Some(socket);
        }
        Ok(())
    }

    /// Send a request and receive a reply.
    fn send_recv(&self, msg: serde_json::Value) -> Result<serde_json::Value, RegistryError> {
        self.ensure_connected()?;

        let mut socket_guard = self.socket.lock().unwrap();
        let socket = socket_guard.as_mut().unwrap();

        self.runtime.block_on(async {
            let data = msg.to_string().into_bytes();
            socket.send(data.into()).await.map_err(|e| {
                RegistryError::ConnectionError(format!("Send failed: {}", e))
            })?;

            let reply = socket.recv().await.map_err(|e| {
                RegistryError::ConnectionError(format!("Recv failed: {}", e))
            })?;

            let reply_data = reply.get(0).map(|b| b.as_ref()).unwrap_or(&[]);
            let reply_str = String::from_utf8(reply_data.to_vec()).map_err(|e| {
                RegistryError::ConnectionError(format!("Invalid UTF-8: {}", e))
            })?;

            serde_json::from_str(&reply_str).map_err(|e| {
                RegistryError::ConnectionError(format!("Invalid JSON: {}", e))
            })
        })
    }

    /// Start the heartbeat background thread.
    ///
    /// Sends Heartbeat messages every 2 seconds to keep actors marked as online.
    pub fn start_heartbeat(&self) {
        let mut running = self.running.lock().unwrap();
        if *running {
            return; // Already running
        }
        *running = true;
        drop(running);

        let manager_id = self.manager_id.clone();
        let registry_endpoint = self.registry_endpoint.clone();
        let running_flag = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create heartbeat runtime");

            rt.block_on(async {
                // Create dedicated socket for heartbeats
                let mut socket = ReqSocket::new();
                if socket.connect(&registry_endpoint).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;

                while *running_flag.lock().unwrap() {
                    // Send heartbeat
                    let timestamp_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    let msg = json!({
                        "message_type": "Heartbeat",
                        "manager_id": manager_id,
                        "timestamp_ms": timestamp_ms
                    });

                    let data = msg.to_string().into_bytes();
                    if socket.send(data.into()).await.is_ok() {
                        let _ = socket.recv().await; // Ignore reply
                    }

                    // Sleep for 2 seconds
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            });
        });

        *self.heartbeat_handle.lock().unwrap() = Some(handle);
    }

    /// Stop the heartbeat background thread.
    pub fn stop_heartbeat(&self) {
        *self.running.lock().unwrap() = false;

        if let Some(handle) = self.heartbeat_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    /// Register an actor with the GlobalRegistry.
    ///
    /// # Arguments
    /// * `actor_name` - Unique name for the actor
    /// * `endpoint` - ZMQ endpoint where the actor can be reached
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeded
    /// * `Err(RegistryError)` if registration failed
    pub fn register(&self, actor_name: &str, endpoint: &str) -> Result<(), RegistryError> {
        let msg = json!({
            "message_type": "RegisterActor",
            "manager_id": self.manager_id,
            "actor_name": actor_name,
            "actor_endpoint": endpoint
        });

        let reply = self.send_recv(msg)?;

        match reply.get("message_type").and_then(|v| v.as_str()) {
            Some("RegistrationOk") => Ok(()),
            Some("RegistrationFailed") => {
                let reason = reply.get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                Err(RegistryError::RegistrationFailed {
                    actor_name: actor_name.to_string(),
                    reason: reason.to_string(),
                })
            }
            _ => Err(RegistryError::ConnectionError("Unexpected response".to_string())),
        }
    }

    /// Lookup an actor by name.
    ///
    /// # Arguments
    /// * `actor_name` - Name of the actor to find
    ///
    /// # Returns
    /// * `Ok(endpoint)` if actor found and online
    /// * `Err(RegistryError::NotFound)` if actor not registered
    /// * `Err(RegistryError::Offline)` if actor's manager missed heartbeats
    pub fn lookup(&self, actor_name: &str) -> Result<String, RegistryError> {
        let msg = json!({
            "message_type": "LookupActor",
            "actor_name": actor_name
        });

        let reply = self.send_recv(msg)?;

        match reply.get("message_type").and_then(|v| v.as_str()) {
            Some("LookupResult") => {
                let endpoint = reply.get("endpoint").and_then(|v| v.as_str());
                let online = reply.get("online").and_then(|v| v.as_bool()).unwrap_or(false);

                match endpoint {
                    Some(ep) => {
                        if online {
                            Ok(ep.to_string())
                        } else {
                            Err(RegistryError::Offline(actor_name.to_string()))
                        }
                    }
                    None => Err(RegistryError::NotFound(actor_name.to_string())),
                }
            }
            _ => Err(RegistryError::ConnectionError("Unexpected response".to_string())),
        }
    }

    /// Lookup an actor, returning the endpoint even if offline.
    ///
    /// Use this when you want to attempt communication with a potentially
    /// recovering actor.
    pub fn lookup_allow_offline(&self, actor_name: &str) -> Result<(String, bool), RegistryError> {
        let msg = json!({
            "message_type": "LookupActor",
            "actor_name": actor_name
        });

        let reply = self.send_recv(msg)?;

        match reply.get("message_type").and_then(|v| v.as_str()) {
            Some("LookupResult") => {
                let endpoint = reply.get("endpoint").and_then(|v| v.as_str());
                let online = reply.get("online").and_then(|v| v.as_bool()).unwrap_or(false);

                match endpoint {
                    Some(ep) => Ok((ep.to_string(), online)),
                    None => Err(RegistryError::NotFound(actor_name.to_string())),
                }
            }
            _ => Err(RegistryError::ConnectionError("Unexpected response".to_string())),
        }
    }
}

impl Drop for RegistryClient {
    fn drop(&mut self) {
        self.stop_heartbeat();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_error_display_not_found() {
        let err = RegistryError::NotFound("pong".to_string());
        assert_eq!(format!("{}", err), "Actor not found: pong");
    }

    #[test]
    fn test_registry_error_display_offline() {
        let err = RegistryError::Offline("pong".to_string());
        assert_eq!(format!("{}", err), "Actor offline: pong");
    }

    #[test]
    fn test_registry_error_display_registration_failed() {
        let err = RegistryError::RegistrationFailed {
            actor_name: "pong".to_string(),
            reason: "Name already registered".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Registration failed for 'pong': Name already registered"
        );
    }

    #[test]
    fn test_registry_error_display_connection_error() {
        let err = RegistryError::ConnectionError("Connection refused".to_string());
        assert_eq!(format!("{}", err), "Registry connection error: Connection refused");
    }

    #[test]
    fn test_registry_error_display_timeout() {
        let err = RegistryError::Timeout("Operation timed out".to_string());
        assert_eq!(format!("{}", err), "Registry timeout: Operation timed out");
    }

    #[test]
    fn test_register_message_format() {
        let msg = json!({
            "message_type": "RegisterActor",
            "manager_id": "mgr1",
            "actor_name": "pong",
            "actor_endpoint": "tcp://localhost:5001"
        });

        assert_eq!(msg["message_type"], "RegisterActor");
        assert_eq!(msg["manager_id"], "mgr1");
        assert_eq!(msg["actor_name"], "pong");
        assert_eq!(msg["actor_endpoint"], "tcp://localhost:5001");
    }

    #[test]
    fn test_lookup_message_format() {
        let msg = json!({
            "message_type": "LookupActor",
            "actor_name": "pong"
        });

        assert_eq!(msg["message_type"], "LookupActor");
        assert_eq!(msg["actor_name"], "pong");
    }

    #[test]
    fn test_heartbeat_message_format() {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let msg = json!({
            "message_type": "Heartbeat",
            "manager_id": "mgr1",
            "timestamp_ms": timestamp_ms
        });

        assert_eq!(msg["message_type"], "Heartbeat");
        assert_eq!(msg["manager_id"], "mgr1");
        assert!(msg["timestamp_ms"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_registry_client_new() {
        let client = RegistryClient::new("test_manager", "tcp://localhost:5555");
        assert_eq!(client.manager_id, "test_manager");
        assert_eq!(client.registry_endpoint, "tcp://localhost:5555");
    }
}
