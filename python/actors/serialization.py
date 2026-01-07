"""
Message serialization for remote communication.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

from typing import Any, Dict, Optional, Type

# Registry of message types for deserialization
MESSAGE_REGISTRY: Dict[str, Type] = {}


def register_message(cls: Type) -> Type:
    """Decorator to register a message type for serialization."""
    MESSAGE_REGISTRY[cls.__name__] = cls
    return cls


def serialize_message(
    receiver: str,
    msg: Any,
    sender_actor: Optional[str],
    sender_endpoint: Optional[str]
) -> dict:
    """Serialize a message for remote transmission."""
    return {
        "sender_actor": sender_actor,
        "sender_endpoint": sender_endpoint,
        "receiver": receiver,
        "message_type": type(msg).__name__,
        "message": msg.__dict__ if hasattr(msg, '__dict__') else {}
    }


def deserialize_message(msg_type: str, data: dict) -> Any:
    """Deserialize a message from remote transmission."""
    cls = MESSAGE_REGISTRY.get(msg_type)
    if cls:
        return cls(**data)
    raise ValueError(f"Unknown message type: {msg_type}. Did you register it with @register_message?")
