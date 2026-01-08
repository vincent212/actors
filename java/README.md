# Actors - Actor Framework for Java

A lightweight actor framework for building concurrent systems in Java.

## Overview

This is part of a **multi-language actor framework** spanning C++, Rust, Python, and Java. All implementations share a common JSON-over-ZMQ wire protocol, enabling seamless cross-language communication.

The Java implementation brings the actor model to the JVM ecosystem, enabling integration with Java's rich enterprise and data processing libraries.

**When to use Java**: Enterprise systems, big data pipelines, Android applications, or when you need JVM ecosystem integration (Spring, Kafka, Hadoop, etc.).

## Features

- **Actor Model**: Independent entities processing messages sequentially
- **Message-Driven**: All communication via message passing
- **Thread-Safe**: Each actor runs in its own thread with isolated state
- **Remote Actors**: ZeroMQ-based communication with other processes/languages
- **Reflection-Based Dispatch**: Name handlers `on_<MessageType>`, no annotations needed

## Quick Start

### 1. Define Messages

```java
// Messages are plain Java classes implementing Message interface
public class Ping implements Message {
    public int count;

    public Ping(int count) {
        this.count = count;
    }
}

public class Pong implements Message {
    public int count;

    public Pong(int count) {
        this.count = count;
    }
}
```

### 2. Create Actors

```java
import actors.*;

class PongActor extends Actor {
    public void on_Ping(Envelope env) {
        Ping ping = (Ping) env.getMsg();
        System.out.println("Received ping " + ping.count);
        reply(env, new Pong(ping.count));
    }
}

class PingActor extends Actor {
    private final ActorRef pongRef;
    private final int maxCount;

    public PingActor(ActorRef pongRef, int maxCount) {
        this.pongRef = pongRef;
        this.maxCount = maxCount;
    }

    public void on_Start(Envelope env) {
        pongRef.send(new Ping(1), actorRef);
    }

    public void on_Pong(Envelope env) {
        Pong pong = (Pong) env.getMsg();
        if (pong.count < maxCount) {
            pongRef.send(new Ping(pong.count + 1), actorRef);
        } else {
            stop();
        }
    }
}
```

Handler methods are named `on_<ClassName>`. The framework uses reflection to dispatch messages automatically.

### 3. Set Up Manager

```java
import actors.Manager;

public class Main {
    public static void main(String[] args) throws Exception {
        Manager manager = new Manager();

        ActorRef pongRef = manager.manage("pong", new PongActor());
        manager.manage("ping", new PingActor(pongRef, 5));

        manager.init();   // Start all actors, send Start message
        Thread.sleep(1000);
        manager.end();    // Send Shutdown, wait for threads
    }
}
```

## Core Concepts

### Actor
Base class for all actors. Implement `on_<MessageType>` methods to handle messages.

### Message
Marker interface for all messages. For remote communication, use `@RegisterMessage` annotation.

### Manager
Manages actor lifecycle, thread creation, and provides a registry for name-based lookup.

### ActorRef
Handle for sending messages to an actor. Can be `LocalActorRef` or `RemoteActorRef`.

### Envelope
Wraps a message with sender info, enabling replies.

## Messaging

### Send (Fire-and-Forget)
```java
otherRef.send(new MyMessage(data), actorRef);
```
Message is queued and processed later by the receiver's thread.

### Reply
```java
public void on_Request(Envelope env) {
    reply(env, new Response(42));
}
```

## Remote Actors (Cross-Language)

### Setup Remote Communication

```java
import actors.remote.*;

// Register messages for remote serialization
@RegisterMessage
public class Ping implements Message {
    public int count;
    // ...
}

// Setup
Manager manager = new Manager();
ZmqSender zmqSender = new ZmqSender("tcp://localhost:5001");

// Create reference to remote actor (e.g., Python or Rust actor)
RemoteActorRef remotePong = zmqSender.remoteRef("pong", "tcp://localhost:5002");

// Create receiver for incoming remote messages
ZmqReceiver receiver = new ZmqReceiver("tcp://0.0.0.0:5001", manager, zmqSender);
manager.manage("zmq_receiver", receiver);
```

### Wire Protocol

All implementations use JSON-over-ZMQ:

```json
{
  "message_type": "Ping",
  "receiver": "pong_actor",
  "sender_actor": "ping_actor",
  "sender_endpoint": "tcp://localhost:5001",
  "message": {"count": 42}
}
```

## Built-in Messages

- **Start**: Sent to all actors when `manager.init()` is called
- **Shutdown**: Sent to all actors when `manager.end()` is called
- **Reject**: Sent back when a remote message cannot be delivered

## Building

```bash
mvn compile
```

### Run Examples

```bash
# Local ping-pong
mvn exec:java -Dexec.mainClass=examples.PingPong

# Remote ping (requires Python/Rust pong running on port 5001)
mvn exec:java -Dexec.mainClass=examples.RemotePingProcess
```

### Requirements

- Java 11+
- Maven
- JeroMQ (ZeroMQ for Java)
- Gson (JSON serialization)

## Files

```
src/main/java/
  actors/
    Actor.java           - Base actor class
    ActorRef.java        - Actor reference interface
    LocalActorRef.java   - Local actor reference
    Envelope.java        - Message wrapper
    Manager.java         - Actor lifecycle manager
    Message.java         - Message marker interface
    RegisterMessage.java - Annotation for remote messages
    msg/
      Start.java         - Start message
      Shutdown.java      - Shutdown message
      Reject.java        - Rejection message
    remote/
      RemoteActorRef.java   - Remote actor reference
      ZmqSender.java        - ZMQ sender
      ZmqReceiver.java      - ZMQ receiver
      Serialization.java    - JSON serialization

  examples/
    Ping.java, Pong.java           - Message types
    PingPong.java                  - Local example
    RemotePingProcess.java         - Remote ping
    RemotePongProcess.java         - Remote pong
```

## License

MIT License

Copyright 2025 Vincent Maciejewski & M2 Tech
