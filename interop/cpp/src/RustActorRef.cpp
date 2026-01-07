/*
 * RustActorRef::send() implementation
 *
 * This file implements the send() method for actors::RustActorRef.
 * It dispatches messages to Rust actors via FFI based on message ID.
 */

#include "actors/ActorRef.hpp"
#include "InteropMessages.hpp"

// Forward declare the Rust bridge function
extern "C" {
    int32_t rust_actor_send(
        const char* actor_name,
        const char* sender_name,
        int32_t msg_type,
        const void* msg_data
    );
}

namespace actors {

void RustActorRef::send(const Message* m, [[maybe_unused]] Actor* sender) {
    const char* sender_name_cstr = sender_name_.empty() ? nullptr : sender_name_.c_str();

    // Dispatch by message ID
    switch (m->get_message_id()) {
        case 1000: {  // Ping
            auto c_msg = static_cast<const msg::Ping*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1000, &c_msg);
            break;
        }
        case 1001: {  // Pong
            auto c_msg = static_cast<const msg::Pong*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1001, &c_msg);
            break;
        }
        case 1002: {  // DataRequest
            auto c_msg = static_cast<const msg::DataRequest*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1002, &c_msg);
            break;
        }
        case 1003: {  // DataResponse
            auto c_msg = static_cast<const msg::DataResponse*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1003, &c_msg);
            break;
        }
        case 1010: {  // Subscribe
            auto c_msg = static_cast<const msg::Subscribe*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1010, &c_msg);
            break;
        }
        case 1011: {  // Unsubscribe
            auto c_msg = static_cast<const msg::Unsubscribe*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1011, &c_msg);
            break;
        }
        case 1012: {  // MarketUpdate
            auto c_msg = static_cast<const msg::MarketUpdate*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1012, &c_msg);
            break;
        }
        case 1013: {  // MarketDepth
            auto c_msg = static_cast<const msg::MarketDepth*>(m)->to_c_struct();
            rust_actor_send(target_name_.c_str(), sender_name_cstr, 1013, &c_msg);
            break;
        }
        default:
            // Unknown message type - silently ignore
            break;
    }

    // Delete the message (caller expects us to take ownership)
    delete m;
}

} // namespace actors
