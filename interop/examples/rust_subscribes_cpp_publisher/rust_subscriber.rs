//! Rust Subscriber Example
//!
//! This Rust actor subscribes to a market data publisher.
//! It demonstrates:
//! - Location transparency - uses ActorRef without knowing if publisher is C++ or Rust
//! - Using get_actor_ref() to look up actors by name
//! - Receiving MarketUpdate messages from the publisher
//! - The pub/sub pattern across language boundaries
//!
//! Uses the standard Actor trait with handle_messages! macro.

use actors::{handle_messages, ActorContext, ActorRef, ManagerHandle};
use actors::messages::Start;

use crate::interop_messages::{Subscribe, Unsubscribe, MarketUpdate, MarketDepth};
use crate::rust_manager_ffi::get_actor_ref;

/// Price Monitor - subscribes to price feed and monitors updates
pub struct RustSubscriber {
    /// ActorRef to the publisher (could be C++ or Rust - we don't know or care!)
    publisher: Option<ActorRef>,
    /// Count of updates received
    update_count: i32,
    /// Subscribed topics
    subscribed_topics: Vec<String>,
    #[allow(dead_code)]
    manager_handle: ManagerHandle,
}

impl RustSubscriber {
    pub fn new(manager_handle: ManagerHandle) -> Self {
        RustSubscriber {
            // Will be looked up on first use via get_actor_ref()
            publisher: None,
            update_count: 0,
            subscribed_topics: Vec::new(),
            manager_handle,
        }
    }

    /// Get the publisher ActorRef, looking it up if needed
    fn get_publisher(&mut self) -> Option<ActorRef> {
        if self.publisher.is_none() {
            // Look up by name - works for C++ or Rust actors!
            self.publisher = get_actor_ref("cpp_price_feed", "rust_price_monitor");
        }
        self.publisher.clone()
    }

    fn on_start(&mut self, _msg: &Start, _ctx: &mut ActorContext) {
        println!("[Rust Subscriber] Started, subscribing to AAPL and GOOG...");
        self.subscribe("AAPL");
        self.subscribe("GOOG");
    }

    /// Subscribe to a market data topic
    pub fn subscribe(&mut self, symbol: &str) {
        println!("[Rust Subscriber] Subscribing to {}", symbol);

        let mut sub = Subscribe {
            topic: [0u8; 32],
        };

        // Copy symbol to fixed-size array
        let bytes = symbol.as_bytes();
        let copy_len = bytes.len().min(31);
        sub.topic[..copy_len].copy_from_slice(&bytes[..copy_len]);

        // Send via ActorRef - location transparent!
        if let Some(publisher) = self.get_publisher() {
            publisher.send(Box::new(sub), None);
        }
        self.subscribed_topics.push(symbol.to_string());
    }

    /// Handle incoming MarketUpdate message
    fn on_market_update(&mut self, msg: &MarketUpdate, _ctx: &mut ActorContext) {
        self.update_count += 1;

        // Extract symbol from fixed-size array
        let symbol = std::str::from_utf8(&msg.symbol)
            .unwrap_or("")
            .trim_end_matches('\0');

        println!(
            "[Rust Subscriber] Update #{}: {} @ {:.2} vol={} ts={}",
            self.update_count, symbol, msg.price, msg.volume, msg.timestamp
        );

        // After 10 updates, unsubscribe from first topic
        if self.update_count == 10 && !self.subscribed_topics.is_empty() {
            let topic = self.subscribed_topics.remove(0);

            let mut unsub = Unsubscribe {
                topic: [0u8; 32],
            };
            let bytes = topic.as_bytes();
            let copy_len = bytes.len().min(31);
            unsub.topic[..copy_len].copy_from_slice(&bytes[..copy_len]);

            if let Some(publisher) = self.get_publisher() {
                publisher.send(Box::new(unsub), None);
            }
            println!("[Rust Subscriber] Unsubscribed from {}", topic);
        }
    }

    /// Handle incoming MarketDepth message (demonstrates array handling)
    fn on_market_depth(&mut self, msg: &MarketDepth, _ctx: &mut ActorContext) {
        let symbol = std::str::from_utf8(&msg.symbol)
            .unwrap_or("")
            .trim_end_matches('\0');

        println!("[Rust Subscriber] Market Depth for {}:", symbol);
        for i in 0..msg.num_levels as usize {
            println!(
                "  Level {}: bid {:.2} x {} | ask {:.2} x {}",
                i + 1,
                msg.bid_prices[i],
                msg.bid_sizes[i],
                msg.ask_prices[i],
                msg.ask_sizes[i]
            );
        }
    }
}

// Register message handlers
handle_messages!(RustSubscriber,
    Start => on_start,
    MarketUpdate => on_market_update,
    MarketDepth => on_market_depth
);
