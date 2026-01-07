/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Message types and traits for the actor framework.
//!
//! All messages must implement the `Message` trait. Use the `define_message!` macro
//! for easy implementation.

use std::any::Any;

/// Trait for all messages in the actor system.
///
/// Messages must be Send + 'static to be passed between threads.
///
/// # Example
/// ```
/// use actors::define_message;
///
/// struct Ping { count: i32 }
/// define_message!(Ping);
///
/// struct Pong { count: i32 }
/// define_message!(Pong);
/// ```
pub trait Message: Any + Send + 'static {
    /// For downcasting to concrete type
    fn as_any(&self) -> &dyn Any;

    /// For downcasting to concrete type (mutable)
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the message ID (for FFI dispatch)
    /// Returns 0 for non-interop messages (default)
    fn message_id(&self) -> i32 {
        0
    }
}

/// Macro to implement the Message trait for a type.
///
/// # Example
/// ```
/// use actors::define_message;
///
/// struct MyMessage {
///     data: String,
/// }
/// define_message!(MyMessage);
/// ```
#[macro_export]
macro_rules! define_message {
    ($name:ty) => {
        impl $crate::Message for $name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}

// Re-export for use in macro
pub use std::any::Any as StdAny;

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;

    struct TestMessage {
        value: i32,
    }
    define_message!(TestMessage);

    #[test]
    fn test_downcast() {
        let msg = TestMessage { value: 42 };
        let any_ref = msg.as_any();
        let downcasted = any_ref.downcast_ref::<TestMessage>().unwrap();
        assert_eq!(downcasted.value, 42);
    }
}
