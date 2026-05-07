package com.agentlib;

/**
 * Role of a message sender in the conversation.
 */
public enum Role {
    USER("user"),
    ASSISTANT("assistant");

    private final String value;

    Role(String value) {
        this.value = value;
    }

    public String value() {
        return value;
    }
}
