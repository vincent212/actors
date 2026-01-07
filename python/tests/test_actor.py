"""Tests for Actor, ActorRef, LocalActorRef, and Envelope."""

import pytest
from queue import Queue
from actors.actor import Actor, Envelope, LocalActorRef


class TestEnvelope:
    """Tests for Envelope dataclass."""

    def test_envelope_with_message_only(self):
        """Envelope can be created with just a message."""
        env = Envelope(msg="hello")
        assert env.msg == "hello"
        assert env.sender is None
        assert env.reply_queue is None

    def test_envelope_with_sender(self):
        """Envelope can include a sender reference."""
        queue = Queue()
        sender = LocalActorRef(queue, "sender")
        env = Envelope(msg="hello", sender=sender)
        assert env.sender is sender

    def test_envelope_with_reply_queue(self):
        """Envelope can include a reply queue for fast_send."""
        reply_q = Queue()
        env = Envelope(msg="hello", reply_queue=reply_q)
        assert env.reply_queue is reply_q


class TestLocalActorRef:
    """Tests for LocalActorRef."""

    def test_name_property(self):
        """LocalActorRef exposes actor name."""
        queue = Queue()
        ref = LocalActorRef(queue, "test_actor")
        assert ref.name == "test_actor"

    def test_send_puts_envelope_in_queue(self):
        """send() puts an Envelope in the queue."""
        queue = Queue()
        ref = LocalActorRef(queue, "receiver")

        ref.send("hello")

        assert not queue.empty()
        env = queue.get_nowait()
        assert isinstance(env, Envelope)
        assert env.msg == "hello"
        assert env.sender is None

    def test_send_with_sender(self):
        """send() includes sender reference in envelope."""
        queue = Queue()
        ref = LocalActorRef(queue, "receiver")

        sender_queue = Queue()
        sender_ref = LocalActorRef(sender_queue, "sender")
        ref.send("hello", sender=sender_ref)

        env = queue.get_nowait()
        assert env.sender is sender_ref

    def test_fast_send_returns_reply(self):
        """fast_send() waits for reply and returns it."""
        queue = Queue()
        ref = LocalActorRef(queue, "receiver")

        import threading

        def responder():
            env = queue.get()
            assert env.reply_queue is not None
            env.reply_queue.put("response")

        thread = threading.Thread(target=responder)
        thread.start()

        result = ref.fast_send("request")
        assert result == "response"
        thread.join()


class TestActor:
    """Tests for Actor base class."""

    def test_process_message_dispatches_to_handler(self):
        """process_message calls on_<classname> handler."""
        class Ping:
            pass

        class TestActor(Actor):
            def __init__(self):
                super().__init__()
                self.received = None

            def on_ping(self, env):
                self.received = env.msg

        actor = TestActor()
        env = Envelope(msg=Ping())
        actor.process_message(env)

        assert isinstance(actor.received, Ping)

    def test_process_message_ignores_unknown(self):
        """process_message silently ignores unknown message types."""
        class Unknown:
            pass

        actor = Actor()
        env = Envelope(msg=Unknown())
        # Should not raise
        actor.process_message(env)

    def test_reply_to_fast_send(self):
        """reply() puts response in reply_queue for fast_send."""
        reply_q = Queue()
        env = Envelope(msg="request", reply_queue=reply_q)

        actor = Actor()
        actor.reply(env, "response")

        assert reply_q.get_nowait() == "response"

    def test_reply_to_async_send(self):
        """reply() sends to sender's mailbox for async messages."""
        sender_queue = Queue()
        sender_ref = LocalActorRef(sender_queue, "sender")
        env = Envelope(msg="request", sender=sender_ref)

        actor = Actor()
        actor._actor_ref = LocalActorRef(Queue(), "responder")
        actor.reply(env, "response")

        result_env = sender_queue.get_nowait()
        assert result_env.msg == "response"

    def test_stop_sets_running_false(self):
        """stop() sets _running to False."""
        actor = Actor()
        assert actor._running is True
        actor.stop()
        assert actor._running is False
