"""
Timer utilities for scheduling delayed and periodic messages.

Timers send Timeout messages to actors after a specified delay.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import threading
from typing import Optional
from dataclasses import dataclass

from .actor import ActorRef


@dataclass
class Timeout:
    """Message sent when a timer fires."""
    id: int


# Global timer ID counter
_next_timer_id = 0
_timer_id_lock = threading.Lock()


def next_timer_id() -> int:
    """Generate a unique timer ID."""
    global _next_timer_id
    with _timer_id_lock:
        _next_timer_id += 1
        return _next_timer_id


class Timer:
    """A timer that sends Timeout messages to an actor.

    Example:
        # One-shot timer (fires once after 5 seconds)
        timer = Timer.once(actor_ref, 5.0, timer_id)

        # Periodic timer (fires every 0.5 seconds)
        timer = Timer.periodic(actor_ref, 0.5, timer_id)

        # Cancel when done
        timer.cancel()
    """

    def __init__(self, actor: ActorRef, interval: float, timer_id: int, periodic: bool):
        """Internal constructor. Use Timer.once() or Timer.periodic() instead."""
        self._actor = actor
        self._interval = interval
        self._timer_id = timer_id
        self._periodic = periodic
        self._running = True
        self._thread: Optional[threading.Thread] = None

    @classmethod
    def once(cls, actor: ActorRef, delay: float, timer_id: int) -> 'Timer':
        """Create a one-shot timer.

        Sends a Timeout message to the actor after the specified delay.

        Args:
            actor: ActorRef to send Timeout message to
            delay: Delay in seconds before firing
            timer_id: Unique ID to identify this timer in Timeout message

        Returns:
            Timer instance that can be cancelled
        """
        timer = cls(actor, delay, timer_id, periodic=False)
        timer._start()
        return timer

    @classmethod
    def periodic(cls, actor: ActorRef, interval: float, timer_id: int) -> 'Timer':
        """Create a periodic timer.

        Sends Timeout messages to the actor at the specified interval.
        The timer runs until cancel() is called.

        Args:
            actor: ActorRef to send Timeout messages to
            interval: Interval in seconds between firings
            timer_id: Unique ID to identify this timer in Timeout messages

        Returns:
            Timer instance that can be cancelled
        """
        timer = cls(actor, interval, timer_id, periodic=True)
        timer._start()
        return timer

    def _start(self) -> None:
        """Start the timer thread."""
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def _run(self) -> None:
        """Timer thread main loop."""
        import time

        if self._periodic:
            # Periodic timer
            while self._running:
                time.sleep(self._interval)
                if self._running:
                    self._actor.send(Timeout(id=self._timer_id))
        else:
            # One-shot timer
            time.sleep(self._interval)
            if self._running:
                self._actor.send(Timeout(id=self._timer_id))

    def cancel(self) -> None:
        """Cancel the timer.

        No more Timeout messages will be sent after this call.
        """
        self._running = False

    def is_running(self) -> bool:
        """Check if the timer is still running."""
        return self._running
