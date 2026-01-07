/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

*/

//! Ping-Pong Example - Two actors exchanging messages
//!
//! Demonstrates:
//! - Creating custom actors
//! - Defining custom messages
//! - Using handle_messages! macro for message dispatch
//! - Using ManagerHandle to signal termination
//! - send() for async messaging
//! - reply() for responding to messages

use actors::{define_message, handle_messages, ActorContext, ActorRef, Manager, ManagerHandle, Start, ThreadConfig};

// Custom messages
struct Ping {
    count: i32,
}
define_message!(Ping);

struct Pong {
    count: i32,
}
define_message!(Pong);

// Actor that receives Pong and sends Ping
struct PingActor {
    pong_actor: ActorRef,
    manager_handle: ManagerHandle,
    max_count: i32,
}

impl PingActor {
    fn new(pong_actor: ActorRef, manager_handle: ManagerHandle, max_count: i32) -> Self {
        PingActor {
            pong_actor,
            manager_handle,
            max_count,
        }
    }

    // Handler for Start message
    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("PingActor: Starting ping-pong");
        self.pong_actor.send(Box::new(Ping { count: 1 }), ctx.self_ref());
    }

    // Handler for Pong message
    fn on_pong(&mut self, msg: &Pong, ctx: &mut ActorContext) {
        println!("PingActor: Received pong {}", msg.count);
        if msg.count >= self.max_count {
            println!("PingActor: Done!");
            self.manager_handle.terminate();
        } else {
            self.pong_actor
                .send(Box::new(Ping { count: msg.count + 1 }), ctx.self_ref());
        }
    }
}

// Register message handlers for PingActor
handle_messages!(PingActor,
    Start => on_start,
    Pong => on_pong
);

// Actor that receives Ping and sends Pong
struct PongActor;

impl PongActor {
    // Handler for Ping message
    fn on_ping(&mut self, msg: &Ping, ctx: &mut ActorContext) {
        println!("PongActor: Received ping {}, sending pong", msg.count);
        ctx.reply(Box::new(Pong { count: msg.count }));
    }
}

// Register message handlers for PongActor
handle_messages!(PongActor,
    Ping => on_ping
);

fn main() {
    println!("=== Ping-Pong Actor Example ===");

    let mut mgr = Manager::new();
    let handle = mgr.get_handle();

    // Create actors
    let pong_ref = mgr.manage("PongActor", Box::new(PongActor), ThreadConfig::default());
    let ping_actor = PingActor::new(pong_ref, handle, 5);
    let _ping_ref = mgr.manage("PingActor", Box::new(ping_actor), ThreadConfig::default());

    // Start all actors
    mgr.init();

    // Wait for an actor to call manager_handle.terminate()
    mgr.run();

    // Shutdown all actors
    mgr.end();
}
