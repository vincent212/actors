/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors;

import java.lang.reflect.Method;
import java.util.Map;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.TimeUnit;

/**
 * Base class for all actors. Subclasses define message handlers as methods
 * named on_<MessageType> that take an Envelope parameter.
 */
public abstract class Actor implements Runnable {
    protected LocalActorRef actorRef;
    protected BlockingQueue<Envelope> queue;
    protected volatile boolean running = true;
    private final Map<String, Method> handlerCache = new ConcurrentHashMap<>();

    public void init() {
        // Override to perform initialization
    }

    public void end() {
        // Override to perform cleanup
    }

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
            try {
                method.invoke(this, env);
            } catch (Exception e) {
                System.err.println("Error in " + methodName + ": " + e.getCause().getMessage());
                e.getCause().printStackTrace();
            }
        }
    }

    public void reply(Envelope env, Object msg) {
        if (env.getSender() != null) {
            env.getSender().send(msg, actorRef);
        }
    }

    public LocalActorRef getActorRef() {
        return actorRef;
    }

    public void stop() {
        running = false;
    }

    void setup(BlockingQueue<Envelope> queue, String name) {
        this.queue = queue;
        this.actorRef = new LocalActorRef(queue, name);
    }
}
