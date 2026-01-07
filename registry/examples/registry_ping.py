#!/usr/bin/env python3
"""
Registry Ping - Client that looks up pong via GlobalRegistry

Run GlobalRegistry first (python -m actors.registry), then registry_pong.py,
then run this.

Usage:
    python registry_ping.py [registry_endpoint]

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import os
import signal
import sys

# Add actors package to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', 'python'))

from actors import (
    Actor, Envelope, Manager, Start, Shutdown,
    ZmqSender, ZmqReceiver, RemoteActorRef, register_message,
    RegistryClient, RegistryError, ActorNotFoundError, ActorOfflineError
)


# Define Ping message
@register_message
class Ping:
    def __init__(self, count: int = 0):
        self.count = count


# Define Pong message
@register_message
class Pong:
    def __init__(self, count: int = 0):
        self.count = count


class PingActor(Actor):
    """Sends Ping to pong (looked up via registry), receives Pong back."""

    def __init__(self, manager_handle, registry_client: RegistryClient, zmq_sender: ZmqSender):
        super().__init__()
        self.manager_handle = manager_handle
        self.registry_client = registry_client
        self.zmq_sender = zmq_sender
        self.pong_ref = None

    def on_start(self, env: Envelope) -> None:
        print("PingActor: Starting ping-pong...")

        try:
            # Look up pong actor via registry
            pong_endpoint = self.registry_client.lookup("pong")
            print(f"PingActor: Found 'pong' at {pong_endpoint}")

            # Create remote ref for pong
            self.pong_ref = RemoteActorRef("pong", pong_endpoint, self.zmq_sender)

            # Send first ping
            print("PingActor: Sending first ping")
            self.pong_ref.send(Ping(1), sender=self._actor_ref)

        except ActorNotFoundError as e:
            print(f"PingActor: Failed to find 'pong': {e}")
            print("Make sure registry_pong.py is running first!")
            self.manager_handle.terminate()
        except ActorOfflineError as e:
            print(f"PingActor: 'pong' is offline: {e}")
            self.manager_handle.terminate()
        except RegistryError as e:
            print(f"PingActor: Registry error: {e}")
            self.manager_handle.terminate()

    def on_pong(self, env: Envelope) -> None:
        print(f"PingActor: Received pong {env.msg.count} from remote")

        if env.msg.count >= 5:
            print("PingActor: Done!")
            self.manager_handle.terminate()
        else:
            # Send next ping
            if self.pong_ref:
                self.pong_ref.send(Ping(env.msg.count + 1), sender=self._actor_ref)


def main():
    registry_endpoint = "tcp://localhost:5555"
    if len(sys.argv) > 1:
        registry_endpoint = sys.argv[1]

    ENDPOINT = "tcp://*:5002"
    LOCAL_ENDPOINT = "tcp://localhost:5002"

    print("=== Registry Ping Process (port 5002) ===")
    print(f"Registry: {registry_endpoint}")

    # Create manager
    mgr = Manager(endpoint=LOCAL_ENDPOINT)
    handle = mgr.get_handle()

    # Create ZMQ sender/receiver for remote communication
    zmq_sender = ZmqSender(local_endpoint=LOCAL_ENDPOINT)
    zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)

    # Create registry client
    registry_client = RegistryClient("PingManager", registry_endpoint)

    try:
        registry_client.start_heartbeat()
    except RegistryError as e:
        print(f"Failed to connect to registry: {e}")
        return 1

    # Create ping actor - it will lookup pong via registry in on_start
    ping_actor = PingActor(handle, registry_client, zmq_sender)

    # Register actors with manager
    mgr.manage("zmq_receiver", zmq_receiver)
    mgr.manage("ping", ping_actor)

    # Set up signal handler for clean shutdown
    def signal_handler(sig, frame):
        print("\nShutting down...")
        handle.terminate()

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # Start actors
    mgr.init()
    print("Ping process starting...")

    try:
        mgr.run()
    except KeyboardInterrupt:
        pass

    # Cleanup
    registry_client.close()
    mgr.end()

    print("=== Registry Ping Process Complete ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
