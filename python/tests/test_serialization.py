"""Tests for message serialization."""

import pytest
from actors.serialization import (
    register_message, serialize_message, deserialize_message,
    MESSAGE_REGISTRY
)


class TestRegisterMessage:
    """Tests for @register_message decorator."""

    def test_register_message_adds_to_registry(self):
        """Decorated class is added to MESSAGE_REGISTRY."""
        @register_message
        class TestMsg1:
            pass

        assert "TestMsg1" in MESSAGE_REGISTRY
        assert MESSAGE_REGISTRY["TestMsg1"] is TestMsg1

    def test_register_message_returns_class(self):
        """Decorator returns the original class unchanged."""
        @register_message
        class TestMsg2:
            def __init__(self, value):
                self.value = value

        msg = TestMsg2(42)
        assert msg.value == 42


class TestSerializeMessage:
    """Tests for serialize_message function."""

    def test_serialize_with_all_fields(self):
        """Serialization includes all required fields."""
        @register_message
        class Ping:
            def __init__(self, count=0):
                self.count = count

        msg = Ping(count=5)
        result = serialize_message(
            receiver="pong",
            msg=msg,
            sender_actor="ping",
            sender_endpoint="tcp://localhost:5001"
        )

        assert result["receiver"] == "pong"
        assert result["sender_actor"] == "ping"
        assert result["sender_endpoint"] == "tcp://localhost:5001"
        assert result["message_type"] == "Ping"
        assert result["message"]["count"] == 5

    def test_serialize_with_none_sender(self):
        """Serialization works with None sender fields."""
        @register_message
        class Pong:
            def __init__(self, count=0):
                self.count = count

        msg = Pong(count=3)
        result = serialize_message(
            receiver="ping",
            msg=msg,
            sender_actor=None,
            sender_endpoint=None
        )

        assert result["sender_actor"] is None
        assert result["sender_endpoint"] is None
        assert result["message_type"] == "Pong"


class TestDeserializeMessage:
    """Tests for deserialize_message function."""

    def test_deserialize_known_message(self):
        """Deserializing a registered message type works."""
        @register_message
        class Greeting:
            def __init__(self, text=""):
                self.text = text

        result = deserialize_message("Greeting", {"text": "hello"})
        assert isinstance(result, Greeting)
        assert result.text == "hello"

    def test_deserialize_unknown_message_raises(self):
        """Deserializing unknown message type raises ValueError."""
        with pytest.raises(ValueError, match="Unknown message type"):
            deserialize_message("UnknownMessageType", {})

    def test_deserialize_empty_data(self):
        """Deserializing with empty data uses defaults."""
        @register_message
        class DefaultMsg:
            def __init__(self, value=42):
                self.value = value

        result = deserialize_message("DefaultMsg", {})
        assert result.value == 42
