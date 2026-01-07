#!/usr/bin/env python3
"""
Remote Ping-Pong: Ping Process

Run pong_process.py first, then run this.

Usage:
    python ping_process.py

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
import os
# Add the parent python directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))

from actors import (
    Actor, Envelope, Manager, ManagerHandle, Start,
    RemoteActorRef, ZmqSender, ZmqReceiver, register_message
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


class PingActor(Actor):
    """Sends Ping to remote pong, receives Pong back."""

    def __init__(self, pong_ref: RemoteActorRef, manager_handle: ManagerHandle):
        self.pong_ref = pong_ref
        self.manager_handle = manager_handle

    def on_start(self, env: Envelope) -> None:
        print("PingActor: Starting ping-pong with remote pong")
        self.pong_ref.send(Ping(1), self._actor_ref)

    def on_pong(self, env: Envelope) -> None:
        print(f"PingActor: Received pong {env.msg.count} from remote")
        if env.msg.count >= 5:
            print("PingActor: Done!")
            self.manager_handle.terminate()
        else:
            self.pong_ref.send(Ping(env.msg.count + 1), self._actor_ref)


def main():
    print("=== Ping Process (port 5002) ===")

    LOCAL_ENDPOINT = "tcp://*:5002"
    REMOTE_PONG_ENDPOINT = "tcp://localhost:5001"

    mgr = Manager(endpoint=LOCAL_ENDPOINT)
    handle = mgr.get_handle()

    zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5002")
    zmq_receiver = ZmqReceiver(LOCAL_ENDPOINT, mgr, zmq_sender)

    # Create remote ref to pong on other process
    remote_pong = RemoteActorRef("pong", REMOTE_PONG_ENDPOINT, zmq_sender)

    mgr.manage("zmq_receiver", zmq_receiver)
    mgr.manage("ping", PingActor(remote_pong, handle))

    mgr.init()
    print("Ping process starting...")

    mgr.run()
    mgr.end()

    print("=== Ping Process Complete ===")


if __name__ == "__main__":
    main()
