/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Remote actor communication via ZeroMQ (pure Rust implementation).
//!
//! Provides types for communicating with actors in other processes:
//! - `RemoteActorRef` - Reference to an actor in another process
//! - `ZmqSender` - Sends messages to remote processes
//! - `ZmqReceiver` - Receives messages from remote processes
//!
//! Uses the `zeromq` crate (pure Rust) for wire-compatible ZMQ messaging.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use tokio::runtime::Runtime;
use zeromq::{PullSocket, PushSocket, Socket, SocketRecv, SocketSend};

use crate::actor::ActorRef;
use crate::messages::Reject;
use crate::serialization::{get_type_name, serialize_message, try_deserialize_message};
use crate::Message;

/// Internal request for async remote sends.
/// Sent to the dedicated sender thread.
struct SendRequest {
    endpoint: String,
    data: Vec<u8>,
}

/// Reference to an actor in a remote process.
///
/// Uses ZMQ to send messages to actors in other processes.
#[derive(Clone)]
pub struct RemoteActorRef {
    name: String,
    endpoint: String,
    zmq_sender: Arc<ZmqSender>,
}

impl RemoteActorRef {
    /// Create a new remote actor reference.
    ///
    /// # Arguments
    /// * `name` - Name of the remote actor
    /// * `endpoint` - ZMQ endpoint (e.g., "tcp://localhost:5001")
    /// * `zmq_sender` - Shared ZmqSender for sending messages
    pub fn new(name: &str, endpoint: &str, zmq_sender: Arc<ZmqSender>) -> Self {
        RemoteActorRef {
            name: name.to_string(),
            endpoint: endpoint.to_string(),
            zmq_sender,
        }
    }

    /// Get the actor's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the endpoint
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Send a message to this remote actor
    pub fn send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) {
        self.zmq_sender.send_to(&self.endpoint, &self.name, msg, sender);
    }

    /// Convert to an ActorRef enum
    pub fn into_actor_ref(self) -> ActorRef {
        ActorRef::Remote(self)
    }
}

/// Sends messages to remote processes via ZMQ PUSH sockets.
///
/// Manages a pool of PUSH sockets, one per remote endpoint.
/// Runs on its own thread so sending never blocks the caller.
///
/// Usage:
///   let zmq_sender = Arc::new(ZmqSender::new("tcp://localhost:5002"));
///   // Sends are now async - returns immediately
///   zmq_sender.send_to("tcp://localhost:5001", "pong", msg, sender);
pub struct ZmqSender {
    send_tx: Sender<SendRequest>,
    local_endpoint: String,
}

impl ZmqSender {
    /// Create a new ZmqSender.
    ///
    /// Spawns a dedicated sender thread that handles all ZMQ sends asynchronously.
    ///
    /// # Arguments
    /// * `local_endpoint` - This process's endpoint for reply routing
    pub fn new(local_endpoint: &str) -> Self {
        let (send_tx, send_rx) = channel::<SendRequest>();

        // Spawn dedicated sender thread
        thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create sender runtime");
            let mut sockets: HashMap<String, PushSocket> = HashMap::new();

            rt.block_on(async {
                while let Ok(req) = send_rx.recv() {
                    // Get or create socket for this endpoint
                    let needs_connect = !sockets.contains_key(&req.endpoint);

                    if needs_connect {
                        let mut socket = PushSocket::new();
                        let connect_endpoint = req.endpoint.replace("tcp://*:", "tcp://localhost:");
                        if socket.connect(&connect_endpoint).await.is_ok() {
                            // Small delay to let connection establish
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                            sockets.insert(req.endpoint.clone(), socket);
                        }
                    }

                    // Send the message
                    if let Some(socket) = sockets.get_mut(&req.endpoint) {
                        let _ = socket.send(req.data.into()).await;
                    }
                }
            });
        });

        ZmqSender {
            send_tx,
            local_endpoint: local_endpoint.to_string(),
        }
    }

    /// Get the local endpoint.
    pub fn local_endpoint(&self) -> &str {
        &self.local_endpoint
    }

    /// Create a remote actor reference
    pub fn remote_ref(self: &Arc<Self>, name: &str, endpoint: &str) -> RemoteActorRef {
        RemoteActorRef::new(name, endpoint, Arc::clone(self))
    }

    /// Send a message to a remote actor (async - returns immediately).
    ///
    /// The message is serialized on the caller's thread, then queued
    /// to the dedicated sender thread for actual ZMQ transmission.
    ///
    /// # Arguments
    /// * `endpoint` - Remote process's ZMQ endpoint
    /// * `actor_name` - Name of the target actor
    /// * `msg` - Message to send
    /// * `sender` - Optional sender for reply routing
    pub fn send_to(
        &self,
        endpoint: &str,
        actor_name: &str,
        msg: Box<dyn Message>,
        sender: Option<ActorRef>,
    ) {
        // Get message type name (must be registered)
        let msg_type = get_message_type_name(msg.as_ref());
        let msg_json = serialize_message(msg.as_ref(), &msg_type);

        // Determine sender info for reply routing
        let (sender_actor, sender_endpoint) = match &sender {
            Some(ActorRef::Local(r)) => (Some(r.name().to_string()), Some(self.local_endpoint.clone())),
            Some(ActorRef::Remote(r)) => (Some(r.name().to_string()), Some(r.endpoint().to_string())),
            Some(ActorRef::Cpp(r)) => (Some(r.name().to_string()), None),  // Cpp actors don't have endpoints
            None => (None, None),
        };

        let data = serde_json::json!({
            "sender_actor": sender_actor,
            "sender_endpoint": sender_endpoint,
            "receiver": actor_name,
            "message_type": msg_type,
            "message": msg_json
        });

        let data_bytes = data.to_string().into_bytes();

        // Queue to sender thread (non-blocking!)
        let _ = self.send_tx.send(SendRequest {
            endpoint: endpoint.to_string(),
            data: data_bytes,
        });
    }

    /// Send a message to a remote actor (async version for use within tokio runtime).
    ///
    /// This is used internally by ZmqReceiver when it needs to send Reject messages
    /// back to the sender. Now just queues to the sender thread like send_to().
    pub async fn send_to_async(
        &self,
        endpoint: &str,
        actor_name: &str,
        msg: Box<dyn Message>,
        sender: Option<ActorRef>,
    ) {
        // Just delegate to send_to - it's already non-blocking
        self.send_to(endpoint, actor_name, msg, sender);
    }
}

/// Get the message type name for serialization.
/// This requires the message to be registered with register_remote_message.
fn get_message_type_name(msg: &dyn Message) -> String {
    get_type_name(msg)
        .unwrap_or_else(|| {
            panic!("Message type not registered for remote serialization. Call register_remote_message::<YourType>(\"YourType\") first.")
        })
}

/// Registry of local actors for the ZmqReceiver.
/// Thread-safe container for looking up ActorRefs by name.
#[derive(Clone)]
pub struct ActorRegistry {
    actors: Arc<Mutex<HashMap<String, ActorRef>>>,
}

impl ActorRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        ActorRegistry {
            actors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a local actor.
    pub fn register(&self, name: &str, actor_ref: ActorRef) {
        self.actors.lock().unwrap().insert(name.to_string(), actor_ref);
    }

    /// Look up an actor by name.
    pub fn get(&self, name: &str) -> Option<ActorRef> {
        self.actors.lock().unwrap().get(name).cloned()
    }
}

impl Default for ActorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Receives messages from remote processes and routes to local actors.
///
/// Binds to a ZMQ PULL socket and forwards incoming messages to local actors.
pub struct ZmqReceiver {
    bind_endpoint: String,
    zmq_sender: Arc<ZmqSender>,
    registry: ActorRegistry,
    running: Arc<Mutex<bool>>,
}

impl ZmqReceiver {
    /// Create a new ZmqReceiver.
    ///
    /// # Arguments
    /// * `bind_endpoint` - Endpoint to bind to (e.g., "tcp://0.0.0.0:5001")
    /// * `zmq_sender` - ZmqSender for creating reply ActorRefs
    pub fn new(bind_endpoint: &str, zmq_sender: Arc<ZmqSender>) -> Self {
        ZmqReceiver {
            bind_endpoint: bind_endpoint.to_string(),
            zmq_sender,
            registry: ActorRegistry::new(),
            running: Arc::new(Mutex::new(true)),
        }
    }

    /// Register a local actor to receive messages.
    pub fn register(&self, name: &str, actor_ref: ActorRef) {
        self.registry.register(name, actor_ref);
    }

    /// Get the registry (for sharing with other components).
    pub fn registry(&self) -> &ActorRegistry {
        &self.registry
    }

    /// Start the receiver in a new thread.
    ///
    /// Returns a handle that can be used to stop the receiver.
    pub fn start(&self) -> ZmqReceiverHandle {
        let bind_endpoint = self.bind_endpoint.clone();
        let zmq_sender = Arc::clone(&self.zmq_sender);
        let registry = self.registry.clone();
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            // Create a new tokio runtime for this thread
            let rt = Runtime::new().expect("Failed to create receiver runtime");

            rt.block_on(async {
                let mut socket = PullSocket::new();

                // Convert tcp://*: to tcp://0.0.0.0: for pure-Rust zeromq
                let normalized_endpoint = bind_endpoint.replace("tcp://*:", "tcp://0.0.0.0:");
                socket.bind(&normalized_endpoint).await.expect("Failed to bind socket");

                loop {
                    // Check running flag
                    if !*running.lock().unwrap() {
                        break;
                    }

                    // Use tokio timeout to periodically check running flag
                    let recv_result = tokio::time::timeout(
                        tokio::time::Duration::from_millis(100),
                        socket.recv()
                    ).await;

                    match recv_result {
                        Ok(Ok(msg)) => {
                            let data = msg.get(0).map(|b| b.as_ref()).unwrap_or(&[]);
                            if let Ok(json_str) = String::from_utf8(data.to_vec()) {
                                if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    Self::handle_remote_message_async(&envelope, &zmq_sender, &registry).await;
                                }
                            }
                        }
                        Ok(Err(_)) => {
                            // Socket error, exit
                            break;
                        }
                        Err(_) => {
                            // Timeout, continue to check running flag
                            continue;
                        }
                    }
                }
            });
        });

        ZmqReceiverHandle {
            running: Arc::clone(&self.running),
            thread: Some(handle),
        }
    }

    async fn handle_remote_message_async(
        data: &serde_json::Value,
        zmq_sender: &Arc<ZmqSender>,
        registry: &ActorRegistry,
    ) {
        let receiver_name = data["receiver"].as_str().unwrap_or("");
        let msg_type = data["message_type"].as_str().unwrap_or("");
        let msg_data = &data["message"];

        // Get sender info for replies (needed for both success and reject)
        let sender_actor = data["sender_actor"].as_str();
        let sender_endpoint = data["sender_endpoint"].as_str();

        // Look up local actor
        let local_ref = match registry.get(receiver_name) {
            Some(r) => r,
            None => {
                // Actor not found - send reject back to sender
                if let (Some(actor), Some(endpoint)) = (sender_actor, sender_endpoint) {
                    let reject = Reject::new(
                        msg_type,
                        &format!("Actor '{}' not found", receiver_name),
                        receiver_name,
                    );
                    zmq_sender.send_to_async(endpoint, actor, Box::new(reject), None).await;
                }
                return;
            }
        };

        // Try to deserialize the message
        match try_deserialize_message(msg_type, msg_data.clone()) {
            Ok(msg) => {
                // Success - create sender ref and deliver to local actor
                let sender_ref = if let (Some(actor), Some(endpoint)) = (sender_actor, sender_endpoint) {
                    Some(ActorRef::Remote(RemoteActorRef::new(
                        actor,
                        endpoint,
                        Arc::clone(zmq_sender),
                    )))
                } else {
                    None
                };
                local_ref.send(msg, sender_ref);
            }
            Err(reason) => {
                // Deserialization failed - send reject back to sender
                if let (Some(actor), Some(endpoint)) = (sender_actor, sender_endpoint) {
                    let reject = Reject::new(msg_type, &reason, receiver_name);
                    zmq_sender.send_to_async(endpoint, actor, Box::new(reject), None).await;
                }
            }
        }
    }
}

/// Handle for controlling a running ZmqReceiver.
pub struct ZmqReceiverHandle {
    running: Arc<Mutex<bool>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ZmqReceiverHandle {
    /// Stop the receiver.
    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ZmqReceiverHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_actor_ref_creation() {
        let sender = Arc::new(ZmqSender::new("tcp://0.0.0.0:5555"));
        let remote_ref = RemoteActorRef::new("test_actor", "tcp://localhost:5556", sender);

        assert_eq!(remote_ref.name(), "test_actor");
        assert_eq!(remote_ref.endpoint(), "tcp://localhost:5556");
    }

    #[test]
    fn test_zmq_sender_creation() {
        let sender = ZmqSender::new("tcp://0.0.0.0:5557");
        assert_eq!(sender.local_endpoint(), "tcp://0.0.0.0:5557");
    }

    #[test]
    fn test_actor_registry() {
        use std::sync::mpsc::channel;
        use crate::actor::Envelope;

        let registry = ActorRegistry::new();
        let (tx, _rx) = channel::<Envelope>();
        let actor_ref = ActorRef::new(tx, "test".to_string());

        registry.register("test", actor_ref.clone());

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
