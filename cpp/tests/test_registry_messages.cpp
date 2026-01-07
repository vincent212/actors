/*
 * Tests for Registry messages
 */

#include <gtest/gtest.h>
#include <set>
#include "actors/registry/RegistryMessages.hpp"

using namespace actors::registry;

TEST(RegistryMessagesTest, RegisterActorMessageId) {
    RegisterActor msg;
    EXPECT_EQ(msg.get_message_id(), MSG_REGISTER_ACTOR);
    EXPECT_EQ(msg.get_message_id(), 900);
}

TEST(RegistryMessagesTest, RegisterActorWithData) {
    RegisterActor msg("mgr1", "pong", actors::ActorRef());
    EXPECT_EQ(msg.manager_id, "mgr1");
    EXPECT_EQ(msg.actor_name, "pong");
}

TEST(RegistryMessagesTest, UnregisterActorMessageId) {
    UnregisterActor msg;
    EXPECT_EQ(msg.get_message_id(), MSG_UNREGISTER_ACTOR);
    EXPECT_EQ(msg.get_message_id(), 901);
}

TEST(RegistryMessagesTest, UnregisterActorWithData) {
    UnregisterActor msg("pong");
    EXPECT_EQ(msg.actor_name, "pong");
}

TEST(RegistryMessagesTest, RegistrationOkMessageId) {
    RegistrationOk msg;
    EXPECT_EQ(msg.get_message_id(), MSG_REGISTRATION_OK);
    EXPECT_EQ(msg.get_message_id(), 902);
}

TEST(RegistryMessagesTest, RegistrationOkWithData) {
    RegistrationOk msg("pong");
    EXPECT_EQ(msg.actor_name, "pong");
}

TEST(RegistryMessagesTest, RegistrationFailedMessageId) {
    RegistrationFailed msg;
    EXPECT_EQ(msg.get_message_id(), MSG_REGISTRATION_FAILED);
    EXPECT_EQ(msg.get_message_id(), 903);
}

TEST(RegistryMessagesTest, RegistrationFailedWithData) {
    RegistrationFailed msg("pong", "Name already registered");
    EXPECT_EQ(msg.actor_name, "pong");
    EXPECT_EQ(msg.reason, "Name already registered");
}

TEST(RegistryMessagesTest, LookupActorMessageId) {
    LookupActor msg;
    EXPECT_EQ(msg.get_message_id(), MSG_LOOKUP_ACTOR);
    EXPECT_EQ(msg.get_message_id(), 904);
}

TEST(RegistryMessagesTest, LookupActorWithData) {
    LookupActor msg("pong");
    EXPECT_EQ(msg.actor_name, "pong");
}

TEST(RegistryMessagesTest, LookupResultMessageId) {
    LookupResult msg;
    EXPECT_EQ(msg.get_message_id(), MSG_LOOKUP_RESULT);
    EXPECT_EQ(msg.get_message_id(), 905);
}

TEST(RegistryMessagesTest, LookupResultDefault) {
    LookupResult msg;
    EXPECT_FALSE(msg.online);
    EXPECT_FALSE(msg.actor_ref.has_value());
}

TEST(RegistryMessagesTest, HeartbeatMessageId) {
    Heartbeat msg;
    EXPECT_EQ(msg.get_message_id(), MSG_HEARTBEAT);
    EXPECT_EQ(msg.get_message_id(), 906);
}

TEST(RegistryMessagesTest, HeartbeatWithManagerId) {
    Heartbeat msg("mgr1");
    EXPECT_EQ(msg.manager_id, "mgr1");
    EXPECT_GT(msg.timestamp, 0ULL);
}

TEST(RegistryMessagesTest, HeartbeatAckMessageId) {
    HeartbeatAck msg;
    EXPECT_EQ(msg.get_message_id(), MSG_HEARTBEAT_ACK);
    EXPECT_EQ(msg.get_message_id(), 907);
}

TEST(RegistryMessagesTest, AllMessageIdsUnique) {
    // Verify all registry message IDs are unique
    RegisterActor reg;
    UnregisterActor unreg;
    RegistrationOk ok;
    RegistrationFailed fail;
    LookupActor lookup;
    LookupResult result;
    Heartbeat hb;
    HeartbeatAck hback;

    std::set<int> ids = {
        reg.get_message_id(),
        unreg.get_message_id(),
        ok.get_message_id(),
        fail.get_message_id(),
        lookup.get_message_id(),
        result.get_message_id(),
        hb.get_message_id(),
        hback.get_message_id()
    };

    EXPECT_EQ(ids.size(), 8u);  // All unique
}
