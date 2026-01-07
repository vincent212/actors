#!/usr/bin/env python3
"""
Reject Example: Sender Process

This example demonstrates error handling when a remote message type is unknown.
The sender sends a message that the receiver doesn't know about, and receives
a Reject message back.

Run receiver first, then run this:
    python examples/reject_example/sender.py

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
sys.path.insert(0, '/home/vm/actors-py')

from actors import (
    Actor, Envelope, Manager, Start, Reject,
    RemoteActorRef, ZmqSender, ZmqReceiver,
    register_message, MESSAGE_REGISTRY
)

# Register Reject for remote deserialization
MESSAGE_REGISTRY["Reject"] = Reject


# Define a message that the receiver DOES NOT know about
@register_message
class UnknownMessage:
    def __init__(self, data: int):
        self.data = data


# Define Ping/Pong for normal communication
@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count


@register_message
class Pong:
    def __init__(self, count: int):
        self.count = count


class SenderActor(Actor):
    """Sends messages and handles Reject responses."""

    def __init__(self, receiver_ref: RemoteActorRef, manager_handle):
        self.receiver_ref = receiver_ref
        self.manager_handle = manager_handle
        self.rejected_count = 0

    def on_start(self, env: Envelope) -> None:
        print("SenderActor: Starting test...")

        # First, send an UnknownMessage (receiver doesn't know this type)
        print("SenderActor: Sending UnknownMessage (should be rejected)...")
        self.receiver_ref.send(UnknownMessage(data=42), sender=self._actor_ref)

        # Also send a valid Ping to show normal operation continues
        print("SenderActor: Sending Ping (should succeed)...")
        self.receiver_ref.send(Ping(count=1), sender=self._actor_ref)

    def on_pong(self, env: Envelope) -> None:
        print(f"SenderActor: Received Pong {env.msg.count} - normal message worked!")

        # Wait a bit then terminate
        print("SenderActor: Test complete!")
        self.manager_handle.terminate()

    def on_reject(self, env: Envelope) -> None:
        self.rejected_count += 1
        msg = env.msg
        print("SenderActor: Received Reject!")
        print(f"  - Message type: {msg.message_type}")
        print(f"  - Reason: {msg.reason}")
        print(f"  - Rejected by: {msg.rejected_by}")


def main():
    print("=== Reject Example: Sender (port 5002) -> Receiver (port 5001) ===")

    local_endpoint = "tcp://0.0.0.0:5002"
    remote_receiver_endpoint = "tcp://localhost:5001"

    # Create manager
    mgr = Manager()
    handle = mgr.get_handle()

    # Create ZMQ sender
    zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5002")

    # Create remote ref to receiver process
    remote_receiver = RemoteActorRef(
        name="receiver",
        endpoint=remote_receiver_endpoint,
        zmq_sender=zmq_sender
    )

    # Create sender actor
    sender = SenderActor(remote_receiver, handle)
    sender_ref = mgr.manage("sender", sender)

    # Create ZMQ receiver for incoming messages
    zmq_receiver = ZmqReceiver(
        bind_endpoint=local_endpoint,
        manager=mgr,
        zmq_sender=zmq_sender
    )
    mgr.manage("zmq_receiver", zmq_receiver)

    # Start all actors
    mgr.init()

    print("Sender process starting...")

    # Run until terminated
    mgr.run()
    mgr.end()

    print("=== Sender Process Complete ===")


if __name__ == "__main__":
    main()
