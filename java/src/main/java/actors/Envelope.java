/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors;

/**
 * Envelope wraps a message with sender information for reply routing.
 */
public class Envelope {
    private final Object msg;
    private final ActorRef sender;

    public Envelope(Object msg, ActorRef sender) {
        this.msg = msg;
        this.sender = sender;
    }

    public Object getMsg() {
        return msg;
    }

    public ActorRef getSender() {
        return sender;
    }
}
