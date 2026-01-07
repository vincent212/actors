"""Tests for Manager and ManagerHandle."""

import pytest
import time
from queue import Queue
from actors.actor import Actor, Envelope
from actors.manager import Manager, ManagerHandle
from actors.messages import Start, Shutdown


class TestManagerHandle:
    """Tests for ManagerHandle."""

    def test_initial_state_not_terminated(self):
        """Handle starts in non-terminated state."""
        handle = ManagerHandle()
        assert handle.is_terminated() is False

    def test_terminate_sets_flag(self):
        """terminate() sets the terminated flag."""
        handle = ManagerHandle()
        handle.terminate()
        assert handle.is_terminated() is True


class TestManager:
    """Tests for Manager."""

    def test_manage_returns_actor_ref(self):
        """manage() returns a LocalActorRef for the actor."""
        mgr = Manager()
        actor = Actor()
        ref = mgr.manage("test", actor)

        assert ref.name == "test"
        assert actor._actor_ref is ref

    def test_manage_sets_actor_queue(self):
        """manage() sets up the actor's queue."""
        mgr = Manager()
        actor = Actor()
        mgr.manage("test", actor)

        assert actor._queue is not None
        assert isinstance(actor._queue, Queue)

    def test_get_ref_returns_managed_actor(self):
        """get_ref() returns the actor's reference."""
        mgr = Manager()
        actor = Actor()
        ref = mgr.manage("test", actor)

        result = mgr.get_ref("test")
        assert result is ref

    def test_get_ref_returns_none_for_unknown(self):
        """get_ref() returns None for unknown actor names."""
        mgr = Manager()
        assert mgr.get_ref("unknown") is None

    def test_get_endpoint(self):
        """get_endpoint() returns the configured endpoint."""
        mgr = Manager(endpoint="tcp://localhost:5001")
        assert mgr.get_endpoint() == "tcp://localhost:5001"

    def test_get_endpoint_none_by_default(self):
        """get_endpoint() returns None if not configured."""
        mgr = Manager()
        assert mgr.get_endpoint() is None

    def test_get_handle(self):
        """get_handle() returns a ManagerHandle."""
        mgr = Manager()
        handle = mgr.get_handle()
        assert isinstance(handle, ManagerHandle)


class TestManagerLifecycle:
    """Tests for Manager lifecycle (init/run/end)."""

    def test_init_sends_start_message(self):
        """init() sends Start message to all actors."""
        received_messages = []

        class TestActor(Actor):
            def on_start(self, env):
                received_messages.append(env.msg)
            def run(self):
                # Override to not loop - just process one message
                self.init()
                env = self._queue.get(timeout=1.0)
                self.process_message(env)

        mgr = Manager()
        actor = TestActor()
        mgr.manage("test", actor)
        mgr.init()

        # Give thread time to process
        time.sleep(0.2)
        mgr.end()

        assert len(received_messages) == 1
        assert isinstance(received_messages[0], Start)

    def test_end_sends_shutdown_message(self):
        """end() sends Shutdown message and stops actors."""
        mgr = Manager()
        actor = Actor()
        mgr.manage("test", actor)
        mgr.init()

        # Actor should be running
        assert actor._running is True

        mgr.end()

        # Actor should be stopped
        assert actor._running is False
