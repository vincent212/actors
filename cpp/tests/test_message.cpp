/*
 * Tests for Message classes
 */

#include <gtest/gtest.h>
#include "actors/Message.hpp"
#include "actors/msg/Start.hpp"
#include "actors/msg/Shutdown.hpp"
#include "actors/msg/Timeout.hpp"

using namespace actors;

// Test custom message with ID
struct TestMessage : public Message_N<100> {
    int value;
    TestMessage(int v = 0) : value(v) {}
};

struct AnotherMessage : public Message_N<200> {
    std::string text;
    AnotherMessage(const std::string& t = "") : text(t) {}
};

TEST(MessageTest, MessageIdTemplate) {
    TestMessage msg;
    EXPECT_EQ(msg.get_message_id(), 100);
}

TEST(MessageTest, DifferentMessageIds) {
    TestMessage msg1;
    AnotherMessage msg2;
    EXPECT_NE(msg1.get_message_id(), msg2.get_message_id());
}

TEST(MessageTest, MessageWithData) {
    TestMessage msg(42);
    EXPECT_EQ(msg.value, 42);
    EXPECT_EQ(msg.get_message_id(), 100);
}

TEST(MessageTest, MessageDefaultFields) {
    TestMessage msg;
    EXPECT_EQ(msg.sender, nullptr);
    EXPECT_EQ(msg.destination, nullptr);
    EXPECT_FALSE(msg.is_fast);
    EXPECT_FALSE(msg.last);
}

TEST(MessageTest, StartMessageId) {
    msg::Start start;
    EXPECT_EQ(start.get_message_id(), 6);
}

TEST(MessageTest, ShutdownMessageId) {
    msg::Shutdown shutdown;
    EXPECT_EQ(shutdown.get_message_id(), 5);
}

TEST(MessageTest, TimeoutMessageId) {
    msg::Timeout timeout;
    EXPECT_EQ(timeout.get_message_id(), 8);
}

TEST(MessageTest, TimeoutWithData) {
    msg::Timeout timeout(123);
    EXPECT_EQ(timeout.get_message_id(), 8);
    EXPECT_EQ(timeout.data, 123);
}

TEST(MessageTest, MessageCopy) {
    TestMessage original(42);
    original.is_fast = true;
    original.last = true;

    TestMessage copy(original);
    EXPECT_EQ(copy.value, 42);
    EXPECT_EQ(copy.is_fast, true);
    EXPECT_EQ(copy.last, true);
    // destination should be reset on copy
    EXPECT_EQ(copy.destination, nullptr);
}
