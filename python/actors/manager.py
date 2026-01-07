"""
Manager and ManagerHandle for actor lifecycle management.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import time
from queue import Queue
from threading import Thread
from typing import Dict, Optional

from .actor import Actor, LocalActorRef
from .messages import Start, Shutdown


class ManagerHandle:
    """Handle for actors to signal termination."""

    def __init__(self):
        self._terminated = False

    def terminate(self) -> None:
        """Signal the manager to terminate."""
        self._terminated = True

    def is_terminated(self) -> bool:
        """Check if termination was signaled."""
        return self._terminated


class Manager:
    """Manages actor lifecycle and provides a registry."""

    def __init__(self, endpoint: Optional[str] = None):
        self._actors: Dict[str, Actor] = {}
        self._threads: Dict[str, Thread] = {}
        self._handle = ManagerHandle()
        self._endpoint = endpoint  # This process's ZMQ endpoint (for remote)

    def get_handle(self) -> ManagerHandle:
        """Get handle for signaling termination."""
        return self._handle

    def get_endpoint(self) -> Optional[str]:
        """Get this process's ZMQ endpoint."""
        return self._endpoint

    def manage(self, name: str, actor: Actor) -> LocalActorRef:
        """Register an actor, returns its LocalActorRef."""
        queue = Queue()
        actor_ref = LocalActorRef(queue, name)
        actor._actor_ref = actor_ref
        actor._queue = queue
        self._actors[name] = actor
        return actor_ref

    def get_ref(self, name: str) -> Optional[LocalActorRef]:
        """Look up actor by name."""
        actor = self._actors.get(name)
        if actor:
            return actor._actor_ref
        return None

    def init(self) -> None:
        """Start all actor threads and send Start message."""
        for name, actor in self._actors.items():
            thread = Thread(target=actor.run, name=f"actor-{name}", daemon=True)
            thread.start()
            self._threads[name] = thread
            actor._actor_ref.send(Start())

    def run(self) -> None:
        """Wait until terminated."""
        while not self._handle.is_terminated():
            time.sleep(0.1)

    def end(self) -> None:
        """Stop all actors and wait for threads."""
        for name, actor in self._actors.items():
            actor._actor_ref.send(Shutdown())
            actor.stop()
        for thread in self._threads.values():
            thread.join(timeout=1.0)
