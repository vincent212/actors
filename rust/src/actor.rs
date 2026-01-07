/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Actor trait and ActorRef for the actor framework.
//!
//! Actors are independent entities that process messages sequentially.
//! Each actor runs in its own thread with isolated state.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::Message;

/// Envelope wraps a message with sender metadata.
///
/// This separates the immutable message from mutable routing information.
pub struct Envelope {
    /// The actual message (boxed trait object)
    pub msg: Box<dyn Message>,
    /// The sender's ActorRef (for replies)
    pub sender: Option<ActorRef>,
    /// For fast_send: channel to send reply back
    pub reply_channel: Option<Sender<Box<dyn Message>>>,
}

impl Envelope {
    /// Create a new envelope for async send
    pub fn new(msg: Box<dyn Message>, sender: Option<ActorRef>) -> Self {
        Envelope {
            msg,
            sender,
            reply_channel: None,
        }
    }

    /// Create a new envelope for fast_send (synchronous)
    pub fn new_sync(
        msg: Box<dyn Message>,
        sender: Option<ActorRef>,
        reply_channel: Sender<Box<dyn Message>>,
    ) -> Self {
        Envelope {
            msg,
            sender,
            reply_channel: Some(reply_channel),
        }
    }
}

/// Local reference to an actor, used for sending messages via channel.
///
/// LocalActorRef is a cloneable handle that acts as the "address" of a local actor.
/// For remote actors, use RemoteActorRef.
#[derive(Clone)]
pub struct LocalActorRef {
    sender: Sender<Envelope>,
    name: String,
}

impl LocalActorRef {
    /// Create a new LocalActorRef from a sender
    pub fn new(sender: Sender<Envelope>, name: String) -> Self {
        LocalActorRef { sender, name }
    }

    /// Get the actor's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send a message to this actor (async, fire-and-forget)
    pub fn send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) {
        let envelope = Envelope::new(msg, sender);
        let _ = self.sender.send(envelope);
    }

    /// Send a message and wait for a reply (synchronous)
    pub fn fast_send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) -> Option<Box<dyn Message>> {
        let (reply_tx, reply_rx) = channel();
        let envelope = Envelope::new_sync(msg, sender, reply_tx);
        if self.sender.send(envelope).is_ok() {
            reply_rx.recv().ok()
        } else {
            None
        }
    }

    /// Convert to an ActorRef enum
    pub fn into_actor_ref(self) -> ActorRef {
        ActorRef::Local(self)
    }
}

/// Type for the FFI send function that dispatches messages to C++ actors.
///
/// The send function takes:
/// - target: target actor name
/// - sender: sender actor name
/// - msg: the boxed message trait object
///
/// The implementation (provided by actors-interop) extracts message_id(),
/// converts to C struct, and calls the FFI function internally.
pub type CppSendFn = fn(&str, &str, &dyn crate::Message) -> i32;

/// Reference to a C++ actor via FFI.
///
/// CppActorRef is location-transparent - actors just call send() like any other ActorRef.
/// The FFI dispatch happens internally via the send_fn provided by actors-interop.
#[derive(Clone)]
pub struct CppActorRef {
    target_name: String,
    sender_name: String,
    /// Function pointer for sending messages via FFI
    /// This function handles message_id extraction and C struct conversion internally
    send_fn: CppSendFn,
}

impl CppActorRef {
    /// Create a new CppActorRef with a send function
    pub fn new(target: &str, sender: &str, send_fn: CppSendFn) -> Self {
        CppActorRef {
            target_name: target.to_string(),
            sender_name: sender.to_string(),
            send_fn,
        }
    }

    /// Get the target actor's name
    pub fn name(&self) -> &str {
        &self.target_name
    }

    /// Get the sender's name
    pub fn sender_name(&self) -> &str {
        &self.sender_name
    }

    /// Send a message to the C++ actor (called internally by ActorRef::send)
    pub(crate) fn send_message(&self, msg: &dyn crate::Message) -> i32 {
        (self.send_fn)(&self.target_name, &self.sender_name, msg)
    }
}

/// Reference to an actor (local, remote, or C++ via FFI).
///
/// ActorRef is an enum that can hold either a local, remote, or C++ actor reference.
/// This provides polymorphism without trait objects.
#[derive(Clone)]
pub enum ActorRef {
    /// Local actor - uses channel
    Local(LocalActorRef),
    /// Remote actor - uses ZMQ
    Remote(crate::remote::RemoteActorRef),
    /// C++ actor - uses FFI
    Cpp(CppActorRef),
}

impl ActorRef {
    /// Create a new local ActorRef from a sender (convenience method)
    pub fn new(sender: Sender<Envelope>, name: String) -> Self {
        ActorRef::Local(LocalActorRef::new(sender, name))
    }

    /// Get the actor's name
    pub fn name(&self) -> &str {
        match self {
            ActorRef::Local(r) => r.name(),
            ActorRef::Remote(r) => r.name(),
            ActorRef::Cpp(r) => r.name(),
        }
    }

    /// Send a message to this actor (async, fire-and-forget)
    ///
    /// The message is queued and processed later by the receiver's thread.
    /// For Cpp actors, the message is dispatched via FFI transparently.
    pub fn send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) {
        match self {
            ActorRef::Local(r) => r.send(msg, sender),
            ActorRef::Remote(r) => r.send(msg, sender),
            ActorRef::Cpp(r) => {
                // Dispatch via FFI - the send_fn handles message_id extraction and C struct conversion
                r.send_message(msg.as_ref());
            }
        }
    }

    /// Send a message and wait for a reply (synchronous)
    ///
    /// The handler runs in the receiver's thread, but the caller blocks
    /// until a reply is received.
    ///
    /// Note: fast_send is only fully supported for local actors.
    /// For Cpp actors, the message is sent but no reply is returned (fire-and-forget behavior).
    pub fn fast_send(&self, msg: Box<dyn Message>, sender: Option<ActorRef>) -> Option<Box<dyn Message>> {
        match self {
            ActorRef::Local(r) => r.fast_send(msg, sender),
            ActorRef::Remote(_) => None, // fast_send not supported for remote
            ActorRef::Cpp(r) => {
                // For Cpp, we send but cannot receive a reply via FFI
                // This behaves like send() - fire-and-forget
                r.send_message(msg.as_ref());
                None
            }
        }
    }

    /// Check if this is a local actor reference
    pub fn is_local(&self) -> bool {
        matches!(self, ActorRef::Local(_))
    }

    /// Check if this is a remote actor reference
    pub fn is_remote(&self) -> bool {
        matches!(self, ActorRef::Remote(_))
    }

    /// Check if this is a C++ actor reference
    pub fn is_cpp(&self) -> bool {
        matches!(self, ActorRef::Cpp(_))
    }

    /// Get the endpoint if this is a remote actor ref
    pub fn endpoint(&self) -> Option<&str> {
        match self {
            ActorRef::Local(_) => None,
            ActorRef::Remote(r) => Some(r.endpoint()),
            ActorRef::Cpp(_) => None,
        }
    }

    /// Create a new C++ actor reference (used internally by actors-interop)
    pub fn cpp(target: &str, sender: &str, send_fn: CppSendFn) -> Self {
        ActorRef::Cpp(CppActorRef::new(target, sender, send_fn))
    }
}

/// Context passed to actors for sending messages and replies.
pub struct ActorContext {
    /// This actor's reference (for passing as sender)
    self_ref: Option<ActorRef>,
    /// Who sent the current message (for replies)
    sender: Option<ActorRef>,
    /// Reply channel from fast_send
    reply_channel: Option<Sender<Box<dyn Message>>>,
}

impl ActorContext {
    pub fn new() -> Self {
        ActorContext {
            self_ref: None,
            sender: None,
            reply_channel: None,
        }
    }

    /// Reply to the current message
    ///
    /// For fast_send, sends through the reply channel.
    /// For regular send, sends to the sender's mailbox.
    pub fn reply(&mut self, msg: Box<dyn Message>) {
        if let Some(tx) = self.reply_channel.take() {
            // fast_send: send reply through dedicated channel
            let _ = tx.send(msg);
        } else if let Some(ref sender) = self.sender {
            // Regular send: send to sender's mailbox
            sender.send(msg, self.self_ref.clone());
        }
        // else: no sender provided, reply is dropped
    }

    /// Get this actor's reference (mailbox address)
    pub fn self_ref(&self) -> Option<ActorRef> {
        self.self_ref.clone()
    }

    /// Set this actor's reference (called by runtime)
    pub fn set_self_ref(&mut self, actor_ref: ActorRef) {
        self.self_ref = Some(actor_ref);
    }

    /// Set up context for processing an envelope
    pub fn prepare_for_envelope(&mut self, sender: Option<ActorRef>, reply_channel: Option<Sender<Box<dyn Message>>>) {
        self.sender = sender;
        self.reply_channel = reply_channel;
    }
}

impl Default for ActorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for all actors in the system.
///
/// Implement this trait to create your own actors.
/// Use the `handle_messages!` macro to register message handlers.
pub trait Actor: Send + 'static {
    /// Called once when the actor starts, before processing any messages.
    ///
    /// Use this for initialization that doesn't require messaging.
    fn init(&mut self) {}

    /// Called once when the actor shuts down, after all messages are processed.
    ///
    /// Use this for cleanup.
    fn end(&mut self) {}

    /// Process a message.
    ///
    /// Use the `handle_messages!` macro to implement this method cleanly.
    fn process_message(&mut self, _msg: &dyn Message, _ctx: &mut ActorContext) {}
}

/// Macro to cleanly dispatch messages to handler methods.
///
/// This macro generates the `process_message` implementation with type-safe
/// message dispatch.
///
/// # Example
/// ```rust
/// use actors::{Actor, ActorContext, Message, define_message, handle_messages};
///
/// struct Ping { count: i32 }
/// define_message!(Ping);
///
/// struct Pong { count: i32 }
/// define_message!(Pong);
///
/// struct MyActor {
///     other_actor: ActorRef,
/// }
///
/// impl Actor for MyActor {}
///
/// handle_messages!(MyActor,
///     Ping => on_ping,
///     Pong => on_pong
/// );
///
/// impl MyActor {
///     fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
///         println!("Got ping: {}", msg.count);
///         ctx.reply(Box::new(Pong { count: msg.count }));
///     }
///
///     fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
///         println!("Got pong: {}", msg.count);
///     }
/// }
/// ```
#[macro_export]
macro_rules! handle_messages {
    ($actor_type:ty, $($msg_type:ty => $handler:ident),+ $(,)?) => {
        impl $actor_type {
            fn dispatch_message(&mut self, msg: &dyn $crate::Message, ctx: &mut $crate::ActorContext) -> bool {
                $(
                    if let Some(typed_msg) = msg.as_any().downcast_ref::<$msg_type>() {
                        self.$handler(typed_msg, ctx);
                        return true;
                    }
                )+
                false
            }
        }

        impl $crate::Actor for $actor_type {
            fn process_message(&mut self, msg: &dyn $crate::Message, ctx: &mut $crate::ActorContext) {
                self.dispatch_message(msg, ctx);
            }
        }
    };
}

/// Runtime for a single actor, manages message loop
pub struct ActorRuntime {
    pub actor: Box<dyn Actor>,
    pub receiver: Receiver<Envelope>,
    pub sender: Sender<Envelope>,
    pub name: String,
    pub context: ActorContext,
    pub running: Arc<Mutex<bool>>,
}

impl ActorRuntime {
    /// Create a new actor runtime
    pub fn new(name: String, actor: Box<dyn Actor>) -> Self {
        let (sender, receiver) = channel();
        let actor_ref = ActorRef::new(sender.clone(), name.clone());

        let mut context = ActorContext::new();
        context.set_self_ref(actor_ref);

        ActorRuntime {
            actor,
            receiver,
            sender,
            name,
            context,
            running: Arc::new(Mutex::new(true)),
        }
    }

    /// Get an ActorRef for this actor
    pub fn get_ref(&self) -> ActorRef {
        ActorRef::new(self.sender.clone(), self.name.clone())
    }

    /// Run the actor's message loop
    pub fn run(&mut self) {
        self.actor.init();

        while *self.running.lock().unwrap() {
            match self.receiver.recv() {
                Ok(envelope) => {
                    self.dispatch(envelope);
                }
                Err(_) => {
                    // Channel closed, exit
                    break;
                }
            }
        }

        self.actor.end();
    }

    /// Dispatch a message to the actor
    fn dispatch(&mut self, envelope: Envelope) {
        let msg = envelope.msg;

        // Set up context for this message
        self.context.prepare_for_envelope(envelope.sender, envelope.reply_channel);

        // Call the actor's process_message
        self.actor.process_message(msg.as_ref(), &mut self.context);
    }

    /// Signal the actor to stop
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::define_message;

    struct TestMessage {
        value: i32,
    }
    define_message!(TestMessage);

    struct TestActor {
        received: i32,
    }

    impl Actor for TestActor {
        fn process_message(&mut self, msg: &dyn Message, _ctx: &mut ActorContext) {
            if let Some(m) = msg.as_any().downcast_ref::<TestMessage>() {
                self.received = m.value;
            }
        }
    }

    #[test]
    fn test_actor_ref_clone() {
        let (tx, _rx) = channel();
        let actor_ref = ActorRef::new(tx, "test".to_string());
        let cloned = actor_ref.clone();
        assert_eq!(actor_ref.name(), cloned.name());
    }

    #[test]
    fn test_envelope_creation() {
        let msg = Box::new(TestMessage { value: 42 });
        let envelope = Envelope::new(msg, None);
        assert!(envelope.sender.is_none());
        assert!(envelope.reply_channel.is_none());
    }
}
