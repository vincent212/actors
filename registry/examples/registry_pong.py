#!/usr/bin/env python3
"""
Registry Pong - Server that registers with GlobalRegistry

Run GlobalRegistry first (python -m actors.registry), then run this,
then run registry_ping.py in another terminal.

Usage:
    python registry_pong.py [registry_endpoint]

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
    ZmqSender, ZmqReceiver, register_message,
    RegistryClient, RegistryError
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


class PongActor(Actor):
    """Receives Ping from remote, sends Pong back."""

    def on_start(self, env: Envelope) -> None:
        print("PongActor: Ready to receive pings...")

    def on_ping(self, env: Envelope) -> None:
        print(f"PongActor: Received ping {env.msg.count} from remote")
        self.reply(env, Pong(env.msg.count))


def main():
    registry_endpoint = "tcp://localhost:5555"
    if len(sys.argv) > 1:
        registry_endpoint = sys.argv[1]

    ENDPOINT = "tcp://*:5001"
    LOCAL_ENDPOINT = "tcp://localhost:5001"

    print("=== Registry Pong Process (port 5001) ===")
    print(f"Registry: {registry_endpoint}")

    # Create manager
    mgr = Manager(endpoint=LOCAL_ENDPOINT)
    handle = mgr.get_handle()

    # Create ZMQ sender/receiver for remote communication
    zmq_sender = ZmqSender(local_endpoint=LOCAL_ENDPOINT)
    zmq_receiver = ZmqReceiver(ENDPOINT, mgr, zmq_sender)

    # Create registry client and start heartbeat
    registry_client = RegistryClient("PongManager", registry_endpoint)

    try:
        registry_client.start_heartbeat()

        # Register actors with manager
        mgr.manage("zmq_receiver", zmq_receiver)
        mgr.manage("pong", PongActor())

        # Register pong actor with GlobalRegistry
        registry_client.register("pong", LOCAL_ENDPOINT)
        print(f"Registered 'pong' with GlobalRegistry at {LOCAL_ENDPOINT}")

    except RegistryError as e:
        print(f"Failed to connect to registry: {e}")
        return 1

    # Set up signal handler for clean shutdown
    def signal_handler(sig, frame):
        print("\nShutting down...")
        handle.terminate()

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # Start actors
    mgr.init()
    print("Pong process ready, 'pong' actor registered with GlobalRegistry")
    print("Press Ctrl+C to stop")

    try:
        mgr.run()
    except KeyboardInterrupt:
        pass

    # Cleanup
    registry_client.close()
    mgr.end()

    print("=== Registry Pong Process Complete ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
