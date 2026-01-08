/*
 * THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
 *
 * Copyright 2025 Vincent Maciejewski, & M2 Tech
 */

package actors.remote;

import actors.RegisterMessage;
import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * JSON serialization for remote message passing.
 */
public class Serialization {
    private static final Gson gson = new GsonBuilder().create();
    private static final Map<String, Class<?>> registry = new ConcurrentHashMap<>();

    public static void registerMessage(Class<?> clazz) {
        RegisterMessage annotation = clazz.getAnnotation(RegisterMessage.class);
        String name;
        if (annotation != null && !annotation.value().isEmpty()) {
            name = annotation.value();
        } else {
            name = clazz.getSimpleName();
        }
        registry.put(name, clazz);
    }

    public static String getTypeName(Object msg) {
        Class<?> clazz = msg.getClass();
        RegisterMessage annotation = clazz.getAnnotation(RegisterMessage.class);
        if (annotation != null && !annotation.value().isEmpty()) {
            return annotation.value();
        }
        return clazz.getSimpleName();
    }

    public static String serialize(String receiver, Object msg, String senderActor, String senderEndpoint) {
        JsonObject json = new JsonObject();
        json.addProperty("sender_actor", senderActor);
        json.addProperty("sender_endpoint", senderEndpoint);
        json.addProperty("receiver", receiver);
        json.addProperty("message_type", getTypeName(msg));
        json.add("message", gson.toJsonTree(msg));
        return gson.toJson(json);
    }

    public static EnvelopeData deserialize(String json) {
        JsonObject obj = gson.fromJson(json, JsonObject.class);
        String senderActor = getStringOrNull(obj, "sender_actor");
        String senderEndpoint = getStringOrNull(obj, "sender_endpoint");
        String receiver = obj.get("receiver").getAsString();
        String messageType = obj.get("message_type").getAsString();
        JsonElement messageJson = obj.get("message");

        Class<?> clazz = registry.get(messageType);
        if (clazz == null) {
            throw new IllegalArgumentException("Unknown message type: " + messageType);
        }

        Object message = gson.fromJson(messageJson, clazz);
        return new EnvelopeData(senderActor, senderEndpoint, receiver, messageType, message);
    }

    private static String getStringOrNull(JsonObject obj, String key) {
        JsonElement element = obj.get(key);
        if (element == null || element.isJsonNull()) {
            return null;
        }
        return element.getAsString();
    }

    public static class EnvelopeData {
        public final String senderActor;
        public final String senderEndpoint;
        public final String receiver;
        public final String messageType;
        public final Object message;

        public EnvelopeData(String senderActor, String senderEndpoint, String receiver, String messageType, Object message) {
            this.senderActor = senderActor;
            this.senderEndpoint = senderEndpoint;
            this.receiver = receiver;
            this.messageType = messageType;
            this.message = message;
        }
    }
}
