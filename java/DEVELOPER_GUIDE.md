# Java Actors Developer Guide

This guide explains the internals of the Java Actors framework.

## Architecture Overview

The framework is built around these core concepts:

1. **Message** - Marker interface for all messages
2. **Actor** - Processes messages in its own thread
3. **ActorRef** - Mailbox address (LocalActorRef or RemoteActorRef)
4. **Manager** - Orchestrates actor lifecycle and registry
5. **Envelope** - Message wrapper with sender info

## Threading Model

Each actor runs in its own thread with its own queue:

```
┌─────────────────┐     ┌─────────────────┐
│   PingActor     │     │   PongActor     │
│   [Queue]       │     │   [Queue]       │
│   [Thread]      │     │   [Thread]      │
└────────┬────────┘     └────────┬────────┘
         │                       │
    blocks on              blocks on
    queue.poll()           queue.poll()
```

Each actor thread:
1. Polls its `BlockingQueue` for messages
2. Dispatches to the appropriate handler via reflection
3. Repeats until `stop()` is called

## ActorRef - The Mailbox Address

`ActorRef` is an interface - same API for local and remote actors:

```java
public interface ActorRef {
    String getName();
    void send(Object msg, ActorRef sender);

    default boolean isLocal() { return true; }
    default boolean isRemote() { return false; }
}
```

### LocalActorRef

Wraps a `BlockingQueue` for direct message delivery:

```java
public class LocalActorRef implements ActorRef {
    private final BlockingQueue<Envelope> queue;
    private final String name;

    @Override
    public void send(Object msg, ActorRef sender) {
        queue.offer(new Envelope(msg, sender));
    }
}
```

### RemoteActorRef

Sends messages over ZMQ to remote processes:

```java
public class RemoteActorRef implements ActorRef {
    private final String name;
    private final String endpoint;
    private final ZmqSender zmqSender;

    @Override
    public void send(Object msg, ActorRef sender) {
        zmqSender.sendTo(endpoint, name, msg, sender);
    }
}
```

**Key distinction:**
- `this` = actor's internal state
- `actorRef` = actor's mailbox address (protected field in Actor)

## Message Dispatch

Handlers are methods named `on_<ClassName>` taking an `Envelope` parameter:

```java
public class MyActor extends Actor {
    public void on_Ping(Envelope env) {
        Ping ping = (Ping) env.getMsg();
        System.out.println("Got ping: " + ping.count);
        reply(env, new Pong(ping.count));
    }

    public void on_Pong(Envelope env) {
        Pong pong = (Pong) env.getMsg();
        System.out.println("Got pong: " + pong.count);
    }
}
```

The framework uses reflection to find handlers:

```java
protected void processMessage(Envelope env) {
    Object msg = env.getMsg();
    String msgType = msg.getClass().getSimpleName();
    String methodName = "on_" + msgType;

    Method method = handlerCache.computeIfAbsent(msgType, k -> {
        try {
            Method m = this.getClass().getMethod(methodName, Envelope.class);
            m.setAccessible(true);
            return m;
        } catch (NoSuchMethodException e) {
            // Try declared method (private/protected)
            try {
                Method m = this.getClass().getDeclaredMethod(methodName, Envelope.class);
                m.setAccessible(true);
                return m;
            } catch (NoSuchMethodException e2) {
                return null;
            }
        }
    });

    if (method != null) {
        method.invoke(this, env);
    }
}
```

### Handler Cache

Methods are cached in a `ConcurrentHashMap` after first lookup:
- First call: Reflection lookup (slower)
- Subsequent calls: Direct from cache (fast)

## Envelope - Message + Metadata

```java
public class Envelope {
    private final Object msg;
    private final ActorRef sender;

    public Object getMsg() { return msg; }
    public ActorRef getSender() { return sender; }
}
```

The envelope carries:
- The message itself
- The sender's ActorRef (for `reply()`)

## Actor Base Class

```java
public abstract class Actor implements Runnable {
    protected LocalActorRef actorRef;
    protected BlockingQueue<Envelope> queue;
    protected volatile boolean running = true;
    private final Map<String, Method> handlerCache = new ConcurrentHashMap<>();

    public void init() { }   // Override for initialization
    public void end() { }    // Override for cleanup

    @Override
    public void run() {
        init();
        while (running) {
            try {
                Envelope env = queue.poll(100, TimeUnit.MILLISECONDS);
                if (env != null) {
                    processMessage(env);
                }
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                break;
            }
        }
        end();
    }

    public void reply(Envelope env, Object msg) {
        if (env.getSender() != null) {
            env.getSender().send(msg, actorRef);
        }
    }

    public void stop() {
        running = false;
    }
}
```

## Sending Messages

### send() - Async (Fire-and-Forget)

```java
// Message is queued, returns immediately
actorRef.send(new MyMessage(data), this.actorRef);
```

### reply() - Respond to Messages

```java
public void on_Request(Envelope env) {
    reply(env, new Response(42));
}
```

Reply goes to `env.getSender()` mailbox.

## Manager - Actor Registry

```java
public class Manager {
    private final Map<String, Actor> actors = new ConcurrentHashMap<>();
    private final Map<String, LocalActorRef> refs = new ConcurrentHashMap<>();
    private final List<Thread> threads = new ArrayList<>();

    public ActorRef manage(String name, Actor actor) {
        BlockingQueue<Envelope> queue = new LinkedBlockingQueue<>();
        actor.setup(queue, name);
        actors.put(name, actor);
        refs.put(name, actor.getActorRef());
        return actor.getActorRef();
    }

    public ActorRef getRef(String name) {
        return refs.get(name);
    }

    public void init() {
        // Send Start to all actors
        Start start = new Start();
        for (ActorRef ref : refs.values()) {
            ref.send(start, null);
        }
        // Spawn threads
        for (Map.Entry<String, Actor> entry : actors.entrySet()) {
            Thread t = new Thread(entry.getValue(), "actor-" + entry.getKey());
            threads.add(t);
            t.start();
        }
    }

    public void end() {
        // Send Shutdown to all
        Shutdown shutdown = new Shutdown();
        for (ActorRef ref : refs.values()) {
            ref.send(shutdown, null);
        }
        // Stop all actors
        for (Actor actor : actors.values()) {
            actor.stop();
        }
        // Join threads
        for (Thread t : threads) {
            t.join();
        }
    }
}
```

## Remote Communication

### Serialization

Messages are serialized to JSON using Gson:

```java
public class Serialization {
    private static final Gson gson = new GsonBuilder().create();
    private static final Map<String, Class<?>> registry = new ConcurrentHashMap<>();

    public static void registerMessage(Class<?> clazz) {
        RegisterMessage annotation = clazz.getAnnotation(RegisterMessage.class);
        String name = (annotation != null && !annotation.value().isEmpty())
            ? annotation.value()
            : clazz.getSimpleName();
        registry.put(name, clazz);
    }

    public static String serialize(String receiver, Object msg,
                                   String senderActor, String senderEndpoint) {
        JsonObject json = new JsonObject();
        json.addProperty("sender_actor", senderActor);
        json.addProperty("sender_endpoint", senderEndpoint);
        json.addProperty("receiver", receiver);
        json.addProperty("message_type", getTypeName(msg));
        json.add("message", gson.toJsonTree(msg));
        return gson.toJson(json);
    }
}
```

### Wire Format

```json
{
    "sender_actor": "ping",
    "sender_endpoint": "tcp://localhost:5002",
    "receiver": "pong",
    "message_type": "Ping",
    "message": {"count": 1}
}
```

### ZmqSender

Creates PUSH sockets to send messages:

```java
public class ZmqSender {
    private final ZContext context;
    private final Map<String, ZMQ.Socket> sockets = new ConcurrentHashMap<>();
    private String localEndpoint;

    public void sendTo(String endpoint, String receiver, Object msg, ActorRef sender) {
        String senderName = sender != null ? sender.getName() : null;
        String senderEndpoint = (sender instanceof RemoteActorRef)
            ? ((RemoteActorRef) sender).getEndpoint()
            : localEndpoint;

        ZMQ.Socket socket = getSocket(endpoint);
        String json = Serialization.serialize(receiver, msg, senderName, senderEndpoint);
        socket.send(json);
    }
}
```

### ZmqReceiver

Actor that bridges remote messages to local actors:

```java
public class ZmqReceiver extends Actor {
    private final String bindEndpoint;
    private final Manager manager;
    private final ZmqSender zmqSender;
    private ZMQ.Socket socket;

    @Override
    public void run() {
        init();
        while (running) {
            String msg = socket.recvStr();
            if (msg != null) {
                handleRemoteMessage(msg);
            }
            // Also check local queue
            Envelope env = queue.poll();
            if (env != null) {
                processMessage(env);
            }
        }
        end();
    }

    private void handleRemoteMessage(String json) {
        EnvelopeData data = Serialization.deserialize(json);

        // Create sender ref if provided
        ActorRef senderRef = null;
        if (data.senderActor != null && data.senderEndpoint != null) {
            senderRef = new RemoteActorRef(data.senderActor, data.senderEndpoint, zmqSender);
        }

        // Find target actor
        ActorRef targetRef = manager.getRef(data.receiver);
        if (targetRef == null) {
            // Send Reject back
            if (senderRef != null) {
                senderRef.send(new Reject(data.messageType,
                    "Unknown actor: " + data.receiver, data.receiver), null);
            }
            return;
        }

        targetRef.send(data.message, senderRef);
    }
}
```

## Complete Example

```java
import actors.*;
import actors.msg.Start;

class Ping implements Message {
    public int count;
    public Ping(int count) { this.count = count; }
}

class Pong implements Message {
    public int count;
    public Pong(int count) { this.count = count; }
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

class PongActor extends Actor {
    public void on_Ping(Envelope env) {
        Ping ping = (Ping) env.getMsg();
        reply(env, new Pong(ping.count));
    }
}

public class Main {
    public static void main(String[] args) throws Exception {
        Manager manager = new Manager();
        ActorRef pongRef = manager.manage("pong", new PongActor());
        manager.manage("ping", new PingActor(pongRef, 5));

        manager.init();
        Thread.sleep(1000);
        manager.end();
    }
}
```

## Built-in Messages

| Message | Package | Purpose |
|---------|---------|---------|
| `Start` | actors.msg | Sent to all actors on `manager.init()` |
| `Shutdown` | actors.msg | Sent to all actors on `manager.end()` |
| `Reject` | actors.msg | Sent back when remote delivery fails |

## Best Practices

1. **Keep handlers fast** - Long handlers block the actor's message queue
2. **Use `actorRef`** - Pass it as sender so others can reply
3. **Handle Shutdown** - Implement `on_Shutdown()` for cleanup
4. **Register remote messages** - Use `@RegisterMessage` and call `Serialization.registerMessage()`
5. **One actor per concern** - Don't overload actors with multiple responsibilities

## Error Handling

Remote message delivery can fail. Handle `Reject` messages:

```java
public void on_Reject(Envelope env) {
    Reject reject = (Reject) env.getMsg();
    System.err.println("Message '" + reject.messageType +
        "' rejected by '" + reject.rejectedBy + "': " + reject.reason);
}
```

## Key Files Reference

| File | Purpose |
|------|---------|
| `Actor.java` | Base actor class with reflection dispatch |
| `ActorRef.java` | Actor reference interface |
| `LocalActorRef.java` | Local queue-based reference |
| `Envelope.java` | Message wrapper with sender |
| `Manager.java` | Actor lifecycle management |
| `Message.java` | Marker interface |
| `RegisterMessage.java` | Annotation for remote messages |
| `remote/RemoteActorRef.java` | ZMQ-based remote reference |
| `remote/ZmqSender.java` | PUSH socket sender |
| `remote/ZmqReceiver.java` | PULL socket receiver actor |
| `remote/Serialization.java` | JSON serialization |
