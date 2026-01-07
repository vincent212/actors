"""
Registry protocol messages.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

from dataclasses import dataclass, asdict
from typing import Optional
import time


@dataclass
class RegisterActor:
    """Manager registers an actor with GlobalRegistry.

    Sent during Manager.manage() to register actor name -> endpoint mapping.
    GlobalRegistry replies with RegistrationOk or RegistrationFailed.
    """
    manager_id: str
    actor_name: str
    actor_endpoint: str  # ZMQ endpoint for reaching this actor

    def to_dict(self):
        return {
            'message_type': 'RegisterActor',
            'manager_id': self.manager_id,
            'actor_name': self.actor_name,
            'actor_endpoint': self.actor_endpoint
        }


@dataclass
class UnregisterActor:
    """Remove an actor from the registry.

    Sent when an actor is stopped or Manager shuts down.
    """
    actor_name: str

    def to_dict(self):
        return {
            'message_type': 'UnregisterActor',
            'actor_name': self.actor_name
        }


@dataclass
class RegistrationOk:
    """Confirms successful actor registration."""
    actor_name: str

    def to_dict(self):
        return {
            'message_type': 'RegistrationOk',
            'actor_name': self.actor_name
        }


@dataclass
class RegistrationFailed:
    """Registration was rejected.

    Common reasons: name already registered, invalid endpoint.
    """
    actor_name: str
    reason: str

    def to_dict(self):
        return {
            'message_type': 'RegistrationFailed',
            'actor_name': self.actor_name,
            'reason': self.reason
        }


@dataclass
class LookupActor:
    """Request endpoint for a named actor.

    Manager sends this when local lookup fails.
    GlobalRegistry replies with LookupResult.
    """
    actor_name: str

    def to_dict(self):
        return {
            'message_type': 'LookupActor',
            'actor_name': self.actor_name
        }


@dataclass
class LookupResult:
    """Response to LookupActor.

    Contains the endpoint if found, and online status.
    If endpoint is None, the actor was not found.
    If online is False, the actor's Manager has missed heartbeats.
    """
    actor_name: str
    endpoint: Optional[str]
    online: bool

    def to_dict(self):
        return {
            'message_type': 'LookupResult',
            'actor_name': self.actor_name,
            'endpoint': self.endpoint,
            'online': self.online
        }


@dataclass
class Heartbeat:
    """Manager health check.

    Managers send this every 2 seconds.
    GlobalRegistry marks Manager offline after 6 seconds without heartbeat.
    """
    manager_id: str
    timestamp_ms: int = 0

    def __post_init__(self):
        if self.timestamp_ms == 0:
            self.timestamp_ms = int(time.time() * 1000)

    def to_dict(self):
        return {
            'message_type': 'Heartbeat',
            'manager_id': self.manager_id,
            'timestamp_ms': self.timestamp_ms
        }


@dataclass
class HeartbeatAck:
    """Acknowledgement of heartbeat."""

    def to_dict(self):
        return {
            'message_type': 'HeartbeatAck'
        }


# Process management messages

@dataclass
class StartManager:
    """Request to start a manager process."""
    manager_id: str

    def to_dict(self):
        return {'manager_id': self.manager_id, 'action': 'start'}


@dataclass
class StopManager:
    """Request to stop a manager process."""
    manager_id: str

    def to_dict(self):
        return {'manager_id': self.manager_id, 'action': 'stop'}


@dataclass
class RestartManager:
    """Request to restart a manager process."""
    manager_id: str

    def to_dict(self):
        return {'manager_id': self.manager_id, 'action': 'restart'}


@dataclass
class ManagerStatus:
    """Status of a manager process."""
    manager_id: str
    running: bool
    pid: Optional[int] = None
    error: Optional[str] = None

    def to_dict(self):
        return {
            'manager_id': self.manager_id,
            'running': self.running,
            'pid': self.pid,
            'error': self.error
        }
