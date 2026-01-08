/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors.remote;

import actors.Actor;
import actors.ActorRef;
import actors.Envelope;
import actors.Manager;
import actors.msg.Reject;
import org.zeromq.SocketType;
import org.zeromq.ZContext;
import org.zeromq.ZMQ;

/**
 * ZMQ receiver actor that bridges remote messages to local actors.
 */
public class ZmqReceiver extends Actor {
    private final String bindEndpoint;
    private final Manager manager;
    private final ZmqSender zmqSender;
    private ZContext context;
    private ZMQ.Socket socket;

    public ZmqReceiver(String bindEndpoint, Manager manager, ZmqSender zmqSender) {
        this.bindEndpoint = bindEndpoint;
        this.manager = manager;
        this.zmqSender = zmqSender;
    }

    @Override
    public void init() {
        context = new ZContext();
        socket = context.createSocket(SocketType.PULL);
        socket.bind(bindEndpoint);
        socket.setReceiveTimeOut(100);
    }

    @Override
    public void run() {
        init();
        while (running) {
            String msg = socket.recvStr();
            if (msg != null) {
                handleRemoteMessage(msg);
            }

            Envelope env = queue.poll();
            if (env != null) {
                processMessage(env);
            }
        }
        end();
    }

    @Override
    public void end() {
        if (socket != null) {
            socket.close();
        }
        if (context != null) {
            context.close();
        }
    }

    private void handleRemoteMessage(String json) {
        Serialization.EnvelopeData data;
        try {
            data = Serialization.deserialize(json);
        } catch (Exception e) {
            System.err.println("Failed to deserialize: " + e.getMessage());
            return;
        }

        ActorRef senderRef = null;
        if (data.senderActor != null && data.senderEndpoint != null) {
            senderRef = new RemoteActorRef(data.senderActor, data.senderEndpoint, zmqSender);
        }

        ActorRef targetRef = manager.getRef(data.receiver);
        if (targetRef == null) {
            if (senderRef != null) {
                Reject reject = new Reject(data.messageType, "Unknown actor: " + data.receiver, data.receiver);
                senderRef.send(reject, null);
            }
            return;
        }

        targetRef.send(data.message, senderRef);
    }
}
