"""
GlobalRegistry - Central actor registry for cross-Manager actor lookup.

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
Copyright 2025 Vincent Maciejewski, & M2 Tech
"""

import json
import subprocess
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Set
import logging

from actors import Actor, Manager, LocalActorRef
from .registry_messages import (
    RegisterActor, UnregisterActor, RegistrationOk, RegistrationFailed,
    LookupActor, LookupResult, Heartbeat, HeartbeatAck,
    StartManager, StopManager, RestartManager, ManagerStatus
)

logger = logging.getLogger(__name__)


@dataclass
class ActorEntry:
    """Registry entry for an actor."""
    endpoint: str
    manager_id: str


@dataclass
class HostConfig:
    """Configuration for a remote host."""
    ssh: str  # e.g., "user@192.168.1.10"
    managers: Dict[str, dict] = field(default_factory=dict)


class GlobalRegistry(Actor):
    """Central actor registry for cross-Manager actor lookup.

    The GlobalRegistry:
    - Maintains actor name -> endpoint mappings from all Managers
    - Tracks Manager health via heartbeats (2s interval, 6s timeout)
    - Provides sync lookup for actors by name
    - Marks actors offline when their Manager misses heartbeats
    - Can restart Managers via SSH + systemctl

    Usage:
        registry = GlobalRegistry(config_path="/path/to/registry.json")
        manager = Manager()
        manager.manage("GlobalRegistry", registry)
        manager.init()
        manager.run()
    """

    HEARTBEAT_TIMEOUT_S = 6.0  # 3 missed heartbeats (2s each)
    HEARTBEAT_CHECK_INTERVAL_S = 1.0

    def __init__(self, config_path: Optional[str] = None):
        super().__init__()

        # actor_name -> ActorEntry
        self._registry: Dict[str, ActorEntry] = {}

        # manager_id -> last_heartbeat_time (monotonic)
        self._heartbeats: Dict[str, float] = {}

        # manager_id -> set of actor_names
        self._manager_actors: Dict[str, Set[str]] = {}

        # Host configuration for SSH control
        self._hosts: Dict[str, HostConfig] = {}

        # manager_id -> host_id mapping
        self._manager_to_host: Dict[str, str] = {}

        # Load config if provided
        if config_path:
            self._load_config(config_path)

        # Background thread for heartbeat monitoring
        self._monitor_thread: Optional[threading.Thread] = None
        self._running = False

    def _load_config(self, config_path: str) -> None:
        """Load host configuration from JSON file."""
        path = Path(config_path)
        if not path.exists():
            logger.warning(f"Config file not found: {config_path}")
            return

        with open(path) as f:
            config = json.load(f)

        for host_id, host_data in config.get("hosts", {}).items():
            self._hosts[host_id] = HostConfig(
                ssh=host_data.get("ssh", ""),
                managers=host_data.get("managers", {})
            )
            # Build manager -> host mapping
            for manager_id in host_data.get("managers", {}).keys():
                self._manager_to_host[manager_id] = host_id

        logger.info(f"Loaded config with {len(self._hosts)} hosts")

    def init(self) -> None:
        """Start heartbeat monitoring thread."""
        self._running = True
        self._monitor_thread = threading.Thread(
            target=self._heartbeat_monitor,
            daemon=True,
            name="heartbeat-monitor"
        )
        self._monitor_thread.start()
        logger.info("GlobalRegistry started")

    def end(self) -> None:
        """Stop heartbeat monitoring."""
        self._running = False
        if self._monitor_thread:
            self._monitor_thread.join(timeout=2.0)
        logger.info("GlobalRegistry stopped")

    def _heartbeat_monitor(self) -> None:
        """Background thread to check for stale heartbeats."""
        while self._running:
            time.sleep(self.HEARTBEAT_CHECK_INTERVAL_S)
            self._check_heartbeats()

    def _check_heartbeats(self) -> None:
        """Check for managers that have missed heartbeats and unregister their actors."""
        now = time.monotonic()
        stale_managers = []

        for manager_id, last_hb in self._heartbeats.items():
            if now - last_hb > self.HEARTBEAT_TIMEOUT_S:
                stale_managers.append(manager_id)

        for manager_id in stale_managers:
            logger.warning(f"Manager '{manager_id}' timed out, unregistering its actors")
            self._unregister_manager(manager_id)

    def _unregister_manager(self, manager_id: str) -> None:
        """Unregister all actors belonging to a manager."""
        # Get actors for this manager
        actor_names = self._manager_actors.pop(manager_id, set())

        # Remove each actor from registry
        for actor_name in actor_names:
            if actor_name in self._registry:
                del self._registry[actor_name]
                logger.info(f"Unregistered '{actor_name}' (manager '{manager_id}' timed out)")

        # Remove heartbeat tracking
        self._heartbeats.pop(manager_id, None)

    def is_manager_online(self, manager_id: str) -> bool:
        """Check if a manager has recent heartbeat."""
        if manager_id not in self._heartbeats:
            return False
        elapsed = time.monotonic() - self._heartbeats[manager_id]
        return elapsed < self.HEARTBEAT_TIMEOUT_S

    def lookup(self, actor_name: str) -> Optional[str]:
        """Synchronous lookup - returns endpoint or None."""
        entry = self._registry.get(actor_name)
        if entry:
            return entry.endpoint
        return None

    def get_all_actors(self) -> List[str]:
        """Get list of all registered actor names."""
        return list(self._registry.keys())

    def get_all_managers(self) -> List[str]:
        """Get list of all registered manager IDs."""
        return list(self._manager_actors.keys())

    # Message handlers

    def _on_register(self, msg: RegisterActor, ctx) -> None:
        """Handle actor registration."""
        if msg.actor_name in self._registry:
            logger.warning(f"Registration failed: '{msg.actor_name}' already registered")
            ctx.reply(RegistrationFailed(
                actor_name=msg.actor_name,
                reason="Name already registered"
            ))
            return

        # Register the actor
        self._registry[msg.actor_name] = ActorEntry(
            endpoint=msg.actor_endpoint,
            manager_id=msg.manager_id
        )

        # Track which actors belong to which manager
        if msg.manager_id not in self._manager_actors:
            self._manager_actors[msg.manager_id] = set()
        self._manager_actors[msg.manager_id].add(msg.actor_name)

        # Registration counts as heartbeat
        self._heartbeats[msg.manager_id] = time.monotonic()

        logger.info(f"Registered '{msg.actor_name}' from manager '{msg.manager_id}'")
        ctx.reply(RegistrationOk(actor_name=msg.actor_name))

    def _on_unregister(self, msg: UnregisterActor, ctx) -> None:
        """Handle actor unregistration."""
        entry = self._registry.pop(msg.actor_name, None)
        if entry is None:
            logger.warning(f"Unregister failed: '{msg.actor_name}' not found")
            return

        # Remove from manager's actor set
        if entry.manager_id in self._manager_actors:
            self._manager_actors[entry.manager_id].discard(msg.actor_name)

        logger.info(f"Unregistered '{msg.actor_name}'")

    def _on_lookup(self, msg: LookupActor, ctx) -> None:
        """Handle actor lookup."""
        entry = self._registry.get(msg.actor_name)

        if entry is None:
            ctx.reply(LookupResult(
                actor_name=msg.actor_name,
                endpoint=None,
                online=False
            ))
            return

        online = self.is_manager_online(entry.manager_id)
        ctx.reply(LookupResult(
            actor_name=msg.actor_name,
            endpoint=entry.endpoint,
            online=online
        ))

    def _on_heartbeat(self, msg: Heartbeat, ctx) -> None:
        """Handle heartbeat from manager."""
        self._heartbeats[msg.manager_id] = time.monotonic()
        ctx.reply(HeartbeatAck())

    # Process management via SSH

    def _on_start_manager(self, msg: StartManager, ctx) -> None:
        """Start a manager via SSH + systemctl."""
        result = self._systemctl_command(msg.manager_id, "start")
        ctx.reply(result)

    def _on_stop_manager(self, msg: StopManager, ctx) -> None:
        """Stop a manager via SSH + systemctl."""
        result = self._systemctl_command(msg.manager_id, "stop")
        ctx.reply(result)

    def _on_restart_manager(self, msg: RestartManager, ctx) -> None:
        """Restart a manager via SSH + systemctl."""
        result = self._systemctl_command(msg.manager_id, "restart")
        ctx.reply(result)

    def _systemctl_command(self, manager_id: str, action: str) -> ManagerStatus:
        """Execute systemctl command via SSH."""
        host_id = self._manager_to_host.get(manager_id)
        if not host_id or host_id not in self._hosts:
            return ManagerStatus(
                manager_id=manager_id,
                running=False,
                error=f"Unknown manager: {manager_id}"
            )

        host = self._hosts[host_id]
        manager_config = host.managers.get(manager_id, {})
        service_name = manager_config.get("service", manager_id)

        cmd = f"sudo systemctl {action} {service_name}"
        ssh_cmd = ["ssh", host.ssh, cmd]

        try:
            logger.info(f"Executing: {' '.join(ssh_cmd)}")
            result = subprocess.run(
                ssh_cmd,
                capture_output=True,
                text=True,
                timeout=30
            )

            if result.returncode == 0:
                logger.info(f"Successfully {action}ed {manager_id}")
                return ManagerStatus(
                    manager_id=manager_id,
                    running=(action != "stop")
                )
            else:
                logger.error(f"Failed to {action} {manager_id}: {result.stderr}")
                return ManagerStatus(
                    manager_id=manager_id,
                    running=False,
                    error=result.stderr
                )

        except subprocess.TimeoutExpired:
            logger.error(f"Timeout executing {action} on {manager_id}")
            return ManagerStatus(
                manager_id=manager_id,
                running=False,
                error="SSH timeout"
            )
        except Exception as e:
            logger.error(f"Error executing {action} on {manager_id}: {e}")
            return ManagerStatus(
                manager_id=manager_id,
                running=False,
                error=str(e)
            )

    def _restart_manager_via_ssh(self, manager_id: str) -> None:
        """Auto-restart a manager that missed heartbeats."""
        logger.info(f"Auto-restarting manager {manager_id}")
        self._systemctl_command(manager_id, "restart")


def run_registry(endpoint: str = "tcp://0.0.0.0:5555", config_path: str = None):
    """Run the GlobalRegistry as a standalone ZMQ server.

    Args:
        endpoint: ZMQ endpoint to bind to (default: tcp://0.0.0.0:5555)
        config_path: Optional path to registry.json config file
    """
    import zmq
    import signal
    from .registry_messages import (
        RegisterActor, UnregisterActor, LookupActor, Heartbeat,
        RegistrationOk, RegistrationFailed, LookupResult, HeartbeatAck
    )

    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )

    logger.info(f"Starting GlobalRegistry on {endpoint}")

    # Create registry
    registry = GlobalRegistry(config_path)
    registry.init()

    # Create ZMQ socket
    context = zmq.Context()
    socket = context.socket(zmq.REP)
    socket.bind(endpoint)

    running = True

    def signal_handler(sig, frame):
        nonlocal running
        logger.info("Received shutdown signal")
        running = False

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    logger.info("GlobalRegistry ready, waiting for messages...")

    while running:
        try:
            # Poll with timeout so we can check running flag
            if socket.poll(1000):
                msg_bytes = socket.recv()
                msg_json = json.loads(msg_bytes.decode('utf-8'))

                msg_type = msg_json.get('message_type')
                reply = None

                if msg_type == 'RegisterActor':
                    msg = RegisterActor(
                        manager_id=msg_json['manager_id'],
                        actor_name=msg_json['actor_name'],
                        actor_endpoint=msg_json['actor_endpoint']
                    )
                    if msg.actor_name in registry._registry:
                        reply = RegistrationFailed(
                            actor_name=msg.actor_name,
                            reason="Name already registered"
                        )
                    else:
                        registry._registry[msg.actor_name] = ActorEntry(
                            endpoint=msg.actor_endpoint,
                            manager_id=msg.manager_id
                        )
                        if msg.manager_id not in registry._manager_actors:
                            registry._manager_actors[msg.manager_id] = set()
                        registry._manager_actors[msg.manager_id].add(msg.actor_name)
                        registry._heartbeats[msg.manager_id] = time.monotonic()
                        logger.info(f"Registered '{msg.actor_name}' from '{msg.manager_id}'")
                        reply = RegistrationOk(actor_name=msg.actor_name)

                elif msg_type == 'UnregisterActor':
                    actor_name = msg_json['actor_name']
                    entry = registry._registry.pop(actor_name, None)
                    if entry and entry.manager_id in registry._manager_actors:
                        registry._manager_actors[entry.manager_id].discard(actor_name)
                    logger.info(f"Unregistered '{actor_name}'")
                    reply = RegistrationOk(actor_name=actor_name)

                elif msg_type == 'LookupActor':
                    actor_name = msg_json['actor_name']
                    entry = registry._registry.get(actor_name)
                    if entry:
                        online = registry.is_manager_online(entry.manager_id)
                        reply = LookupResult(
                            actor_name=actor_name,
                            endpoint=entry.endpoint,
                            online=online
                        )
                    else:
                        reply = LookupResult(
                            actor_name=actor_name,
                            endpoint=None,
                            online=False
                        )

                elif msg_type == 'Heartbeat':
                    manager_id = msg_json['manager_id']
                    registry._heartbeats[manager_id] = time.monotonic()
                    reply = HeartbeatAck()

                else:
                    logger.warning(f"Unknown message type: {msg_type}")
                    reply = {'error': f'Unknown message type: {msg_type}'}

                # Send reply
                if hasattr(reply, 'to_dict'):
                    reply_json = reply.to_dict()
                else:
                    reply_json = reply
                socket.send(json.dumps(reply_json).encode('utf-8'))

        except zmq.ZMQError as e:
            if running:
                logger.error(f"ZMQ error: {e}")

    # Cleanup
    registry.end()
    socket.close()
    context.term()
    logger.info("GlobalRegistry stopped")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Run GlobalRegistry server")
    parser.add_argument(
        "--endpoint",
        default="tcp://0.0.0.0:5555",
        help="ZMQ endpoint to bind to (default: tcp://0.0.0.0:5555)"
    )
    parser.add_argument(
        "--config",
        default=None,
        help="Path to registry.json config file"
    )

    args = parser.parse_args()
    run_registry(args.endpoint, args.config)
