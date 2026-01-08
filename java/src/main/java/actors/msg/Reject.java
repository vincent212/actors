/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors.msg;

import actors.Message;

/**
 * Reject message indicating an actor could not process a message.
 */
public class Reject implements Message {
    public String messageType;
    public String reason;
    public String rejectedBy;

    public Reject() {
    }

    public Reject(String messageType, String reason, String rejectedBy) {
        this.messageType = messageType;
        this.reason = reason;
        this.rejectedBy = rejectedBy;
    }

    @Override
    public String toString() {
        return "Reject{messageType='" + messageType + "', reason='" + reason + "', rejectedBy='" + rejectedBy + "'}";
    }
}
