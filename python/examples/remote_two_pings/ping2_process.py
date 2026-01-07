#!/usr/bin/env python3
"""
Remote Two Pings: Ping2 Process

Second ping process - sends pings to shared pong.

Run pong_process.py first, then run this and ping1_process.py.

Usage:
    python ping2_process.py

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
sys.path.insert(0, '/home/vm/actors-py')

from actors import (
    Actor, Envelope, Manager, ManagerHandle, Start,
    RemoteActorRef, ZmqSender, ZmqReceiver, register_message
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


class PingActor(Actor):
    """Sends Ping to remote pong, receives Pong back."""

    def __init__(self, pong_ref: RemoteActorRef, manager_handle: ManagerHandle, name: str):
        self.pong_ref = pong_ref
        self.manager_handle = manager_handle
        self.my_name = name

    def on_start(self, env: Envelope) -> None:
        print(f"{self.my_name}: Starting ping-pong with remote pong")
        self.pong_ref.send(Ping(1, self.my_name), self._actor_ref)

    def on_pong(self, env: Envelope) -> None:
        print(f"{self.my_name}: Received pong {env.msg.count}")
        if env.msg.count >= 3:
            print(f"{self.my_name}: Done!")
            self.manager_handle.terminate()
        else:
            self.pong_ref.send(Ping(env.msg.count + 1, self.my_name), self._actor_ref)


def main():
    print("=== Ping2 Process (port 5003) ===")

    LOCAL_ENDPOINT = "tcp://*:5003"
    REMOTE_PONG_ENDPOINT = "tcp://localhost:5001"

    mgr = Manager(endpoint=LOCAL_ENDPOINT)
    handle = mgr.get_handle()

    zmq_sender = ZmqSender(local_endpoint="tcp://localhost:5003")
    zmq_receiver = ZmqReceiver(LOCAL_ENDPOINT, mgr, zmq_sender)

    # Create remote ref to pong on other process
    remote_pong = RemoteActorRef("pong", REMOTE_PONG_ENDPOINT, zmq_sender)

    mgr.manage("zmq_receiver", zmq_receiver)
    mgr.manage("ping2", PingActor(remote_pong, handle, "Ping2"))

    mgr.init()
    print("Ping2 process starting...")

    mgr.run()
    mgr.end()

    print("=== Ping2 Process Complete ===")


if __name__ == "__main__":
    main()
