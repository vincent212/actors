/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors.remote;

import actors.ActorRef;
import org.zeromq.SocketType;
import org.zeromq.ZContext;
import org.zeromq.ZMQ;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * ZMQ sender for remote message passing using PUSH sockets.
 */
public class ZmqSender {
    private final ZContext context;
    private final Map<String, ZMQ.Socket> sockets = new ConcurrentHashMap<>();
    private String localEndpoint;

    public ZmqSender() {
        this(null);
    }

    public ZmqSender(String localEndpoint) {
        this.context = new ZContext();
        this.localEndpoint = localEndpoint;
    }

    public void setLocalEndpoint(String localEndpoint) {
        this.localEndpoint = localEndpoint;
    }

    public String getLocalEndpoint() {
        return localEndpoint;
    }

    private synchronized ZMQ.Socket getSocket(String endpoint) {
        return sockets.computeIfAbsent(endpoint, ep -> {
            ZMQ.Socket socket = context.createSocket(SocketType.PUSH);
            socket.connect(ep);
            return socket;
        });
    }

    public void sendTo(String endpoint, String receiver, Object msg, ActorRef sender) {
        String senderName = sender != null ? sender.getName() : null;
        String senderEndpoint;
        if (sender instanceof RemoteActorRef) {
            senderEndpoint = ((RemoteActorRef) sender).getEndpoint();
        } else {
            senderEndpoint = localEndpoint;
        }

        ZMQ.Socket socket = getSocket(endpoint);
        String json = Serialization.serialize(receiver, msg, senderName, senderEndpoint);
        socket.send(json);
    }

    public RemoteActorRef remoteRef(String name, String endpoint) {
        return new RemoteActorRef(name, endpoint, this);
    }

    public void close() {
        for (ZMQ.Socket socket : sockets.values()) {
            socket.close();
        }
        sockets.clear();
        context.close();
    }
}
