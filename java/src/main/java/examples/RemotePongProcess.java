/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package examples;

import actors.Actor;
import actors.Envelope;
import actors.Manager;
import actors.remote.Serialization;
import actors.remote.ZmqReceiver;
import actors.remote.ZmqSender;

/**
 * Remote pong process - receives pings and replies with pongs.
 */
public class RemotePongProcess {
    public static void main(String[] args) throws Exception {
        System.out.println("=== Java Remote Pong (port 5001) ===\n");

        Serialization.registerMessage(Ping.class);
        Serialization.registerMessage(Pong.class);

        Manager manager = new Manager();
        ZmqSender zmqSender = new ZmqSender("tcp://localhost:5001");

        manager.manage("pong", new RemotePongActor());

        ZmqReceiver receiver = new ZmqReceiver("tcp://0.0.0.0:5001", manager, zmqSender);
        manager.manage("zmq_receiver", receiver);

        manager.init();
        System.out.println("Waiting for ping messages...\n");

        Runtime.getRuntime().addShutdownHook(new Thread(() -> {
            System.out.println("\nShutting down...");
            manager.end();
            zmqSender.close();
        }));

        Thread.currentThread().join();
    }
}

class RemotePongActor extends Actor {
    public void on_Ping(Envelope env) {
        Ping ping = (Ping) env.getMsg();
        String senderName = env.getSender() != null ? env.getSender().getName() : "unknown";
        System.out.println("Received ping #" + ping.count + " from " + senderName);
        reply(env, new Pong(ping.count));
    }
}
