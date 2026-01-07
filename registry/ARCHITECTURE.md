# Global Actor Registry - Architecture

## System Overview

<system-description>
Multi-language actor registry enabling cross-process actor lookup with heartbeat-based health monitoring. The GlobalRegistry is a centralized actor (implemented in Python) that maintains actor name → endpoint mappings. Managers register their actors during manage() and send periodic heartbeats. Failed heartbeats mark actors offline but still return their endpoints (caller decides what to do).
</system-description>

## Key Concepts

<concept name="GlobalRegistry">
Central actor (Python) that maintains actor name → endpoint mappings across all Managers.
Tracks heartbeats from each Manager and marks actors offline after 6 seconds of silence.
Can restart managers via SSH + systemd when heartbeats fail.
</concept>

<concept name="Manager">
Per-process actor lifecycle manager. Each language (C++, Rust, Python) has its own Manager implementation.
Registers actors with GlobalRegistry during manage(). Sends heartbeats every 2 seconds.
</concept>

<concept name="RegistryClient">
Helper class for communicating with GlobalRegistry. Available in C++ and Rust.
Handles heartbeat thread, registration, and lookup operations.
Python actors interact directly with GlobalRegistry messages.
</concept>

<concept name="ActorRef">
Unified reference to local or remote actor. In C++ uses std::variant for zero-overhead polymorphism.
For cross-process communication, contains ZMQ endpoint string.
</concept>

<concept name="Heartbeat">
Managers send Heartbeat every 2 seconds with their manager_id and timestamp.
GlobalRegistry marks all actors from a manager as offline if no heartbeat for 6 seconds.
Actors automatically come back online when heartbeats resume.
</concept>

## Message Protocol

<message-protocol version="1.0">

<message name="RegisterActor" id="900" direction="Manager → Registry">
  <field name="manager_id" type="string">Unique identifier for the sending Manager</field>
  <field name="actor_name" type="string">Unique name for the actor being registered</field>
  <field name="actor_endpoint" type="string">ZMQ endpoint where actor can be reached</field>
</message>

<message name="UnregisterActor" id="901" direction="Manager → Registry">
  <field name="actor_name" type="string">Name of actor to unregister</field>
</message>

<message name="RegistrationOk" id="902" direction="Registry → Manager">
  <field name="actor_name" type="string">Name of successfully registered actor</field>
</message>

<message name="RegistrationFailed" id="903" direction="Registry → Manager">
  <field name="actor_name" type="string">Name of actor that failed to register</field>
  <field name="reason" type="string">Why registration failed (e.g., "name already taken")</field>
</message>

<message name="LookupActor" id="904" direction="Manager → Registry">
  <field name="actor_name" type="string">Name of actor to lookup</field>
</message>

<message name="LookupResult" id="905" direction="Registry → Manager">
  <field name="actor_name" type="string">Name that was looked up</field>
  <field name="endpoint" type="optional<string>">ZMQ endpoint if found, None if not found</field>
  <field name="online" type="bool">True if manager is sending heartbeats</field>
</message>

<message name="Heartbeat" id="906" direction="Manager → Registry">
  <field name="manager_id" type="string">Manager sending the heartbeat</field>
  <field name="timestamp_ms" type="uint64">Milliseconds since epoch</field>
</message>

<message name="HeartbeatAck" id="907" direction="Registry → Manager">
  (no fields - simple acknowledgement)
</message>

</message-protocol>

## Data Structures

<data-structure name="GlobalRegistry.registry" language="python">
Dict mapping actor_name → ActorEntry(endpoint, manager_id)
Example: {"OrderProcessor": ActorEntry("tcp://192.168.1.10:5556", "OrderManager")}
</data-structure>

<data-structure name="GlobalRegistry.heartbeats" language="python">
Dict mapping manager_id → last_heartbeat_time (float)
Example: {"OrderManager": 1704067200.123, "RiskManager": 1704067199.456}
</data-structure>

<data-structure name="GlobalRegistry.manager_actors" language="python">
Dict mapping manager_id → List[actor_name]
Used to mark all actors offline when a manager fails.
Example: {"OrderManager": ["OrderProcessor", "OrderValidator"]}
</data-structure>

## Data Flows

<flow name="Actor Registration">
1. Manager calls manage(actor) with new actor
2. Manager sends RegisterActor{manager_id, actor_name, endpoint} to GlobalRegistry
3. GlobalRegistry checks if name is already taken
4. If available: stores in registry dict, replies RegistrationOk
5. If taken: replies RegistrationFailed with reason
6. Manager throws exception or logs error on failure
</flow>

<flow name="Actor Lookup">
1. Actor A needs to communicate with actor B by name
2. Manager checks local registry first (same process)
3. If not found locally, Manager sends LookupActor{actor_name} to GlobalRegistry
4. GlobalRegistry finds the entry and checks if manager is online (heartbeat within 6s)
5. GlobalRegistry replies LookupResult{actor_name, endpoint, online}
6. Manager creates RemoteActorRef with endpoint
7. Actor A can now send messages to Actor B via the ActorRef
</flow>

<flow name="Heartbeat Monitoring">
1. Manager starts heartbeat thread on init
2. Thread sends Heartbeat{manager_id, timestamp} every 2 seconds
3. GlobalRegistry updates heartbeats[manager_id] = current_time
4. GlobalRegistry's check_heartbeats() runs every 1 second
5. For each manager with last_heartbeat > 6 seconds ago:
   - Log warning
   - All actors from that manager are marked offline (lookup returns online=false)
   - Optionally: SSH to host and restart via systemd
6. When heartbeat resumes, actors automatically become online again
</flow>

<flow name="Manager Restart (on heartbeat failure)">
1. GlobalRegistry detects manager missed heartbeats (6+ seconds)
2. Looks up host config: hosts[host_name].ssh = "actors@192.168.1.10"
3. Looks up service name: hosts[host_name].managers[manager_id].service = "manager-order"
4. Executes: ssh actors@192.168.1.10 "sudo systemctl restart manager-order"
5. Waits for heartbeats to resume
6. Logs success or failure
</flow>

## Error Handling

<error name="ActorNotFoundError">
Thrown by lookup() when actor_name is not registered.
Caller should handle by logging error or retrying later.
</error>

<error name="ActorOfflineError">
Thrown by lookup() when actor exists but manager missed heartbeats.
Endpoint is still available via lookup_allow_offline() if caller wants to try anyway.
</error>

<error name="RegistrationFailedError">
Thrown by register_actor() when name is already taken.
Caller should use a different name or check if existing registration is stale.
</error>

<error name="TimeoutError">
Thrown when no reply from GlobalRegistry within timeout (default 5 seconds).
Indicates network issue or registry overloaded.
</error>

## File Map

<file-map>
registry/
├── ARCHITECTURE.md              # This file - AI-readable architecture
├── README.md                    # Developer guide
├── cpp/
│   ├── RegistryMessages.hpp     # C++ protocol messages (IDs 900-907)
│   ├── RegistryClient.hpp       # C++ registry client helper
│   ├── RegistryClient.cpp       # Implementation
│   └── Makefile                 # Build libactors_registry.a
├── rust/
│   ├── Cargo.toml               # Rust crate config
│   └── src/
│       ├── lib.rs               # Module exports
│       ├── messages.rs          # Protocol messages
│       └── registry.rs          # RegistryClient implementation
├── python/
│   └── actors_registry/
│       ├── __init__.py          # Package exports
│       ├── main.py              # Entry point for running GlobalRegistry
│       ├── registry.py          # GlobalRegistry actor implementation
│       └── messages.py          # Protocol message dataclasses
├── systemd/
│   ├── global-registry.service  # Systemd unit for GlobalRegistry
│   ├── manager-template.service # Template for manager services
│   └── install.sh               # Service installation script
├── config/
│   └── registry.json            # Example configuration
└── examples/
    ├── shared/
    │   └── example_messages.hpp # Shared example message types
    ├── cpp_to_python/           # C++ actor calls Python actor
    │   ├── README.md
    │   ├── order_processor.cpp
    │   ├── risk_checker.py
    │   └── Makefile
    └── multi_host/              # Multi-machine deployment
        ├── README.md
        ├── start_system.sh
        ├── stop_system.sh
        └── status_system.sh
</file-map>

## Integration Points

<integration language="C++" entry="Manager.hpp + RegistryClient">
C++ managers use RegistryClient to communicate with GlobalRegistry.
RegistryClient handles heartbeat thread and provides register_actor()/lookup() methods.
Throws C++ exceptions on errors (ActorNotFoundError, etc.)
</integration>

<integration language="Rust" entry="registry.rs::RegistryClient">
Rust managers use RegistryClient struct with start_heartbeat(), register(), lookup() methods.
Returns Result<T, RegistryError> for all operations.
Uses actors crate's ActorRef for communication.
</integration>

<integration language="Python" entry="actors_registry.GlobalRegistry">
The GlobalRegistry is implemented in Python. Run via:
  python -m actors_registry.main --config /etc/actors/registry.json
Python managers can send messages directly without a separate client class.
</integration>

## Configuration

<config-schema path="/etc/actors/registry.json">
{
  "registry_endpoint": "tcp://0.0.0.0:5555",  // ZMQ bind address
  "heartbeat_timeout_s": 6.0,                  // Seconds before marking offline
  "heartbeat_check_interval_s": 1.0,           // How often to check heartbeats
  "hosts": {
    "<host_name>": {
      "ssh": "<user>@<ip>",                    // SSH address for remote control
      "managers": {
        "<manager_id>": {
          "service": "<systemd-service-name>", // e.g., "manager-order-processor"
          "language": "<cpp|rust|python>",     // Optional, for documentation
          "description": "<text>"              // Optional
        }
      }
    }
  }
}
</config-schema>

## Design Decisions

<decision name="Python for GlobalRegistry">
GlobalRegistry is implemented in Python because:
1. Not on hot path (registration/lookup are infrequent)
2. Easier SSH/subprocess handling for remote process control
3. Easier JSON config parsing
4. Simpler to extend with web UI or REST API later
</decision>

<decision name="Endpoint strings instead of ActorRef">
Cross-language messages use endpoint strings (e.g., "tcp://host:port") instead of ActorRef objects.
Reason: ActorRef contains language-specific runtime pointers that can't be serialized across processes.
Each language reconstructs its own ActorRef from the endpoint string.
</decision>

<decision name="Return offline actors">
LookupResult returns the endpoint even if manager is offline.
Reason: Caller may want to:
1. Wait and retry (actor might recover)
2. Queue messages for later delivery
3. Show status to user
Throwing immediately removes this flexibility.
</decision>

<decision name="systemd + SSH for process control">
Using systemd for service management and SSH for remote control.
Simpler than custom HostAgent daemons. Leverages existing Linux tooling.
</decision>
