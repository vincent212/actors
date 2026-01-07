"""Tests for GlobalRegistry (without ZMQ)."""

import pytest
import time
from unittest.mock import patch
from actors.registry import GlobalRegistry, ActorEntry


class TestGlobalRegistryState:
    """Tests for GlobalRegistry internal state management."""

    def test_initial_state_empty(self):
        """Registry starts with no actors registered."""
        registry = GlobalRegistry()
        assert registry.get_all_actors() == []
        assert registry.get_all_managers() == []

    def test_lookup_returns_none_for_unknown(self):
        """lookup() returns None for unregistered actors."""
        registry = GlobalRegistry()
        assert registry.lookup("unknown") is None

    def test_add_actor_manually(self):
        """Actors can be added to registry directly."""
        registry = GlobalRegistry()
        registry._registry["pong"] = ActorEntry(
            endpoint="tcp://localhost:5001",
            manager_id="mgr1"
        )
        registry._manager_actors["mgr1"] = {"pong"}
        registry._heartbeats["mgr1"] = time.monotonic()

        assert "pong" in registry.get_all_actors()
        assert registry.lookup("pong") == "tcp://localhost:5001"

    def test_is_manager_online_no_heartbeat(self):
        """is_manager_online() returns False if no heartbeat received."""
        registry = GlobalRegistry()
        assert registry.is_manager_online("unknown") is False

    def test_is_manager_online_recent_heartbeat(self):
        """is_manager_online() returns True for recent heartbeats."""
        registry = GlobalRegistry()
        registry._heartbeats["mgr1"] = time.monotonic()
        assert registry.is_manager_online("mgr1") is True

    def test_is_manager_online_stale_heartbeat(self):
        """is_manager_online() returns False for stale heartbeats."""
        registry = GlobalRegistry()
        # Heartbeat from 10 seconds ago (> 6s timeout)
        registry._heartbeats["mgr1"] = time.monotonic() - 10
        assert registry.is_manager_online("mgr1") is False


class TestGlobalRegistryUnregister:
    """Tests for unregistering actors on timeout."""

    def test_unregister_manager_removes_actors(self):
        """_unregister_manager removes all actors for that manager."""
        registry = GlobalRegistry()

        # Register two actors for mgr1
        registry._registry["actor1"] = ActorEntry("tcp://host:5001", "mgr1")
        registry._registry["actor2"] = ActorEntry("tcp://host:5002", "mgr1")
        registry._manager_actors["mgr1"] = {"actor1", "actor2"}
        registry._heartbeats["mgr1"] = time.monotonic()

        # Unregister mgr1
        registry._unregister_manager("mgr1")

        assert "actor1" not in registry._registry
        assert "actor2" not in registry._registry
        assert "mgr1" not in registry._manager_actors
        assert "mgr1" not in registry._heartbeats

    def test_unregister_manager_preserves_other_managers(self):
        """_unregister_manager only affects the specified manager."""
        registry = GlobalRegistry()

        # Register actors for two managers
        registry._registry["actor1"] = ActorEntry("tcp://host:5001", "mgr1")
        registry._registry["actor2"] = ActorEntry("tcp://host:5002", "mgr2")
        registry._manager_actors["mgr1"] = {"actor1"}
        registry._manager_actors["mgr2"] = {"actor2"}
        registry._heartbeats["mgr1"] = time.monotonic()
        registry._heartbeats["mgr2"] = time.monotonic()

        # Unregister mgr1
        registry._unregister_manager("mgr1")

        # mgr2's actor should still be there
        assert "actor2" in registry._registry
        assert "mgr2" in registry._manager_actors


class TestHeartbeatTimeout:
    """Tests for heartbeat timeout detection."""

    def test_check_heartbeats_removes_stale_managers(self):
        """_check_heartbeats unregisters managers that timed out."""
        registry = GlobalRegistry()

        # Register an actor with stale heartbeat
        registry._registry["actor1"] = ActorEntry("tcp://host:5001", "mgr1")
        registry._manager_actors["mgr1"] = {"actor1"}
        registry._heartbeats["mgr1"] = time.monotonic() - 10  # 10s ago

        registry._check_heartbeats()

        assert "actor1" not in registry._registry
        assert "mgr1" not in registry._heartbeats

    def test_check_heartbeats_preserves_healthy_managers(self):
        """_check_heartbeats keeps managers with recent heartbeats."""
        registry = GlobalRegistry()

        # Register an actor with recent heartbeat
        registry._registry["actor1"] = ActorEntry("tcp://host:5001", "mgr1")
        registry._manager_actors["mgr1"] = {"actor1"}
        registry._heartbeats["mgr1"] = time.monotonic()

        registry._check_heartbeats()

        assert "actor1" in registry._registry
        assert "mgr1" in registry._heartbeats


class TestGlobalRegistryLifecycle:
    """Tests for GlobalRegistry init/end lifecycle."""

    def test_init_starts_monitor_thread(self):
        """init() starts the heartbeat monitor thread."""
        registry = GlobalRegistry()
        registry.init()

        assert registry._running is True
        assert registry._monitor_thread is not None
        assert registry._monitor_thread.is_alive()

        registry.end()

    def test_end_stops_monitor_thread(self):
        """end() stops the heartbeat monitor thread."""
        registry = GlobalRegistry()
        registry.init()
        registry.end()

        assert registry._running is False
