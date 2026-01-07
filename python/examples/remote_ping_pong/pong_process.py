#!/usr/bin/env python3
"""
Remote Ping-Pong: Pong Process

Run this first, then run ping_process.py in another terminal.

Usage:
    python pong_process.py

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
import os
# Add the parent python directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))

from actors import (
    Actor, Envelope, Manager,
    ZmqSender, ZmqReceiver, register_message
)


# Register messages for serialization
@register_message
class Ping:
    def __init__(self, count: int):
        self.count = count


@register_message
class Pong:
    def __init__(self, count: int):
        self.count = count


class PongActor(Actor):
    """Receives Ping from remote, sends Pong back."""

    def on_ping(self, env: Envelope) -> None:
        print(f"PongActor: Received ping {env.msg.count} from {env.sender.name}")
        self.reply(env, Pong(env.msg.count))


def main():
    print("=== Pong Process (port 5001) ===")

    ENDPOINT = "tcp://*:5001"

    mgr = Manager(endpoint=ENDPOINT)
    zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5001")
    zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)

    mgr.manage("zmq_receiver", zmq_receiver)
    mgr.manage("pong", PongActor())

    mgr.init()
    print("Pong process ready, waiting for pings...")
    print("Press Ctrl+C to stop")

    try:
        mgr.run()
    except KeyboardInterrupt:
        print("\nShutting down...")

    mgr.end()
    print("=== Pong Process Complete ===")


if __name__ == "__main__":
    main()
