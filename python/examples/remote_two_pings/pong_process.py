#!/usr/bin/env python3
"""
Remote Two Pings: Pong Process

Handles pings from two different processes and routes replies correctly.

Run this first, then run ping1_process.py and ping2_process.py.

Usage:
    python pong_process.py

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
sys.path.insert(0, '/home/vm/actors-py')

from actors import (
    Actor, Envelope, Manager,
    ZmqSender, ZmqReceiver, register_message
)


# Register messages for serialization
@register_message
class Ping:
    def __init__(self, count: int, source: str = ""):
        self.count = count
        self.source = source


@register_message
class Pong:
    def __init__(self, count: int, source: str = ""):
        self.count = count
        self.source = source


class PongActor(Actor):
    """Receives Ping from multiple remote senders, routes replies correctly."""

    def on_ping(self, env: Envelope) -> None:
        sender_name = env.sender.name if env.sender else "unknown"
        print(f"PongActor: Received ping {env.msg.count} from {sender_name} (source={env.msg.source})")
        # Reply goes back to correct sender via RemoteActorRef
        self.reply(env, Pong(env.msg.count, env.msg.source))


def main():
    print("=== Pong Process (port 5001) ===")
    print("Handles pings from multiple remote processes")

    ENDPOINT = "tcp://*:5001"

    mgr = Manager(endpoint=ENDPOINT)
    zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5001")
    zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)

    mgr.manage("zmq_receiver", zmq_receiver)
    mgr.manage("pong", PongActor())

    mgr.init()
    print("Pong process ready, waiting for pings from multiple sources...")
    print("Press Ctrl+C to stop")

    try:
        mgr.run()
    except KeyboardInterrupt:
        print("\nShutting down...")

    mgr.end()
    print("=== Pong Process Complete ===")


if __name__ == "__main__":
    main()
