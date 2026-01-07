/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech

*/

#include "actors/registry/RegistryClient.hpp"
#include <iostream>

namespace actors::registry {

RegistryClient::RegistryClient(const std::string& manager_id, ActorRef registry_ref)
    : manager_id_(manager_id)
    , registry_ref_(std::move(registry_ref))
{
}

RegistryClient::~RegistryClient() {
    stop_heartbeat();
}

void RegistryClient::start_heartbeat() {
    std::lock_guard<std::mutex> lock(mutex_);

    if (running_.load()) {
        return;  // Already running
    }

    running_.store(true);

    heartbeat_thread_ = std::make_unique<std::thread>([this]() {
        heartbeat_loop();
    });
}

void RegistryClient::stop_heartbeat() {
    running_.store(false);

    if (heartbeat_thread_ && heartbeat_thread_->joinable()) {
        heartbeat_thread_->join();
    }
    heartbeat_thread_.reset();
}

void RegistryClient::heartbeat_loop() {
    while (running_.load()) {
        try {
            Heartbeat hb(manager_id_);
            registry_ref_.send(new Heartbeat(hb), nullptr);
        } catch (const std::exception& e) {
            std::cerr << "RegistryClient: heartbeat failed: " << e.what() << std::endl;
        }

        // Sleep for 2 seconds
        std::this_thread::sleep_for(std::chrono::seconds(2));
    }
}

void RegistryClient::register_actor(const std::string& actor_name, const std::string& endpoint) {
    RegisterActor msg(manager_id_, actor_name, ActorRef());
    // For remote registration, we pass endpoint as string in a modified message
    // Since C++ RegisterActor uses ActorRef, we need to handle endpoint differently

    // Create a registration message
    // Note: The Python GlobalRegistry expects actor_endpoint as a string
    // For C++ -> Python communication, we'll need serialization
    // For now, use fast_send with local registry

    auto reply = registry_ref_.fast_send(new RegisterActor(msg), nullptr);

    if (!reply) {
        throw TimeoutError("No response from registry for registration");
    }

    if (dynamic_cast<const RegistrationOk*>(reply.get())) {
        return;  // Success
    }

    if (auto* failed = dynamic_cast<const RegistrationFailed*>(reply.get())) {
        throw RegistrationFailedError(failed->actor_name, failed->reason);
    }

    throw RegistryError("Unexpected response type from registry");
}

void RegistryClient::register_actor(const std::string& actor_name, const ActorRef& actor_ref) {
    RegisterActor msg(manager_id_, actor_name, actor_ref);

    auto reply = registry_ref_.fast_send(new RegisterActor(msg), nullptr);

    if (!reply) {
        throw TimeoutError("No response from registry for registration");
    }

    if (dynamic_cast<const RegistrationOk*>(reply.get())) {
        return;  // Success
    }

    if (auto* failed = dynamic_cast<const RegistrationFailed*>(reply.get())) {
        throw RegistrationFailedError(failed->actor_name, failed->reason);
    }

    throw RegistryError("Unexpected response type from registry");
}

std::string RegistryClient::lookup(const std::string& actor_name) {
    LookupActor msg(actor_name);

    auto reply = registry_ref_.fast_send(new LookupActor(msg), nullptr);

    if (!reply) {
        throw TimeoutError("No response from registry for lookup");
    }

    if (auto* result = dynamic_cast<const LookupResult*>(reply.get())) {
        if (!result->actor_ref.has_value()) {
            throw ActorNotFoundError(actor_name);
        }

        if (!result->online) {
            throw ActorOfflineError(actor_name);
        }

        // Return the endpoint from the ActorRef if it's remote
        const auto& ref = result->actor_ref.value();
        if (ref.is_remote()) {
            return ref.remote_ref().endpoint();
        }
        // For local refs, return empty string (caller should use get_actor_by_name)
        return "";
    }

    throw RegistryError("Unexpected response type from registry");
}

std::pair<std::string, bool> RegistryClient::lookup_allow_offline(const std::string& actor_name) {
    LookupActor msg(actor_name);

    auto reply = registry_ref_.fast_send(new LookupActor(msg), nullptr);

    if (!reply) {
        throw TimeoutError("No response from registry for lookup");
    }

    if (auto* result = dynamic_cast<const LookupResult*>(reply.get())) {
        if (!result->actor_ref.has_value()) {
            throw ActorNotFoundError(actor_name);
        }

        const auto& ref = result->actor_ref.value();
        std::string endpoint;
        if (ref.is_remote()) {
            endpoint = ref.remote_ref().endpoint();
        }
        return {endpoint, result->online};
    }

    throw RegistryError("Unexpected response type from registry");
}

} // namespace actors::registry
