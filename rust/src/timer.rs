/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Timer utilities for scheduling delayed and periodic messages.
//!
//! Timers send Timeout messages to actors after a specified delay.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::actor::ActorRef;
use crate::messages::Timeout;

/// A timer that sends Timeout messages to an actor.
///
/// # Example
/// ```
/// use actors::{Timer, ActorRef};
/// use std::time::Duration;
///
/// // One-shot timer
/// let timer = Timer::once(actor_ref, Duration::from_secs(5), 42);
///
/// // Periodic timer
/// let timer = Timer::periodic(actor_ref, Duration::from_millis(100), 1);
///
/// // Cancel when done
/// timer.cancel();
/// ```
pub struct Timer {
    handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl Timer {
    /// Create a one-shot timer.
    ///
    /// Sends a Timeout message to the actor after the specified delay.
    pub fn once(actor: ActorRef, delay: Duration, id: u64) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let handle = thread::spawn(move || {
            thread::sleep(delay);
            if running_clone.load(Ordering::SeqCst) {
                actor.send(Box::new(Timeout::new(id)), None);
            }
        });

        Timer {
            handle: Some(handle),
            running,
        }
    }

    /// Create a periodic timer.
    ///
    /// Sends Timeout messages to the actor at the specified interval.
    /// The timer runs until cancel() is called.
    pub fn periodic(actor: ActorRef, interval: Duration, id: u64) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let handle = thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                thread::sleep(interval);
                if running_clone.load(Ordering::SeqCst) {
                    actor.send(Box::new(Timeout::new(id)), None);
                }
            }
        });

        Timer {
            handle: Some(handle),
            running,
        }
    }

    /// Cancel the timer.
    ///
    /// No more Timeout messages will be sent after this call.
    pub fn cancel(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the timer is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.cancel();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Timer ID generator for unique timer IDs
static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a unique timer ID
pub fn next_timer_id() -> u64 {
    NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_timer_id_generation() {
        let id1 = next_timer_id();
        let id2 = next_timer_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_timer_cancel() {
        // Create a mock channel to act as actor
        let (tx, _rx) = channel();
        let actor_ref = ActorRef::new(tx, "test".to_string());

        let timer = Timer::periodic(actor_ref, Duration::from_millis(10), 1);
        assert!(timer.is_running());

        timer.cancel();
        assert!(!timer.is_running());
    }
}
