"""
Actor, ActorRef, LocalActorRef, and Envelope classes.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from queue import Queue, Empty
from threading import Thread
from typing import Any, Optional


@dataclass
class Envelope:
    """Message wrapper with sender info for replies."""
    msg: Any
    sender: Optional['ActorRef'] = None
    reply_queue: Optional[Queue] = field(default=None, repr=False)


class ActorRef(ABC):
    """Base class for actor references (mailbox addresses)."""

    @property
    @abstractmethod
    def name(self) -> str:
        """Actor name."""
        pass

    @abstractmethod
    def send(self, msg: Any, sender: Optional['ActorRef'] = None) -> None:
        """Send a message asynchronously."""
        pass

    @abstractmethod
    def fast_send(self, msg: Any, sender: Optional['ActorRef'] = None) -> Any:
        """Send a message and wait for reply."""
        pass


class LocalActorRef(ActorRef):
    """ActorRef for actors in the same process - uses Queue."""

    def __init__(self, queue: Queue, name: str):
        self._queue = queue
        self._name = name

    @property
    def name(self) -> str:
        return self._name

    def send(self, msg: Any, sender: Optional[ActorRef] = None) -> None:
        """Send a message asynchronously."""
        self._queue.put(Envelope(msg, sender))

    def fast_send(self, msg: Any, sender: Optional[ActorRef] = None) -> Any:
        """Send a message and wait for reply."""
        reply_queue = Queue()
        self._queue.put(Envelope(msg, sender, reply_queue))
        return reply_queue.get()


class Actor:
    """Base class for all actors."""

    _actor_ref: Optional[LocalActorRef] = None
    _queue: Optional[Queue] = None
    _running: bool = True

    def init(self) -> None:
        """Called once when actor starts, before processing messages."""
        pass

    def end(self) -> None:
        """Called once when actor shuts down."""
        pass

    def run(self) -> None:
        """Main loop - runs in actor's thread."""
        self.init()
        while self._running:
            try:
                envelope = self._queue.get(timeout=0.1)
                self.process_message(envelope)
            except Empty:
                pass
        self.end()

    def process_message(self, envelope: Envelope) -> None:
        """Dispatch to on_<classname> handler via reflection."""
        handler_name = f"on_{type(envelope.msg).__name__.lower()}"
        handler = getattr(self, handler_name, None)
        if handler:
            handler(envelope)

    def reply(self, envelope: Envelope, response: Any) -> None:
        """Reply to a message - works for both local and remote."""
        if envelope.reply_queue:
            # fast_send: put reply in the reply queue
            envelope.reply_queue.put(response)
        elif envelope.sender:
            # async: send to sender's mailbox (polymorphic - works for Local or Remote)
            envelope.sender.send(response, self._actor_ref)

    def stop(self) -> None:
        """Signal the actor to stop."""
        self._running = False
