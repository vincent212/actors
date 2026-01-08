# Actors Framework - Developer Guide

## Table of Contents
1. [What is Actors?](#what-is-actors)
2. [Core Architecture](#core-architecture)
3. [Message System](#message-system)
4. [Handler Registration](#handler-registration)
5. [send() and reply()](#send-and-reply)
6. [Queue Implementation](#queue-implementation)
7. [Complete Working Example](#complete-working-example)
8. [Best Practices](#best-practices)
9. [Key Files Reference](#key-files-reference)

---

## What is Actors?

Actors is a **high-performance actor framework** designed for ultra-low-latency concurrent systems, particularly trading systems.

### Core Principles

- **Actor Model**: Each Actor is an independent entity processing messages sequentially
- **Message-Driven**: All communication via typed message passing
- **Thread-Safe**: Each Actor runs in its own thread with isolated state
- **Zero-Copy**: Messages are passed by pointer, not copied

### When to Use Actors

**Good Use Cases**:
- Real-time trading systems (order routing, market making)
- High-frequency market data processing
- Low-latency event-driven systems
- Microservice-style architectures within a single process

**Not Ideal For**:
- Simple request/response APIs (use REST/gRPC)
- Batch processing (use task queues)
- Shared-memory parallel algorithms (use thread pools)

---

## Core Architecture

### Actor Base Class

**File**: `include/actors/Actor.hpp`

```cpp
class Actor
{
protected:
  // Message queue
  Queue<const Message *> *msgq;

  // Handler management
  std::map<std::type_index, generic_handler_t> handlers;
  std::vector<generic_handler_t> handler_cache;  // 512 slots
  std::vector<bool> dont_have_handler;           // Cache misses

  // State
  bool terminated = false;
  Actor *group = nullptr;    // Parent group if member
  Actor *reply_to = nullptr; // Return address for async replies

public:
  Actor();

  // Message sending
  void send(const Message *m, Actor *sender) noexcept;

  // Reply mechanism
  void reply(const Message *m) noexcept;

  // Main processing loop (run in dedicated thread)
  void operator()() noexcept;
};
```

### Key Member Variables

| Variable | Purpose |
|---|---|
| `msgq` | Message queue (blocking queue) |
| `handlers` | Type-indexed map of handler function pointers |
| `handler_cache[512]` | Fast lookup cache by message ID |
| `dont_have_handler[512]` | Tracks cache misses to avoid repeated lookups |
| `msg_cnt` | Total messages processed by this Actor |
| `affinity` | CPU core binding (set via Manager) |
| `priority` | Thread priority (SCHED_FIFO, SCHED_RR, etc.) |

---

## Message System

### Base Message Class

**File**: `include/actors/Message.hpp`

```cpp
struct Message
{
  virtual int get_message_id() const = 0;
  mutable Actor *sender;      // Who sent this
  mutable Actor *destination; // Where it's going
  mutable bool last;          // Is this the last message?
};
```

### Creating Custom Messages

Use the `Message_N<ID>` template where ID is 0-511 for optimal caching:

```cpp
// MyMessages.hpp
#pragma once
#include "actors/Message.hpp"

namespace myapp::msg {

  // Simple message with data
  struct PriceUpdate : public actors::Message_N<100> {
    double price;
    int64_t timestamp;

    PriceUpdate(double p, int64_t ts) : price(p), timestamp(ts) {}
  };

  // Request message
  struct GetPosition : public actors::Message_N<101> {
    std::string symbol;
    GetPosition(const std::string &s) : symbol(s) {}
  };

  // Response message
  struct PositionResponse : public actors::Message_N<102> {
    std::string symbol;
    double quantity;
    double avg_price;

    PositionResponse(const std::string &s, double q, double ap)
      : symbol(s), quantity(q), avg_price(ap) {}
  };
}
```

### Built-in Messages

| Message Type | ID | Usage |
|---|---|---|
| `actors::msg::Start` | 6 | Sent to Actors on init |
| `actors::msg::Shutdown` | 5 | Graceful termination signal |
| `actors::msg::Continue` | 1 | Self-continuation pattern |
| `actors::msg::Timeout` | 8 | Timer expiration |
| `actors::msg::Subscribe` | 7 | Subscribe to events |

---

## Handler Registration

### The MESSAGE_HANDLER Macro

**Defined in**: `include/actors/Actor.hpp`

```cpp
#define MESSAGE_HANDLER(message_type, function_name)                            \
{                                                                               \
  typedef typename std::remove_reference<decltype(*this)>::type ActorT;        \
  actors::register_handler<ActorT, message_type>(this)(&ActorT::function_name); \
}
```

### How It Works

1. **Type extraction**: Gets the Actor subclass type from `decltype(*this)`
2. **Instantiates** `register_handler<MyClass, MessageType>`
3. **Type-erases** the member function pointer to `void*`
4. **Stores** in `handlers` map with `std::type_index(typeid(MsgT))` as key

### Registration Pattern

**Always register handlers in the constructor**:

```cpp
class OrderManager : public actors::Actor {
public:
  OrderManager() {
    strncpy(name, "OrderManager", sizeof(name));

    // Register all handlers here
    MESSAGE_HANDLER(actors::msg::Start, start_handler);
    MESSAGE_HANDLER(actors::msg::Shutdown, shutdown_handler);
    MESSAGE_HANDLER(msg::NewOrder, new_order_handler);
    MESSAGE_HANDLER(msg::CancelOrder, cancel_order_handler);
  }

  // Handler declarations
  void start_handler(const actors::msg::Start *) noexcept;
  void shutdown_handler(const actors::msg::Shutdown *) noexcept;
  void new_order_handler(const msg::NewOrder *m) noexcept;
  void cancel_order_handler(const msg::CancelOrder *m) noexcept;
};
```

### Handler Lookup

**Performance**: First call is O(log n) lookup, all subsequent calls are O(1) cache hits.

```cpp
bool Actor::call_handler(const Message *m) noexcept
{
  // Fast path: check handler cache
  auto id = m->get_message_id();
  auto f0 = handler_cache[id];
  if (f0) {
    (this->*f0)(m);  // Direct call (fastest)
    return true;
  }

  // Check if we know there's no handler
  if (dont_have_handler[id]) {
    return false;  // Skip expensive lookup
  }

  // Slow path: lookup by type_index
  auto midx = std::type_index(typeid(*m));
  auto p = handlers.find(midx);
  if (p == handlers.end()) {
    dont_have_handler[id] = true;  // Cache the miss
    return false;
  }

  // Found: call and cache for next time
  auto f = p->second;
  (this->*f)(m);
  handler_cache[id] = f;  // Cache for future calls
  return true;
}
```

---

## send() and reply()

### send() - Asynchronous Message Passing

**Usage**: Fire-and-forget, no immediate response needed

```cpp
void Actor::send(const Message *m, Actor *sender) noexcept
{
  if (terminated) return;

  m->sender = sender;
  m->destination = this;

  // Add to message queue
  msgq->push(m);
}
```

**Characteristics**:
- **Asynchronous**: Returns immediately
- **Queued**: Message goes into queue
- **Thread-safe**: Safe to call from any thread
- **Memory**: Must use `new`; Actor deletes after processing

**Example**:

```cpp
// Send notification (fire-and-forget)
market_data->send(new msg::PriceUpdate(99.5, timestamp), this);

// The Actor will delete the message after processing
```

### reply() - Respond to Messages

```cpp
void Actor::reply(const Message *m) noexcept
{
  if (reply_to) {
    reply_to->send(m, this);
  }
}
```

Use `reply()` to send a response back to the sender:

```cpp
void get_position_handler(const msg::GetPosition *m) noexcept {
  auto it = positions.find(m->symbol);
  if (it != positions.end()) {
    reply(new msg::PositionInfo(m->symbol, it->second, 0));
  }
}
```

---

## Queue Implementation

### BQueue (Blocking Queue)

**File**: `include/actors/BQueue.hpp`

```cpp
template <class T>
class BQueue : public Queue<T>
{
  std::mutex mut;
  std::condition_variable cv;
  boost::circular_buffer<T> cb_;  // Fast path (default 64)
  std::deque<T> overflow_;        // Overflow storage

public:
  std::tuple<T, bool> pop() noexcept {
    std::unique_lock<std::mutex> lock(mut);
    cv.wait(lock, [this]() {
      return !cb_.empty() || !overflow_.empty();
    });
    // Pop from cb_ first, then overflow_
  }

  void push(const T &x) noexcept {
    // Push to cb_ if space, else overflow_
    cv.notify_one();
  }
};
```

**Characteristics**:
- **Low CPU usage**: Sleeps when empty
- **Overflow handling**: Deque for large bursts
- **Default size**: 64-element circular buffer

---

## Complete Working Example

### Step 1: Define Messages

```cpp
// File: myapp/msg/Messages.hpp
#pragma once
#include "actors/Message.hpp"
#include <string>

namespace myapp::msg {

  struct GetPosition : public actors::Message_N<100> {
    std::string symbol;
    GetPosition(const std::string &s) : symbol(s) {}
  };

  struct PositionInfo : public actors::Message_N<101> {
    std::string symbol;
    double quantity;
    double avg_price;

    PositionInfo(const std::string &s, double q, double p)
      : symbol(s), quantity(q), avg_price(p) {}
  };

  struct Trade : public actors::Message_N<102> {
    std::string symbol;
    double quantity;
    double price;

    Trade(const std::string &s, double q, double p)
      : symbol(s), quantity(q), price(p) {}
  };
}
```

### Step 2: Create Position Manager

```cpp
// File: myapp/act/PositionManager.hpp
#pragma once
#include "actors/Actor.hpp"
#include "actors/msg/Start.hpp"
#include "actors/msg/Shutdown.hpp"
#include "myapp/msg/Messages.hpp"
#include <map>

namespace myapp::act {

  class PositionManager : public actors::Actor {
  private:
    std::map<std::string, double> positions;

  public:
    PositionManager() {
      strncpy(name, "PositionManager", sizeof(name));

      MESSAGE_HANDLER(actors::msg::Start, start_handler);
      MESSAGE_HANDLER(actors::msg::Shutdown, shutdown_handler);
      MESSAGE_HANDLER(msg::GetPosition, get_position_handler);
      MESSAGE_HANDLER(msg::Trade, trade_handler);
    }

    void start_handler(const actors::msg::Start *) noexcept {
      std::cout << "PositionManager starting..." << std::endl;
    }

    void shutdown_handler(const actors::msg::Shutdown *) noexcept {
      std::cout << "PositionManager shutting down..." << std::endl;
    }

    void get_position_handler(const msg::GetPosition *m) noexcept {
      auto it = positions.find(m->symbol);
      if (it != positions.end()) {
        reply(new msg::PositionInfo(m->symbol, it->second, 0));
      } else {
        reply(new msg::PositionInfo(m->symbol, 0, 0));
      }
    }

    void trade_handler(const msg::Trade *m) noexcept {
      positions[m->symbol] += m->quantity;
      std::cout << "Trade: " << m->symbol << " qty=" << m->quantity << std::endl;
    }
  };
}
```

### Step 3: Main Application

```cpp
// File: main.cpp
#include "actors/act/Manager.hpp"
#include "myapp/act/PositionManager.hpp"
#include <thread>

class MyManager : public actors::Manager {
public:
  MyManager() {
    auto* pos_mgr = new myapp::act::PositionManager();
    manage(pos_mgr);
  }
};

int main() {
  MyManager mgr;
  mgr.init();   // Start all actors
  mgr.end();    // Wait for completion
  return 0;
}
```

### Build

```bash
g++ -std=c++20 -O2 -I./include main.cpp -L./src -lactors -lpthread -o myapp
```

---

## Best Practices

### 1. Handler Registration

**DO**: Register all handlers in constructor
```cpp
MyActor() {
  MESSAGE_HANDLER(msg::Type1, handler1);
  MESSAGE_HANDLER(msg::Type2, handler2);
}
```

**DON'T**: Register handlers at runtime (not thread-safe)

### 2. Message IDs

**DO**: Use sequential IDs 0-511 for cache efficiency
```cpp
struct MyMsg1 : public actors::Message_N<100> { ... };
struct MyMsg2 : public actors::Message_N<101> { ... };
```

**DON'T**: Use random IDs > 511 (cache miss)

### 3. Handler Signatures

**DO**: Make handlers noexcept
```cpp
void my_handler(const msg::MyMsg *m) noexcept {
  // Process message
}
```

**DON'T**: Throw exceptions (will crash)

### 4. Memory Management

**DO**: Use new for send(), let Actor delete
```cpp
other->send(new msg::Trade(...), this);
// Actor owns it now, will delete automatically
```

**DON'T**: Keep pointer or delete yourself (double-free)

### 5. Actor Isolation

**DO**: Communicate only via messages

**DON'T**: Share mutable state between actors

---

## Key Files Reference

| File | Purpose |
|---|---|
| `include/actors/Actor.hpp` | Core Actor class and MESSAGE_HANDLER macro |
| `include/actors/Message.hpp` | Message base classes |
| `src/Actor.cpp` | send(), reply(), operator() implementation |
| `include/actors/act/Manager.hpp` | Actor lifecycle management |
| `include/actors/act/Group.hpp` | Multi-actor single-thread container |
| `include/actors/act/Timer.hpp` | Timer utilities |
| `include/actors/BQueue.hpp` | Blocking queue |
| `include/actors/Queue.hpp` | Queue interface |
| `examples/ping_pong.cpp` | Working example |

---

**Version**: 2.0 (actors library)
**Date**: 2025-12-23
