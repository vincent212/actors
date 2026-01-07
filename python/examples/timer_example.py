#!/usr/bin/env python3
"""
Timer Example

Demonstrates how to use timers in the actor framework:
- Periodic timers that fire repeatedly at fixed intervals
- One-shot timers that fire once after a delay
- Timer cancellation

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
sys.path.insert(0, '/home/vm/actors-py')

from actors import (
    Actor, Envelope, Manager, Start,
    Timer, Timeout, next_timer_id
)


class TimerActor(Actor):
    """An actor that demonstrates timer usage."""

    def __init__(self, manager_handle):
        self.manager_handle = manager_handle
        self.tick_count = 0
        self.max_ticks = 10

        # Generate unique timer IDs
        self.periodic_timer_id = next_timer_id()
        self.countdown_timer_id = next_timer_id()

        # Timer references (to allow cancellation)
        self.periodic_timer = None
        self.countdown_timer = None

    def on_start(self, env: Envelope) -> None:
        print("TimerActor: Starting timers...")
        print(f"  - Periodic timer (id={self.periodic_timer_id}): every 500ms")
        print(f"  - One-shot timer (id={self.countdown_timer_id}): fires after 2.5s")
        print()

        # Start a periodic timer that fires every 500ms
        self.periodic_timer = Timer.periodic(
            self._actor_ref,
            interval=0.5,  # 500ms
            timer_id=self.periodic_timer_id
        )

        # Start a one-shot timer that fires after 2.5 seconds
        self.countdown_timer = Timer.once(
            self._actor_ref,
            delay=2.5,  # 2500ms
            timer_id=self.countdown_timer_id
        )

    def on_timeout(self, env: Envelope) -> None:
        timer_id = env.msg.id

        if timer_id == self.periodic_timer_id:
            self.tick_count += 1
            print(f"TimerActor: Periodic tick #{self.tick_count}")

            if self.tick_count >= self.max_ticks:
                print("TimerActor: Max ticks reached, cancelling periodic timer")
                if self.periodic_timer:
                    self.periodic_timer.cancel()
                    self.periodic_timer = None
                # Terminate after periodic timer is done
                print("TimerActor: Shutting down...")
                self.manager_handle.terminate()

        elif timer_id == self.countdown_timer_id:
            print("TimerActor: *** COUNTDOWN COMPLETE! One-shot timer fired! ***")
            self.countdown_timer = None

        else:
            print(f"TimerActor: Unknown timer id={timer_id}")


def main():
    print("=== Timer Example ===")
    print()

    # Create manager
    mgr = Manager()
    handle = mgr.get_handle()

    # Create timer actor
    timer_actor = TimerActor(handle)
    mgr.manage("timer_actor", timer_actor)

    # Start all actors
    mgr.init()

    print("Running... (will stop after 10 ticks)")
    print()

    # Run until terminated
    mgr.run()
    mgr.end()

    print()
    print("=== Timer Example Complete ===")


if __name__ == "__main__":
    main()
