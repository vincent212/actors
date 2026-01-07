"""Tests for registry message dataclasses."""

import pytest
import time
from actors.registry_messages import (
    RegisterActor, UnregisterActor, RegistrationOk, RegistrationFailed,
    LookupActor, LookupResult, Heartbeat, HeartbeatAck
)


class TestRegisterActor:
    """Tests for RegisterActor message."""

    def test_to_dict_includes_message_type(self):
        """to_dict() includes message_type field."""
        msg = RegisterActor(
            manager_id="mgr1",
            actor_name="pong",
            actor_endpoint="tcp://localhost:5001"
        )
        result = msg.to_dict()

        assert result["message_type"] == "RegisterActor"
        assert result["manager_id"] == "mgr1"
        assert result["actor_name"] == "pong"
        assert result["actor_endpoint"] == "tcp://localhost:5001"


class TestUnregisterActor:
    """Tests for UnregisterActor message."""

    def test_to_dict(self):
        msg = UnregisterActor(actor_name="pong")
        result = msg.to_dict()

        assert result["message_type"] == "UnregisterActor"
        assert result["actor_name"] == "pong"


class TestRegistrationOk:
    """Tests for RegistrationOk message."""

    def test_to_dict(self):
        msg = RegistrationOk(actor_name="pong")
        result = msg.to_dict()

        assert result["message_type"] == "RegistrationOk"
        assert result["actor_name"] == "pong"


class TestRegistrationFailed:
    """Tests for RegistrationFailed message."""

    def test_to_dict(self):
        msg = RegistrationFailed(actor_name="pong", reason="Name already registered")
        result = msg.to_dict()

        assert result["message_type"] == "RegistrationFailed"
        assert result["actor_name"] == "pong"
        assert result["reason"] == "Name already registered"


class TestLookupActor:
    """Tests for LookupActor message."""

    def test_to_dict(self):
        msg = LookupActor(actor_name="pong")
        result = msg.to_dict()

        assert result["message_type"] == "LookupActor"
        assert result["actor_name"] == "pong"


class TestLookupResult:
    """Tests for LookupResult message."""

    def test_to_dict_found_online(self):
        msg = LookupResult(
            actor_name="pong",
            endpoint="tcp://localhost:5001",
            online=True
        )
        result = msg.to_dict()

        assert result["message_type"] == "LookupResult"
        assert result["actor_name"] == "pong"
        assert result["endpoint"] == "tcp://localhost:5001"
        assert result["online"] is True

    def test_to_dict_not_found(self):
        msg = LookupResult(actor_name="unknown", endpoint=None, online=False)
        result = msg.to_dict()

        assert result["endpoint"] is None
        assert result["online"] is False


class TestHeartbeat:
    """Tests for Heartbeat message."""

    def test_to_dict_includes_timestamp(self):
        msg = Heartbeat(manager_id="mgr1")
        result = msg.to_dict()

        assert result["message_type"] == "Heartbeat"
        assert result["manager_id"] == "mgr1"
        assert "timestamp_ms" in result
        # Timestamp should be recent (within last second)
        now_ms = int(time.time() * 1000)
        assert abs(result["timestamp_ms"] - now_ms) < 1000

    def test_custom_timestamp(self):
        msg = Heartbeat(manager_id="mgr1", timestamp_ms=12345)
        result = msg.to_dict()

        assert result["timestamp_ms"] == 12345


class TestHeartbeatAck:
    """Tests for HeartbeatAck message."""

    def test_to_dict(self):
        msg = HeartbeatAck()
        result = msg.to_dict()

        assert result["message_type"] == "HeartbeatAck"
