//! Rust Publisher for pub/sub example
//!
//! Receives Subscribe from subscribers, sends MarketUpdates back.
//! Uses ActorRef for location transparency - doesn't know if subscribers are C++ or Rust.
//!
//! Uses the standard Actor trait with handle_messages! macro.

use std::time::{SystemTime, UNIX_EPOCH};

use actors::{handle_messages, ActorContext, ActorRef, ManagerHandle};
use actors::messages::Start;
use crate::interop_messages::{Subscribe, MarketUpdate};
use crate::rust_manager_ffi::get_actor_ref;

pub struct RustPublisher {
    // ActorRef to subscriber - location transparent!
    cpp_subscriber: Option<ActorRef>,
    // Track subscribed topics
    topics: Vec<String>,
    // Count of updates sent (for demo purposes)
    update_count: i32,
    #[allow(dead_code)]
    manager_handle: ManagerHandle,
}

impl RustPublisher {
    pub fn new(manager_handle: ManagerHandle) -> Self {
        RustPublisher {
            // Will be looked up on first use
            cpp_subscriber: None,
            topics: Vec::new(),
            update_count: 0,
            manager_handle,
        }
    }

    /// Get the subscriber ActorRef, looking it up if needed
    fn get_subscriber(&mut self) -> Option<ActorRef> {
        if self.cpp_subscriber.is_none() {
            // Look up by name - works for C++ or Rust subscribers!
            self.cpp_subscriber = get_actor_ref("cpp_subscriber", "rust_publisher");
        }
        self.cpp_subscriber.clone()
    }

    fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
        println!("[Rust Publisher] Started");
    }

    fn on_subscribe(&mut self, msg: &Subscribe, _ctx: &mut ActorContext) {
        let topic = std::str::from_utf8(&msg.topic)
            .unwrap_or("")
            .trim_end_matches('\0')
            .to_string();

        println!("[Rust Publisher] Subscriber subscribing to '{}'", topic);

        if !self.topics.contains(&topic) {
            self.topics.push(topic.clone());
        }

        // Get subscriber ActorRef and send updates
        let subscriber = self.get_subscriber();

        // Send 3 updates via ActorRef - location transparent!
        for i in 0..3 {
            self.update_count += 1;
            let price = 150.0 + (i as f64 * 0.25);

            let mut update = MarketUpdate {
                symbol: [0u8; 8],
                price,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
                volume: (i + 1) * 100,
            };

            let bytes = topic.as_bytes();
            update.symbol[..bytes.len().min(7)].copy_from_slice(&bytes[..bytes.len().min(7)]);

            println!("[Rust Publisher] Sending update: {} @ ${:.2}", topic, price);

            if let Some(ref sub) = subscriber {
                sub.send(Box::new(update), None);
            }
        }
    }
}

// Register message handlers
handle_messages!(RustPublisher,
    Start => on_start,
    Subscribe => on_subscribe
);
