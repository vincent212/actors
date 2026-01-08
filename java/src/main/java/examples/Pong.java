/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package examples;

import actors.Message;
import actors.RegisterMessage;

/**
 * Pong message - compatible with Python/Rust wire format.
 */
@RegisterMessage
public class Pong implements Message {
    public int count;

    public Pong() {}

    public Pong(int count) {
        this.count = count;
    }
}
