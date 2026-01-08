/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package examples;

import actors.Actor;
import actors.ActorRef;
import actors.Envelope;
import actors.Manager;
import actors.remote.RemoteActorRef;
import actors.remote.Serialization;
import actors.remote.ZmqReceiver;
import actors.remote.ZmqSender;

/**
 * Remote ping process - sends pings to a remote pong actor.
 */
public class RemotePingProcess {
    public static void main(String[] args) throws Exception {
        System.out.println("=== Java Remote Ping (port 5002) ===\n");

        Serialization.registerMessage(Ping.class);
        Serialization.registerMessage(Pong.class);

        Manager manager = new Manager();
        ZmqSender zmqSender = new ZmqSender("tcp://localhost:5002");

        RemoteActorRef remotePongRef = zmqSender.remoteRef("pong", "tcp://localhost:5001");
        ActorRef pingRef = manager.manage("ping", new RemotePingActor(remotePongRef, 5));

        ZmqReceiver receiver = new ZmqReceiver("tcp://0.0.0.0:5002", manager, zmqSender);
        manager.manage("zmq_receiver", receiver);

        manager.init();
        Thread.sleep(2000);
        manager.end();
        zmqSender.close();

        System.out.println("\nDone!");
    }
}

class RemotePingActor extends Actor {
    private final ActorRef pongRef;
    private final int maxCount;

    public RemotePingActor(ActorRef pongRef, int maxCount) {
        this.pongRef = pongRef;
        this.maxCount = maxCount;
    }

    public void on_Start(Envelope env) {
        System.out.println("Ping starting, sending first ping to remote pong");
        pongRef.send(new Ping(1), actorRef);
    }

    public void on_Pong(Envelope env) {
        Pong pong = (Pong) env.getMsg();
        String senderName = env.getSender() != null ? env.getSender().getName() : "unknown";
        System.out.println("Received pong #" + pong.count + " from " + senderName);
        if (pong.count < maxCount) {
            pongRef.send(new Ping(pong.count + 1), actorRef);
        } else {
            System.out.println("Reached max count, stopping");
            stop();
        }
    }
}
