"""
RegistryClient - Client for communicating with GlobalRegistry.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import json
import threading
import time
from typing import Optional, Tuple

import zmq

from .registry_messages import (
    RegisterActor, LookupActor, Heartbeat
)


class RegistryError(Exception):
    """Base exception for registry operations."""
    pass


class ActorNotFoundError(RegistryError):
    """Actor name not found in registry."""
    def __init__(self, actor_name: str):
        super().__init__(f"Actor not found: {actor_name}")
        self.actor_name = actor_name


class ActorOfflineError(RegistryError):
    """Actor's manager is offline (missed heartbeats)."""
    def __init__(self, actor_name: str):
        super().__init__(f"Actor offline: {actor_name}")
        self.actor_name = actor_name


class RegistrationFailedError(RegistryError):
    """Registration was rejected by the registry."""
    def __init__(self, actor_name: str, reason: str):
        super().__init__(f"Registration failed for '{actor_name}': {reason}")
        self.actor_name = actor_name
        self.reason = reason


class TimeoutError(RegistryError):
    """Operation timed out."""
    pass


class RegistryClient:
    """Client for communicating with the GlobalRegistry.

    The RegistryClient:
    - Sends heartbeats every 2 seconds in a background thread
    - Provides sync lookup for actors by name
    - Handles registration of local actors

    Example:
        client = RegistryClient("MyManager", "tcp://localhost:5555")
        client.start_heartbeat()

        # Register an actor
        client.register("MyActor", "tcp://localhost:5001")

        # Lookup a remote actor
        endpoint = client.lookup("OtherActor")

        client.stop_heartbeat()
    """

    HEARTBEAT_INTERVAL_S = 2.0

    def __init__(self, manager_id: str, registry_endpoint: str):
        """Create a new registry client.

        Args:
            manager_id: Unique identifier for this manager
            registry_endpoint: ZMQ endpoint of the GlobalRegistry (e.g., "tcp://localhost:5555")
        """
        self.manager_id = manager_id
        self.registry_endpoint = registry_endpoint

        self._context = zmq.Context.instance()
        self._socket: Optional[zmq.Socket] = None
        self._socket_lock = threading.Lock()

        self._heartbeat_thread: Optional[threading.Thread] = None
        self._running = False

    def _get_socket(self) -> zmq.Socket:
        """Get or create the ZMQ REQ socket."""
        if self._socket is None:
            self._socket = self._context.socket(zmq.REQ)
            self._socket.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
            self._socket.connect(self.registry_endpoint)
        return self._socket

    def _send_recv(self, msg: dict) -> dict:
        """Send a message and receive a reply."""
        with self._socket_lock:
            socket = self._get_socket()
            socket.send_json(msg)
            return socket.recv_json()

    def start_heartbeat(self) -> None:
        """Start the heartbeat background thread."""
        if self._running:
            return

        self._running = True
        self._heartbeat_thread = threading.Thread(
            target=self._heartbeat_loop,
            daemon=True,
            name=f"heartbeat-{self.manager_id}"
        )
        self._heartbeat_thread.start()

    def stop_heartbeat(self) -> None:
        """Stop the heartbeat background thread."""
        self._running = False
        if self._heartbeat_thread:
            self._heartbeat_thread.join(timeout=3.0)
            self._heartbeat_thread = None

    def _heartbeat_loop(self) -> None:
        """Background thread that sends heartbeats."""
        while self._running:
            try:
                hb = Heartbeat(manager_id=self.manager_id)
                self._send_recv(hb.to_dict())
            except Exception as e:
                # Log but don't crash on heartbeat failures
                pass

            time.sleep(self.HEARTBEAT_INTERVAL_S)

    def register(self, actor_name: str, endpoint: str) -> None:
        """Register an actor with the GlobalRegistry.

        Args:
            actor_name: Unique name for the actor
            endpoint: ZMQ endpoint where the actor can be reached

        Raises:
            RegistrationFailedError: If registration was rejected
            TimeoutError: If no response from registry
        """
        msg = RegisterActor(
            manager_id=self.manager_id,
            actor_name=actor_name,
            actor_endpoint=endpoint
        )

        try:
            reply = self._send_recv(msg.to_dict())
        except zmq.Again:
            raise TimeoutError("No response from registry for registration")

        if reply.get('message_type') == 'RegistrationOk':
            return
        elif reply.get('message_type') == 'RegistrationFailed':
            raise RegistrationFailedError(
                reply.get('actor_name', actor_name),
                reply.get('reason', 'Unknown')
            )
        else:
            raise RegistryError(f"Unexpected response: {reply}")

    def lookup(self, actor_name: str) -> str:
        """Lookup an actor by name.

        Args:
            actor_name: Name of the actor to find

        Returns:
            The ZMQ endpoint where the actor can be reached

        Raises:
            ActorNotFoundError: If actor not registered
            ActorOfflineError: If actor's manager missed heartbeats
            TimeoutError: If no response from registry
        """
        msg = LookupActor(actor_name=actor_name)

        try:
            reply = self._send_recv(msg.to_dict())
        except zmq.Again:
            raise TimeoutError("No response from registry for lookup")

        if reply.get('message_type') == 'LookupResult':
            endpoint = reply.get('endpoint')
            online = reply.get('online', False)

            if endpoint is None:
                raise ActorNotFoundError(actor_name)
            if not online:
                raise ActorOfflineError(actor_name)
            return endpoint
        else:
            raise RegistryError(f"Unexpected response: {reply}")

    def lookup_allow_offline(self, actor_name: str) -> Tuple[str, bool]:
        """Lookup an actor, returning the endpoint even if offline.

        Use this when you want to attempt communication with a potentially
        recovering actor.

        Args:
            actor_name: Name of the actor to find

        Returns:
            Tuple of (endpoint, is_online)

        Raises:
            ActorNotFoundError: If actor not registered
            TimeoutError: If no response from registry
        """
        msg = LookupActor(actor_name=actor_name)

        try:
            reply = self._send_recv(msg.to_dict())
        except zmq.Again:
            raise TimeoutError("No response from registry for lookup")

        if reply.get('message_type') == 'LookupResult':
            endpoint = reply.get('endpoint')
            online = reply.get('online', False)

            if endpoint is None:
                raise ActorNotFoundError(actor_name)
            return (endpoint, online)
        else:
            raise RegistryError(f"Unexpected response: {reply}")

    def close(self) -> None:
        """Close the registry client and stop heartbeats."""
        self.stop_heartbeat()
        with self._socket_lock:
            if self._socket:
                self._socket.close()
                self._socket = None
