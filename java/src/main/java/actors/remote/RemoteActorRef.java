/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors.remote;

import actors.ActorRef;

/**
 * Remote actor reference that sends messages over ZMQ.
 */
public class RemoteActorRef implements ActorRef {
    private final String name;
    private final String endpoint;
    private final ZmqSender zmqSender;

    public RemoteActorRef(String name, String endpoint, ZmqSender zmqSender) {
        this.name = name;
        this.endpoint = endpoint;
        this.zmqSender = zmqSender;
    }

    @Override
    public String getName() {
        return name;
    }

    public String getEndpoint() {
        return endpoint;
    }

    @Override
    public void send(Object msg, ActorRef sender) {
        zmqSender.sendTo(endpoint, name, msg, sender);
    }

    @Override
    public boolean isLocal() {
        return false;
    }

    @Override
    public boolean isRemote() {
        return true;
    }
}
