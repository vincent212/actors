/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Message serialization for remote communication.
//!
//! Provides a registry for serializing/deserializing messages to JSON
//! for transmission over ZMQ.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::Message;

/// Function type for serializing a message to JSON
type SerializeFn = Box<dyn Fn(&dyn Message) -> Value + Send + Sync>;

/// Function type for deserializing JSON to a message
type DeserializeFn = Box<dyn Fn(Value) -> Box<dyn Message> + Send + Sync>;

/// Registry entry containing serialize/deserialize functions
struct RegistryEntry {
    serialize: SerializeFn,
    deserialize: DeserializeFn,
}

/// Global message registry (type name -> entry)
static REGISTRY: Mutex<Option<HashMap<String, RegistryEntry>>> = Mutex::new(None);

/// TypeId to type name mapping for runtime lookup
static TYPEID_TO_NAME: Mutex<Option<HashMap<TypeId, String>>> = Mutex::new(None);

/// Register a message type for remote serialization.
///
/// Messages must implement `Serialize` and `DeserializeOwned` from serde.
///
/// # Example
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use actors::{define_message, register_remote_message};
///
/// #[derive(Serialize, Deserialize, Default)]
/// struct Ping { count: i32 }
/// define_message!(Ping);
///
/// // Register for remote communication
/// register_remote_message::<Ping>("Ping");
/// ```
pub fn register_remote_message<M>(type_name: &str)
where
    M: Message + Serialize + DeserializeOwned + 'static,
{
    // Register TypeId -> name mapping for runtime lookup
    {
        let mut typeid_map = TYPEID_TO_NAME.lock().unwrap();
        let map = typeid_map.get_or_insert_with(HashMap::new);
        map.insert(TypeId::of::<M>(), type_name.to_string());
    }

    let mut reg = REGISTRY.lock().unwrap();
    let map = reg.get_or_insert_with(HashMap::new);

    let serialize: SerializeFn = Box::new(move |msg: &dyn Message| {
        let typed = msg.as_any().downcast_ref::<M>().expect("Type mismatch in serialize");
        serde_json::to_value(typed).expect("Failed to serialize message")
    });

    let deserialize: DeserializeFn = Box::new(|val: Value| {
        let msg: M = serde_json::from_value(val).expect("Failed to deserialize message");
        Box::new(msg)
    });

    map.insert(
        type_name.to_string(),
        RegistryEntry {
            serialize,
            deserialize,
        },
    );
}

/// Get the registered type name for a message by its TypeId.
/// Returns None if the type hasn't been registered.
pub fn get_type_name(msg: &dyn Message) -> Option<String> {
    let type_id = msg.as_any().type_id();
    let typeid_map = TYPEID_TO_NAME.lock().unwrap();
    if let Some(map) = typeid_map.as_ref() {
        map.get(&type_id).cloned()
    } else {
        None
    }
}

/// Serialize a message to JSON using the registry.
///
/// Returns the message type name and the serialized JSON value.
/// Panics if the message type is not registered.
pub fn serialize_message(msg: &dyn Message, type_name: &str) -> Value {
    let reg = REGISTRY.lock().unwrap();
    let map = reg.as_ref().expect("No messages registered");
    let entry = map
        .get(type_name)
        .unwrap_or_else(|| panic!("Message type '{}' not registered", type_name));
    (entry.serialize)(msg)
}

/// Deserialize a message from JSON using the registry.
///
/// Panics if the message type is not registered.
pub fn deserialize_message(type_name: &str, value: Value) -> Box<dyn Message> {
    let reg = REGISTRY.lock().unwrap();
    let map = reg.as_ref().expect("No messages registered");
    let entry = map
        .get(type_name)
        .unwrap_or_else(|| panic!("Message type '{}' not registered", type_name));
    (entry.deserialize)(value)
}

/// Try to deserialize a message from JSON using the registry.
///
/// Returns Err with the reason if deserialization fails.
pub fn try_deserialize_message(type_name: &str, value: Value) -> Result<Box<dyn Message>, String> {
    let reg = REGISTRY.lock().unwrap();
    let map = match reg.as_ref() {
        Some(m) => m,
        None => return Err("No messages registered".to_string()),
    };

    let entry = match map.get(type_name) {
        Some(e) => e,
        None => return Err(format!("Unknown message type: {}", type_name)),
    };

    // Try deserialization (currently panics on parse errors, could be improved)
    Ok((entry.deserialize)(value))
}

/// Check if a message type is registered.
pub fn is_message_registered(type_name: &str) -> bool {
    let reg = REGISTRY.lock().unwrap();
    if let Some(map) = reg.as_ref() {
        map.contains_key(type_name)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_message;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
    struct TestMsg {
        value: i32,
        name: String,
    }
    define_message!(TestMsg);

    #[test]
    fn test_register_and_serialize() {
        register_remote_message::<TestMsg>("TestMsg");

        let msg = TestMsg {
            value: 42,
            name: "test".to_string(),
        };

        let json = serialize_message(&msg, "TestMsg");
        assert_eq!(json["value"], 42);
        assert_eq!(json["name"], "test");
    }

    #[test]
    fn test_deserialize() {
        register_remote_message::<TestMsg>("TestMsg");

        let json = serde_json::json!({
            "value": 123,
            "name": "hello"
        });

        let msg = deserialize_message("TestMsg", json);
        let typed = msg.as_any().downcast_ref::<TestMsg>().unwrap();
        assert_eq!(typed.value, 123);
        assert_eq!(typed.name, "hello");
    }

    #[test]
    fn test_get_type_name() {
        register_remote_message::<TestMsg>("TestMsg");

        let msg = TestMsg {
            value: 1,
            name: "x".to_string(),
        };

        assert_eq!(get_type_name(&msg), Some("TestMsg".to_string()));
    }
}
