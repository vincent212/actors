/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech

*/

#pragma once

#include <atomic>
#include <chrono>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include "actors/ActorRef.hpp"
#include "actors/registry/RegistryMessages.hpp"

namespace actors::registry {

/**
 * Error types for registry operations.
 */
class RegistryError : public std::runtime_error {
public:
    explicit RegistryError(const std::string& msg) : std::runtime_error(msg) {}
};

class ActorNotFoundError : public RegistryError {
public:
    explicit ActorNotFoundError(const std::string& name)
        : RegistryError("Actor not found: " + name), actor_name_(name) {}
    const std::string& actor_name() const { return actor_name_; }
private:
    std::string actor_name_;
};

class ActorOfflineError : public RegistryError {
public:
    explicit ActorOfflineError(const std::string& name)
        : RegistryError("Actor offline: " + name), actor_name_(name) {}
    const std::string& actor_name() const { return actor_name_; }
private:
    std::string actor_name_;
};

class RegistrationFailedError : public RegistryError {
public:
    RegistrationFailedError(const std::string& name, const std::string& reason)
        : RegistryError("Registration failed for '" + name + "': " + reason)
        , actor_name_(name), reason_(reason) {}
    const std::string& actor_name() const { return actor_name_; }
    const std::string& reason() const { return reason_; }
private:
    std::string actor_name_;
    std::string reason_;
};

class TimeoutError : public RegistryError {
public:
    explicit TimeoutError(const std::string& msg) : RegistryError("Timeout: " + msg) {}
};

/**
 * RegistryClient - Client for communicating with GlobalRegistry
 *
 * The RegistryClient:
 * - Sends heartbeats every 2 seconds in a background thread
 * - Provides sync lookup for actors by name
 * - Handles registration of local actors
 *
 * Usage:
 *   auto registry_ref = ActorRef("GlobalRegistry", "tcp://localhost:5555", zmq_sender);
 *   RegistryClient client("MyManager", registry_ref);
 *   client.start_heartbeat();
 *
 *   // Register an actor
 *   client.register_actor("MyActor", "tcp://localhost:5001");
 *
 *   // Lookup a remote actor
 *   auto endpoint = client.lookup("OtherActor");
 */
class RegistryClient {
public:
    /**
     * Create a new registry client.
     *
     * @param manager_id Unique identifier for this manager
     * @param registry_ref ActorRef to the GlobalRegistry (typically remote via ZMQ)
     */
    RegistryClient(const std::string& manager_id, ActorRef registry_ref);

    ~RegistryClient();

    // Non-copyable, movable
    RegistryClient(const RegistryClient&) = delete;
    RegistryClient& operator=(const RegistryClient&) = delete;
    RegistryClient(RegistryClient&&) = default;
    RegistryClient& operator=(RegistryClient&&) = default;

    /**
     * Start the heartbeat background thread.
     * Sends Heartbeat messages every 2 seconds to keep actors marked as online.
     */
    void start_heartbeat();

    /**
     * Stop the heartbeat background thread.
     */
    void stop_heartbeat();

    /**
     * Register an actor with the GlobalRegistry.
     *
     * @param actor_name Unique name for the actor
     * @param endpoint ZMQ endpoint where the actor can be reached
     * @throws RegistrationFailedError if registration is rejected
     * @throws TimeoutError if no response from registry
     */
    void register_actor(const std::string& actor_name, const std::string& endpoint);

    /**
     * Register an actor with the GlobalRegistry using its ActorRef.
     *
     * @param actor_name Unique name for the actor
     * @param actor_ref ActorRef for the actor (must contain endpoint info)
     * @throws RegistrationFailedError if registration is rejected
     * @throws TimeoutError if no response from registry
     */
    void register_actor(const std::string& actor_name, const ActorRef& actor_ref);

    /**
     * Lookup an actor by name.
     *
     * @param actor_name Name of the actor to find
     * @return Endpoint string for the actor
     * @throws ActorNotFoundError if actor not registered
     * @throws ActorOfflineError if actor's manager missed heartbeats
     * @throws TimeoutError if no response from registry
     */
    std::string lookup(const std::string& actor_name);

    /**
     * Lookup an actor, returning the endpoint even if offline.
     *
     * @param actor_name Name of the actor to find
     * @return Pair of (endpoint, online_status)
     * @throws ActorNotFoundError if actor not registered
     * @throws TimeoutError if no response from registry
     */
    std::pair<std::string, bool> lookup_allow_offline(const std::string& actor_name);

    /**
     * Get the manager ID.
     */
    const std::string& manager_id() const { return manager_id_; }

    /**
     * Check if heartbeat thread is running.
     */
    bool is_heartbeat_running() const { return running_.load(); }

private:
    std::string manager_id_;
    ActorRef registry_ref_;
    std::atomic<bool> running_{false};
    std::unique_ptr<std::thread> heartbeat_thread_;
    mutable std::mutex mutex_;

    void heartbeat_loop();
};

} // namespace actors::registry
