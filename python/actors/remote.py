"""
Remote actor communication via ZeroMQ.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import time
from queue import Queue, Empty
from typing import Any, Dict, Optional

import zmq

from .actor import Actor, ActorRef, Envelope
from .messages import Reject
from .serialization import serialize_message, deserialize_message


class RemoteActorRef(ActorRef):
    """ActorRef for actors in other processes - uses ZMQ."""

    def __init__(self, name: str, endpoint: str, zmq_sender: 'ZmqSender'):
        self._name = name
        self._endpoint = endpoint
        self._zmq_sender = zmq_sender

    @property
    def name(self) -> str:
        return self._name

    @property
    def endpoint(self) -> str:
        return self._endpoint

    def send(self, msg: Any, sender: Optional[ActorRef] = None) -> None:
        """Send a message to remote actor."""
        self._zmq_sender.send_to(self._endpoint, self._name, msg, sender)

    def fast_send(self, msg: Any, sender: Optional[ActorRef] = None) -> Any:
        """Not supported for remote actors."""
        raise NotImplementedError("fast_send not supported for remote actors")


class ZmqSender:
    """Sends messages to remote processes via ZMQ PUSH sockets."""

    def __init__(self, context: Optional[zmq.Context] = None, local_endpoint: Optional[str] = None):
        self._context = context or zmq.Context.instance()
        self._sockets: Dict[str, zmq.Socket] = {}
        self._local_endpoint = local_endpoint  # This process's endpoint for replies

    def set_local_endpoint(self, endpoint: str) -> None:
        """Set the local endpoint for reply routing."""
        self._local_endpoint = endpoint

    def _get_socket(self, endpoint: str) -> zmq.Socket:
        """Get or create a PUSH socket for the given endpoint."""
        if endpoint not in self._sockets:
            socket = self._context.socket(zmq.PUSH)
            socket.connect(endpoint)
            self._sockets[endpoint] = socket
        return self._sockets[endpoint]

    def send_to(
        self,
        endpoint: str,
        actor_name: str,
        msg: Any,
        sender: Optional[ActorRef]
    ) -> None:
        """Send a message to a remote actor."""
        # Determine sender info for reply routing
        sender_actor = sender.name if sender else None

        # Get sender endpoint - could be local or remote
        if sender and isinstance(sender, RemoteActorRef):
            sender_endpoint = sender.endpoint
        else:
            sender_endpoint = self._local_endpoint

        socket = self._get_socket(endpoint)
        data = serialize_message(actor_name, msg, sender_actor, sender_endpoint)
        socket.send_json(data)

    def close(self) -> None:
        """Close all sockets."""
        for socket in self._sockets.values():
            socket.close()
        self._sockets.clear()


class ZmqReceiver(Actor):
    """Receives messages from remote processes and routes to local actors."""

    def __init__(self, bind_endpoint: str, manager: 'Manager', zmq_sender: ZmqSender):
        self._bind_endpoint = bind_endpoint
        self._manager = manager
        self._zmq_sender = zmq_sender
        self._zmq_socket: Optional[zmq.Socket] = None

    def init(self) -> None:
        """Bind ZMQ socket."""
        context = zmq.Context.instance()
        self._zmq_socket = context.socket(zmq.PULL)
        self._zmq_socket.bind(self._bind_endpoint)

    def run(self) -> None:
        """Override: poll both ZMQ and local queue."""
        self.init()
        while self._running:
            # Check ZMQ (non-blocking)
            try:
                data = self._zmq_socket.recv_json(flags=zmq.NOBLOCK)
                self._handle_remote_message(data)
            except zmq.Again:
                pass

            # Check local queue for Shutdown message
            try:
                envelope = self._queue.get_nowait()
                self.process_message(envelope)
            except Empty:
                pass

            time.sleep(0.001)  # Small sleep to avoid busy-waiting

        self.end()

    def end(self) -> None:
        """Close ZMQ socket."""
        if self._zmq_socket:
            self._zmq_socket.close()

    def _handle_remote_message(self, data: dict) -> None:
        """Route incoming remote message to local actor."""
        receiver_name = data["receiver"]
        msg_type = data.get("message_type", "")

        # Create RemoteActorRef for the sender (so replies/rejects go back)
        sender_ref = None
        if data.get("sender_actor") and data.get("sender_endpoint"):
            sender_ref = RemoteActorRef(
                name=data["sender_actor"],
                endpoint=data["sender_endpoint"],
                zmq_sender=self._zmq_sender
            )

        # Look up local actor
        local_ref = self._manager.get_ref(receiver_name)
        if not local_ref:
            # Actor not found - send reject back to sender
            if sender_ref:
                reject = Reject(
                    message_type=msg_type,
                    reason=f"Actor '{receiver_name}' not found",
                    rejected_by=receiver_name
                )
                sender_ref.send(reject, sender=None)
            return

        # Try to deserialize and deliver
        try:
            msg = deserialize_message(msg_type, data["message"])
            local_ref.send(msg, sender=sender_ref)
        except ValueError as e:
            # Deserialization failed - send reject back to sender
            if sender_ref:
                reject = Reject(
                    message_type=msg_type,
                    reason=str(e),
                    rejected_by=receiver_name
                )
                sender_ref.send(reject, sender=None)
