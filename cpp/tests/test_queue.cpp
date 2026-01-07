/*
 * Tests for BQueue class (Queue is abstract base)
 */

#include <gtest/gtest.h>
#include <thread>
#include "actors/Queue.hpp"
#include "actors/BQueue.hpp"

using namespace actors;

// Note: Queue<T> is an abstract base class - test via BQueue

TEST(BQueueTest, BasicPushPop) {
    BQueue<int> q(16);
    q.push(1);
    q.push(2);

    auto [val1, last1] = q.pop();
    EXPECT_EQ(val1, 1);
    EXPECT_FALSE(last1);

    auto [val2, last2] = q.pop();
    EXPECT_EQ(val2, 2);
    EXPECT_TRUE(last2);  // Last item
}

TEST(BQueueTest, IsEmpty) {
    BQueue<int> q(16);
    EXPECT_TRUE(q.is_empty());
    q.push(1);
    EXPECT_FALSE(q.is_empty());
}

TEST(BQueueTest, Length) {
    BQueue<int> q(16);
    EXPECT_EQ(q.length(), 0u);
    q.push(1);
    EXPECT_EQ(q.length(), 1u);
    q.push(2);
    EXPECT_EQ(q.length(), 2u);
}

TEST(BQueueTest, Peek) {
    BQueue<int> q(16);
    q.push(42);
    EXPECT_EQ(q.peek(), 42);
    EXPECT_EQ(q.length(), 1u);  // peek doesn't remove
}

TEST(BQueueTest, LastFlag) {
    BQueue<int> q(16);
    q.push(1);
    q.push(2);
    q.push(3);

    auto [val1, last1] = q.pop();
    EXPECT_EQ(val1, 1);
    EXPECT_FALSE(last1);

    auto [val2, last2] = q.pop();
    EXPECT_EQ(val2, 2);
    EXPECT_FALSE(last2);

    auto [val3, last3] = q.pop();
    EXPECT_EQ(val3, 3);
    EXPECT_TRUE(last3);  // Last item in queue
}

TEST(BQueueTest, Overflow) {
    // Small capacity, push more than capacity
    BQueue<int> q(4);
    for (int i = 0; i < 10; i++) {
        q.push(i);
    }
    EXPECT_EQ(q.length(), 10u);

    // Should get all values back in order
    for (int i = 0; i < 10; i++) {
        auto [val, last] = q.pop();
        EXPECT_EQ(val, i);
    }
}

TEST(BQueueTest, ThreadSafety) {
    BQueue<int> q(1024);
    const int count = 100;

    // Producer thread
    std::thread producer([&q, count]() {
        for (int i = 0; i < count; i++) {
            q.push(i);
        }
    });

    // Consumer thread
    std::thread consumer([&q, count]() {
        int received = 0;
        while (received < count) {
            auto [val, last] = q.pop();
            EXPECT_EQ(val, received);
            received++;
        }
    });

    producer.join();
    consumer.join();
}

TEST(BQueueTest, PolymorphicUsage) {
    // Test using through Queue<T>* pointer
    BQueue<int> bq(16);
    Queue<int>* q = &bq;

    q->push(1);
    q->push(2);

    EXPECT_EQ(q->length(), 2u);
    EXPECT_FALSE(q->is_empty());
    EXPECT_EQ(q->peek(), 1);

    auto [val, last] = q->pop();
    EXPECT_EQ(val, 1);
}
