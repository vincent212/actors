#!/usr/bin/env python3
"""
Local Ping-Pong Example

Demonstrates:
- Creating custom actors
- Defining custom messages
- Handler registration via on_<classname> methods
- reply() for responding to messages
- ManagerHandle for termination

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import sys
sys.path.insert(0, '/home/vm/actors-py')

from actors import Actor, ActorRef, Envelope, Manager, ManagerHandle, Start


# Custom messages
class Ping:
    def __init__(self, count: int):
        self.count = count


class Pong:
    def __init__(self, count: int):
        self.count = count


class PingActor(Actor):
    """Sends Ping, receives Pong."""

    def __init__(self, pong_ref: ActorRef, manager_handle: ManagerHandle):
        self.pong_ref = pong_ref
        self.manager_handle = manager_handle

    def on_start(self, env: Envelope) -> None:
        print("PingActor: Starting ping-pong")
        self.pong_ref.send(Ping(1), self._actor_ref)

    def on_pong(self, env: Envelope) -> None:
        print(f"PingActor: Received pong {env.msg.count}")
        if env.msg.count >= 5:
            print("PingActor: Done!")
            self.manager_handle.terminate()
        else:
            self.pong_ref.send(Ping(env.msg.count + 1), self._actor_ref)


class PongActor(Actor):
    """Receives Ping, sends Pong."""

    def on_ping(self, env: Envelope) -> None:
        print(f"PongActor: Received ping {env.msg.count}, sending pong")
        self.reply(env, Pong(env.msg.count))


def main():
    print("=== Ping-Pong Actor Example ===")

    mgr = Manager()
    handle = mgr.get_handle()

    # Create actors
    pong_ref = mgr.manage("pong", PongActor())
    ping_actor = PingActor(pong_ref, handle)
    mgr.manage("ping", ping_actor)

    # Start all actors
    mgr.init()

    # Wait for termination
    mgr.run()

    # Shutdown
    mgr.end()

    print("=== Example Complete ===")


if __name__ == "__main__":
    main()
