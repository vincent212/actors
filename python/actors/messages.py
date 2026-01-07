"""
Built-in messages.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""


class Start:
    """Sent when actor starts."""
    pass


class Shutdown:
    """Sent when actor is shutting down."""
    pass


class Reject:
    """Sent when a remote message cannot be processed.

    This is sent back to the sender when:
    - The message type is not registered
    - The target actor is not found
    - Deserialization fails
    """
    def __init__(self, message_type: str, reason: str, rejected_by: str):
        self.message_type = message_type
        self.reason = reason
        self.rejected_by = rejected_by

    def __repr__(self):
        return f"Reject(message_type={self.message_type!r}, reason={self.reason!r}, rejected_by={self.rejected_by!r})"
