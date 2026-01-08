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
import actors.msg.Start;

/**
 * Local ping-pong example demonstrating basic actor communication.
 */
public class PingPong {
    public static void main(String[] args) throws Exception {
        System.out.println("=== Java Actors Ping-Pong Example ===\n");

        Manager manager = new Manager();
        ActorRef pongRef = manager.manage("pong", new PongActor());
        ActorRef pingRef = manager.manage("ping", new PingActor(pongRef, 5));

        manager.init();
        Thread.sleep(1000);
        manager.end();

        System.out.println("\nDone!");
    }
}

class PingActor extends Actor {
    private final ActorRef pongRef;
    private final int maxCount;

    public PingActor(ActorRef pongRef, int maxCount) {
        this.pongRef = pongRef;
        this.maxCount = maxCount;
    }

    public void on_Start(Envelope env) {
        System.out.println("Ping starting, sending first ping");
        pongRef.send(new Ping(1), actorRef);
    }

    public void on_Pong(Envelope env) {
        Pong pong = (Pong) env.getMsg();
        System.out.println("Received pong #" + pong.count);
        if (pong.count < maxCount) {
            pongRef.send(new Ping(pong.count + 1), actorRef);
        } else {
            System.out.println("Reached max count, stopping");
            stop();
        }
    }
}

class PongActor extends Actor {
    public void on_Ping(Envelope env) {
        Ping ping = (Ping) env.getMsg();
        System.out.println("Received ping #" + ping.count);
        reply(env, new Pong(ping.count));
    }
}
