/*
Timer Example

Demonstrates using Timer for periodic and one-shot callbacks.

Usage:
    cargo run --example timer_example

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
*/

use std::time::Duration;

use actors::{
    ActorContext, Manager, ManagerHandle, Start, ThreadConfig, Timeout, Timer,
    define_message, handle_messages, next_timer_id,
};

// Custom message to trigger a one-shot timer
struct StartCountdown;
define_message!(StartCountdown);

/// An actor that demonstrates both periodic and one-shot timers.
struct TimerActor {
    manager_handle: ManagerHandle,
    periodic_timer: Option<Timer>,
    countdown_timer: Option<Timer>,
    tick_count: u32,
    periodic_timer_id: u64,
    countdown_timer_id: u64,
}

handle_messages!(TimerActor,
    Start => on_start,
    Timeout => on_timeout,
    StartCountdown => on_start_countdown
);

impl TimerActor {
    fn new(manager_handle: ManagerHandle) -> Self {
        TimerActor {
            manager_handle,
            periodic_timer: None,
            countdown_timer: None,
            tick_count: 0,
            periodic_timer_id: next_timer_id(),
            countdown_timer_id: next_timer_id(),
        }
    }

    fn on_start(&mut self, _msg: &Start, ctx: &mut ActorContext) {
        println!("TimerActor: Starting...");
        println!("TimerActor: Setting up periodic timer (every 500ms)");

        // Create a periodic timer that fires every 500ms
        self.periodic_timer = Some(Timer::periodic(
            ctx.self_ref().unwrap(),
            Duration::from_millis(500),
            self.periodic_timer_id,
        ));

        // Also schedule a one-shot countdown timer
        println!("TimerActor: Setting up one-shot timer (fires in 2.5 seconds)");
        self.countdown_timer = Some(Timer::once(
            ctx.self_ref().unwrap(),
            Duration::from_millis(2500),
            self.countdown_timer_id,
        ));
    }

    fn on_timeout(&mut self, msg: &Timeout, _ctx: &mut ActorContext) {
        if msg.id == self.periodic_timer_id {
            // Periodic timer fired
            self.tick_count += 1;
            println!("TimerActor: Periodic tick #{}", self.tick_count);

            // Stop after 10 ticks
            if self.tick_count >= 10 {
                println!("TimerActor: 10 ticks reached, canceling periodic timer");
                if let Some(ref timer) = self.periodic_timer {
                    timer.cancel();
                }
                self.periodic_timer = None;

                // Check if countdown is also done
                if self.countdown_timer.is_none() {
                    println!("TimerActor: All timers done, shutting down");
                    self.manager_handle.terminate();
                }
            }
        } else if msg.id == self.countdown_timer_id {
            // One-shot countdown timer fired
            println!("TimerActor: *** ONE-SHOT TIMER FIRED! ***");
            self.countdown_timer = None;

            // Check if periodic is also done
            if self.periodic_timer.is_none() {
                println!("TimerActor: All timers done, shutting down");
                self.manager_handle.terminate();
            }
        } else {
            println!("TimerActor: Unknown timer ID: {}", msg.id);
        }
    }

    fn on_start_countdown(&mut self, _msg: &StartCountdown, ctx: &mut ActorContext) {
        // Example of starting a timer from a message handler
        println!("TimerActor: Starting new countdown timer (3 seconds)...");
        let timer_id = next_timer_id();
        let _timer = Timer::once(
            ctx.self_ref().unwrap(),
            Duration::from_secs(3),
            timer_id,
        );
        // Note: Timer is dropped immediately, so the callback will still fire
        // but we lose the ability to cancel it. To keep control, store it.
    }
}

fn main() {
    println!("=== Timer Example ===");
    println!();

    let mut mgr = Manager::new();
    let handle = mgr.get_handle();

    let _timer_actor_ref = mgr.manage(
        "timer_actor",
        Box::new(TimerActor::new(handle)),
        ThreadConfig::default(),
    );

    mgr.init();
    println!("Timer example running...\n");

    mgr.run();
    mgr.end();

    println!("\n=== Timer Example Complete ===");
}
