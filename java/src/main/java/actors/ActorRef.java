/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors;

/**
 * Reference to an actor that can receive messages.
 */
public interface ActorRef {
    String getName();
    void send(Object msg, ActorRef sender);

    default boolean isLocal() {
        return true;
    }

    default boolean isRemote() {
        return false;
    }
}
