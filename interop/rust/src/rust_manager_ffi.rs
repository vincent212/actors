//! FFI functions for C++ to manage Rust actors
//!
//! Provides extern "C" functions to:
//! - Create a Rust Manager
//! - Register actors with the Manager
//! - Initialize and run the Manager
//! - Shutdown
//! - Register C++ actor lookup for cross-language transparency

use std::ffi::CString;
use std::sync::Mutex;
use actors::{register_cpp_lookup, ActorRef, CppActorRef, Manager, ThreadConfig};
use crate::ping_pong::RustPongActor;
use crate::rust_ping::PingActor;
use crate::pubsub::RustPublisher;
use crate::rust_subscriber::RustSubscriber;

// Wrapper to make Manager pointer safe for static storage
struct ManagerPtr(*mut Manager);
unsafe impl Send for ManagerPtr {}
unsafe impl Sync for ManagerPtr {}

// Global Manager - created and owned by this module
// Use Mutex with a pointer wrapper since Manager doesn't impl Sync
static RUST_MANAGER: Mutex<ManagerPtr> = Mutex::new(ManagerPtr(std::ptr::null_mut()));

/// Create the Rust Manager
/// Call this once at startup before registering actors
#[no_mangle]
pub extern "C" fn create_rust_manager() {
    let mgr = Box::new(Manager::new());
    let ptr = Box::into_raw(mgr);
    let mut guard = RUST_MANAGER.lock().unwrap();
    guard.0 = ptr;
}

/// Register the PingActor with the Rust Manager
/// Returns the Manager pointer for rust_actor_init()
/// Note: init_cpp_actor_lookup() must be called first so we can find cpp_pong
#[no_mangle]
pub extern "C" fn register_rust_ping_actor() -> *const Manager {
    // Look up cpp_pong ActorRef BEFORE acquiring manager lock to avoid deadlock
    // (cpp_actor_lookup doesn't need RUST_MANAGER lock)
    let pong_ref = cpp_actor_lookup("cpp_pong", "rust_ping")
        .expect("cpp_pong not found - call init_cpp_actor_lookup() first");

    let mut guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &mut *guard.0 };
        let handle = mgr.get_handle();

        let actor = PingActor::new(pong_ref, handle);
        mgr.manage("rust_ping", Box::new(actor), ThreadConfig::default());
        guard.0 as *const Manager
    } else {
        std::ptr::null()
    }
}

/// Register the RustPongActor with the Rust Manager
/// Returns the Manager pointer for rust_actor_init()
#[no_mangle]
pub extern "C" fn register_rust_pong_actor() -> *const Manager {
    let mut guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &mut *guard.0 };
        let handle = mgr.get_handle();
        let actor = RustPongActor::new(handle);
        mgr.manage("rust_pong", Box::new(actor), ThreadConfig::default());
        guard.0 as *const Manager
    } else {
        std::ptr::null()
    }
}

/// Get pointer to the Rust Manager
/// For passing to rust_actor_init()
#[no_mangle]
pub extern "C" fn get_rust_manager() -> *const Manager {
    let guard = RUST_MANAGER.lock().unwrap();
    guard.0 as *const Manager
}

/// Get an ActorRef by name from the global Manager.
///
/// This provides location transparency - the caller doesn't know if the actor
/// is in Rust or C++. Use this in Rust actors to look up other actors.
///
/// # Arguments
/// * `name` - The actor name to look up
/// * `sender` - The sender name (for C++ actors that need it)
///
/// # Returns
/// Some(ActorRef) if found, None otherwise
pub fn get_actor_ref(name: &str, sender: &str) -> Option<ActorRef> {
    let guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &*guard.0 };
        mgr.get_ref_with_sender(name, sender)
    } else {
        None
    }
}

/// Initialize and start all Rust actors
/// This sends Start message to all actors
#[no_mangle]
pub extern "C" fn rust_manager_init() {
    let mut guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &mut *guard.0 };
        mgr.init();
    }
}

/// Shutdown all Rust actors and wait for threads to finish
#[no_mangle]
pub extern "C" fn rust_manager_end() {
    let mut guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &mut *guard.0 };
        mgr.end();
    }
}

/// Register the RustPublisher with the Rust Manager
/// Returns the Manager pointer for rust_actor_init()
#[no_mangle]
pub extern "C" fn register_rust_publisher() -> *const Manager {
    let mut guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &mut *guard.0 };
        let handle = mgr.get_handle();
        let actor = RustPublisher::new(handle);
        mgr.manage("rust_publisher", Box::new(actor), ThreadConfig::default());
        guard.0 as *const Manager
    } else {
        std::ptr::null()
    }
}

/// Register the RustSubscriber with the Rust Manager
/// Returns the Manager pointer for rust_actor_init()
#[no_mangle]
pub extern "C" fn register_rust_subscriber() -> *const Manager {
    let mut guard = RUST_MANAGER.lock().unwrap();
    if !guard.0.is_null() {
        let mgr = unsafe { &mut *guard.0 };
        let handle = mgr.get_handle();
        let actor = RustSubscriber::new(handle);
        mgr.manage("rust_price_monitor", Box::new(actor), ThreadConfig::default());
        guard.0 as *const Manager
    } else {
        std::ptr::null()
    }
}

// ============================================================================
// C++ Actor Lookup Integration
// ============================================================================

use std::os::raw::{c_char, c_int, c_void};

// FFI functions to send to C++ actors
extern "C" {
    fn cpp_actor_send(
        actor_name: *const c_char,
        sender_name: *const c_char,
        msg_type: c_int,
        msg_data: *const c_void,
    ) -> c_int;

    fn cpp_actor_exists(name: *const c_char) -> c_int;
}

/// The send function that will be passed to CppActorRef.
/// This dispatches by message_id, downcasts to concrete type, converts to C struct,
/// and calls the FFI function. Actors just call send() - they don't know about FFI.
fn cpp_send_fn(target: &str, sender: &str, msg: &dyn actors::Message) -> i32 {
    use crate::interop_messages::*;

    let target_cstr = CString::new(target).unwrap();
    let sender_cstr = if sender.is_empty() {
        None
    } else {
        Some(CString::new(sender).unwrap())
    };
    let sender_ptr = sender_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());

    let msg_id = msg.message_id();

    // Dispatch by message ID, downcast, convert to C struct, call FFI
    match msg_id {
        MSG_PING => {
            if let Some(m) = msg.as_any().downcast_ref::<Ping>() {
                let c_msg = m.to_c_struct();
                unsafe { cpp_actor_send(target_cstr.as_ptr(), sender_ptr, msg_id, &c_msg as *const _ as *const c_void) }
            } else { -3 }
        }
        MSG_PONG => {
            if let Some(m) = msg.as_any().downcast_ref::<Pong>() {
                let c_msg = m.to_c_struct();
                unsafe { cpp_actor_send(target_cstr.as_ptr(), sender_ptr, msg_id, &c_msg as *const _ as *const c_void) }
            } else { -3 }
        }
        MSG_SUBSCRIBE => {
            if let Some(m) = msg.as_any().downcast_ref::<Subscribe>() {
                let c_msg = m.to_c_struct();
                unsafe { cpp_actor_send(target_cstr.as_ptr(), sender_ptr, msg_id, &c_msg as *const _ as *const c_void) }
            } else { -3 }
        }
        MSG_MARKETUPDATE => {
            if let Some(m) = msg.as_any().downcast_ref::<MarketUpdate>() {
                let c_msg = m.to_c_struct();
                unsafe { cpp_actor_send(target_cstr.as_ptr(), sender_ptr, msg_id, &c_msg as *const _ as *const c_void) }
            } else { -3 }
        }
        _ => -2  // Unknown message type
    }
}

/// Lookup function for C++ actors
/// Returns Some(ActorRef::Cpp) if the actor exists in C++
fn cpp_actor_lookup(name: &str, sender: &str) -> Option<ActorRef> {
    let name_cstr = CString::new(name).unwrap();
    unsafe {
        if cpp_actor_exists(name_cstr.as_ptr()) != 0 {
            Some(ActorRef::Cpp(CppActorRef::new(name, sender, cpp_send_fn)))
        } else {
            None
        }
    }
}

/// Initialize C++ actor lookup for cross-language transparency.
/// Call this after cpp_actor_init() and before using Manager::get_ref().
#[no_mangle]
pub extern "C" fn init_cpp_actor_lookup() {
    register_cpp_lookup(cpp_actor_lookup);
}
