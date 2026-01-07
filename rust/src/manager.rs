/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Actor lifecycle manager.
//!
//! The Manager orchestrates actor lifecycle:
//! - Registers actors and starts their threads
//! - Handles CPU affinity and thread priority
//! - Coordinates startup and shutdown
//! - Provides actor registry for name-based lookup

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crate::actor::{ActorRef, ActorRuntime};
use crate::messages::{Shutdown, Start};
use crate::Actor;

/// Type for the C++ actor lookup function
/// Returns Some(ActorRef) if the actor exists in C++, None otherwise
pub type CppActorLookupFn = fn(&str, &str) -> Option<ActorRef>;

/// Global C++ actor lookup function (set by actors-interop)
static CPP_LOOKUP: RwLock<Option<CppActorLookupFn>> = RwLock::new(None);

/// Register a C++ actor lookup function.
///
/// This is called by actors-interop to enable cross-language actor lookup.
pub fn register_cpp_lookup(lookup_fn: CppActorLookupFn) {
    *CPP_LOOKUP.write().unwrap() = Some(lookup_fn);
}

/// Check if an actor exists in C++.
fn lookup_cpp_actor(name: &str, sender: &str) -> Option<ActorRef> {
    if let Some(lookup_fn) = *CPP_LOOKUP.read().unwrap() {
        lookup_fn(name, sender)
    } else {
        None
    }
}

/// Configuration for an actor's thread
#[derive(Clone)]
pub struct ThreadConfig {
    /// CPU cores to pin the thread to (empty = no pinning)
    pub affinity: Vec<usize>,
    /// Thread priority (1-99 for RT, 0 = default)
    pub priority: i32,
    /// Scheduling policy (SCHED_OTHER, SCHED_FIFO, SCHED_RR)
    pub sched_policy: i32,
}

impl Default for ThreadConfig {
    fn default() -> Self {
        ThreadConfig {
            affinity: vec![],
            priority: 0,
            sched_policy: libc::SCHED_OTHER,
        }
    }
}

impl ThreadConfig {
    /// Create config with CPU affinity
    pub fn with_affinity(cores: Vec<usize>) -> Self {
        ThreadConfig {
            affinity: cores,
            ..Default::default()
        }
    }

    /// Create config with real-time priority
    pub fn with_priority(priority: i32, policy: i32) -> Self {
        ThreadConfig {
            priority,
            sched_policy: policy,
            ..Default::default()
        }
    }

    /// Create config with both affinity and priority
    pub fn new(affinity: Vec<usize>, priority: i32, policy: i32) -> Self {
        ThreadConfig {
            affinity,
            priority,
            sched_policy: policy,
        }
    }
}

/// Handle for actors to signal termination to the manager.
///
/// This is a cloneable handle that actors can use to signal the manager
/// to shut down. Clone it and pass it to actors that need to trigger shutdown.
#[derive(Clone)]
pub struct ManagerHandle {
    terminate_flag: Arc<AtomicBool>,
}

impl ManagerHandle {
    /// Signal the manager to terminate.
    ///
    /// This will cause the manager's `run()` method to return.
    pub fn terminate(&self) {
        self.terminate_flag.store(true, Ordering::SeqCst);
    }

    /// Check if termination has been signaled.
    pub fn is_terminated(&self) -> bool {
        self.terminate_flag.load(Ordering::SeqCst)
    }
}

/// Manages the lifecycle of actors.
///
/// # Example
/// ```
/// use actors::{Manager, Actor, ActorContext, ThreadConfig};
///
/// struct MyActor {
///     manager: ManagerHandle,
/// }
///
/// impl Actor for MyActor {
///     fn process_message(&mut self, msg: &dyn Message, ctx: &mut ActorContext) {
///         // Signal termination when done
///         self.manager.terminate();
///     }
/// }
///
/// let mut mgr = Manager::new();
/// let handle = mgr.get_handle();
/// mgr.manage("my_actor", Box::new(MyActor { manager: handle }), Default::default());
/// mgr.init();
/// mgr.run();  // Blocks until terminate() is called
/// mgr.end();
/// ```
pub struct Manager {
    /// Actor name -> ActorRef registry
    registry: HashMap<String, ActorRef>,
    /// Actor runtimes (owned until init())
    runtimes: Vec<(ActorRuntime, ThreadConfig)>,
    /// Thread handles (after init())
    threads: Vec<JoinHandle<()>>,
    /// Actor refs for shutdown
    actor_refs: Vec<ActorRef>,
    /// Termination flag
    terminate_flag: Arc<AtomicBool>,
}

impl Manager {
    /// Create a new Manager
    pub fn new() -> Self {
        Manager {
            registry: HashMap::new(),
            runtimes: Vec::new(),
            threads: Vec::new(),
            actor_refs: Vec::new(),
            terminate_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle for actors to signal termination.
    ///
    /// Clone this handle and pass it to actors that need to trigger shutdown.
    pub fn get_handle(&self) -> ManagerHandle {
        ManagerHandle {
            terminate_flag: Arc::clone(&self.terminate_flag),
        }
    }

    /// Register an actor to be managed.
    ///
    /// Returns an ActorRef that can be used to send messages to the actor.
    pub fn manage(
        &mut self,
        name: &str,
        actor: Box<dyn Actor>,
        config: ThreadConfig,
    ) -> ActorRef {
        let runtime = ActorRuntime::new(name.to_string(), actor);
        let actor_ref = runtime.get_ref();

        self.registry.insert(name.to_string(), actor_ref.clone());
        self.actor_refs.push(actor_ref.clone());
        self.runtimes.push((runtime, config));

        actor_ref
    }

    /// Get an ActorRef by name.
    ///
    /// Checks local Rust actors first, then C++ actors if a lookup function is registered.
    /// Returns None if no actor with that name is registered in either.
    pub fn get_ref(&self, name: &str) -> Option<ActorRef> {
        // Check local Rust actors first
        if let Some(actor_ref) = self.registry.get(name) {
            return Some(actor_ref.clone());
        }

        // Check C++ actors (if lookup function registered)
        lookup_cpp_actor(name, "")
    }

    /// Get an ActorRef by name with a sender name for C++ actors.
    ///
    /// Use this when you need to specify the sender for cross-language messaging.
    pub fn get_ref_with_sender(&self, name: &str, sender: &str) -> Option<ActorRef> {
        // Check local Rust actors first
        if let Some(actor_ref) = self.registry.get(name) {
            return Some(actor_ref.clone());
        }

        // Check C++ actors (if lookup function registered)
        lookup_cpp_actor(name, sender)
    }

    /// Get all registered actor names.
    pub fn get_names(&self) -> Vec<String> {
        self.registry.keys().cloned().collect()
    }

    /// Start all managed actors.
    ///
    /// Sends Start message to each actor and launches their threads.
    pub fn init(&mut self) {
        // Take ownership of runtimes
        let runtimes = std::mem::take(&mut self.runtimes);

        for (runtime, config) in runtimes {
            let actor_ref = runtime.get_ref();

            // Send Start message
            actor_ref.send(Box::new(Start), None);

            // Spawn thread with config
            let handle = spawn_with_config(runtime, config);
            self.threads.push(handle);
        }
    }

    /// Run until terminate() is called.
    ///
    /// This blocks until an actor calls `manager_handle.terminate()`.
    pub fn run(&self) {
        while !self.terminate_flag.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Signal all actors to shut down.
    ///
    /// Sends Shutdown message to each actor.
    pub fn shutdown(&self) {
        for actor_ref in &self.actor_refs {
            actor_ref.send(Box::new(Shutdown), None);
        }
    }

    /// Wait for all actor threads to finish.
    pub fn wait(&mut self) {
        let threads = std::mem::take(&mut self.threads);
        for handle in threads {
            let _ = handle.join();
        }
    }

    /// Shutdown and wait for all actors.
    pub fn end(&mut self) {
        self.shutdown();
        self.wait();
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn an actor thread with the given configuration
fn spawn_with_config(mut runtime: ActorRuntime, config: ThreadConfig) -> JoinHandle<()> {
    thread::spawn(move || {
        // Set CPU affinity if specified
        if !config.affinity.is_empty() {
            set_affinity(&config.affinity);
        }

        // Set thread priority if specified
        if config.priority > 0 {
            set_priority(config.priority, config.sched_policy);
        }

        // Run the actor
        runtime.run();
    })
}

/// Set CPU affinity for the current thread
fn set_affinity(cores: &[usize]) {
    #[cfg(target_os = "linux")]
    unsafe {
        let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
        for &core in cores {
            libc::CPU_SET(core, &mut cpuset);
        }
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpuset);
    }
}

/// Set thread priority
fn set_priority(priority: i32, policy: i32) {
    #[cfg(target_os = "linux")]
    unsafe {
        let param = libc::sched_param {
            sched_priority: priority,
        };
        libc::sched_setscheduler(0, policy, &param);
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    struct CountingActor {
        count: Arc<AtomicI32>,
    }

    impl Actor for CountingActor {
        fn init(&mut self) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_manager_registry() {
        let mut mgr = Manager::new();

        struct DummyActor;
        impl Actor for DummyActor {}

        let _ref1 = mgr.manage("actor1", Box::new(DummyActor), Default::default());
        let _ref2 = mgr.manage("actor2", Box::new(DummyActor), Default::default());

        assert!(mgr.get_ref("actor1").is_some());
        assert!(mgr.get_ref("actor2").is_some());
        assert!(mgr.get_ref("actor3").is_none());

        let names = mgr.get_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_thread_config() {
        let config = ThreadConfig::with_affinity(vec![0, 1]);
        assert_eq!(config.affinity, vec![0, 1]);
        assert_eq!(config.priority, 0);

        let config2 = ThreadConfig::with_priority(50, libc::SCHED_FIFO);
        assert_eq!(config2.priority, 50);
        assert_eq!(config2.sched_policy, libc::SCHED_FIFO);
    }

    #[test]
    fn test_manager_handle() {
        let mgr = Manager::new();
        let handle = mgr.get_handle();

        assert!(!handle.is_terminated());
        handle.terminate();
        assert!(handle.is_terminated());
    }
}
