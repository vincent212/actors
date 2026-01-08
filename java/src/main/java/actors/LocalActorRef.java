/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors;

import java.util.concurrent.BlockingQueue;

/**
 * Local actor reference that delivers messages directly to a queue.
 */
public class LocalActorRef implements ActorRef {
    private final BlockingQueue<Envelope> queue;
    private final String name;

    public LocalActorRef(BlockingQueue<Envelope> queue, String name) {
        this.queue = queue;
        this.name = name;
    }

    @Override
    public String getName() {
        return name;
    }

    @Override
    public void send(Object msg, ActorRef sender) {
        queue.offer(new Envelope(msg, sender));
    }

    BlockingQueue<Envelope> getQueue() {
        return queue;
    }
}
