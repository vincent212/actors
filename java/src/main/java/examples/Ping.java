/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package examples;

import actors.Message;
import actors.RegisterMessage;

/**
 * Ping message - compatible with Python/Rust wire format.
 */
@RegisterMessage
public class Ping implements Message {
    public int count;

    public Ping() {}

    public Ping(int count) {
        this.count = count;
    }
}
