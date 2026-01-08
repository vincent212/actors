/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors;

import actors.msg.Shutdown;
import actors.msg.Start;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.LinkedBlockingQueue;

/**
 * Manager creates and manages actors, handling their lifecycle.
 */
public class Manager {
    private final Map<String, Actor> actors = new ConcurrentHashMap<>();
    private final Map<String, LocalActorRef> refs = new ConcurrentHashMap<>();
    private final List<Thread> threads = new ArrayList<>();
    private volatile boolean initialized = false;

    public ActorRef manage(String name, Actor actor) {
        if (actors.containsKey(name)) {
            throw new IllegalArgumentException("Actor already exists: " + name);
        }
        LinkedBlockingQueue<Envelope> queue = new LinkedBlockingQueue<>();
        actor.setup(queue, name);
        actors.put(name, actor);
        refs.put(name, actor.getActorRef());
        return actor.getActorRef();
    }

    public ActorRef getRef(String name) {
        return refs.get(name);
    }

    public void init() {
        if (initialized) {
            throw new IllegalStateException("Manager already initialized");
        }
        initialized = true;

        Start start = new Start();
        for (ActorRef ref : refs.values()) {
            ref.send(start, null);
        }

        for (Map.Entry<String, Actor> entry : actors.entrySet()) {
            Thread t = new Thread(entry.getValue(), "actor-" + entry.getKey());
            threads.add(t);
            t.start();
        }
    }

    public void end() {
        Shutdown shutdown = new Shutdown();
        for (ActorRef ref : refs.values()) {
            ref.send(shutdown, null);
        }

        for (Actor actor : actors.values()) {
            actor.stop();
        }

        for (Thread t : threads) {
            try {
                t.join();
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
        }
    }

    public List<String> getManagedNames() {
        return new ArrayList<>(actors.keySet());
    }
}
