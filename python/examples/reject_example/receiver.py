#!/usr/bin/env python3
"""
Reject Example: Receiver Process

This example demonstrates error handling when a remote message type is unknown.
The receiver only knows about Ping/Pong, not UnknownMessage.

Run this first, then run sender:
    python examples/reject_example/receiver.py

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
import signal
sys.path.insert(0, '/home/vm/actors-py')

from actors import (
    Actor, Envelope, Manager,
    ZmqSender, ZmqReceiver,
    register_message
)


# Receiver only knows about Ping/Pong - NOT UnknownMessage
@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count


@register_message
class Pong:
    def __init__(self, count: int):
        self.count = count


class ReceiverActor(Actor):
    """Receives Ping, sends Pong back.
    Unknown messages will trigger automatic Reject responses.
    """

    def on_ping(self, env: Envelope) -> None:
        print(f"ReceiverActor: Received Ping {env.msg.count}")
        self.reply(env, Pong(count=env.msg.count))
        print(f"ReceiverActor: Sent Pong {env.msg.count} back")


def main():
    print("=== Reject Example: Receiver (port 5001) ===")
    print("This receiver only knows Ping/Pong - UnknownMessage will be rejected")

    local_endpoint = "tcp://0.0.0.0:5001"

    # Create manager
    mgr = Manager()

    # Create ZMQ sender (for replies)
    zmq_sender = ZmqSender(local_endpoint=local_endpoint)

    # Create receiver actor
    receiver = ReceiverActor()
    receiver_ref = mgr.manage("receiver", receiver)

    # Create ZMQ receiver for incoming messages
    zmq_receiver = ZmqReceiver(
        bind_endpoint=local_endpoint,
        manager=mgr,
        zmq_sender=zmq_sender
    )
    mgr.manage("zmq_receiver", zmq_receiver)

    # Handle Ctrl+C
    handle = mgr.get_handle()
    def signal_handler(signum, frame):
        print("\nReceiving shutdown signal...")
        handle.terminate()

    signal.signal(signal.SIGINT, signal_handler)

    # Start all actors
    mgr.init()

    print("Receiver process ready, waiting for messages...")
    print("Press Ctrl+C to stop")

    # Run until terminated
    mgr.run()
    mgr.end()

    print("=== Receiver Process Complete ===")


if __name__ == "__main__":
    main()
