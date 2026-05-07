package com.agentlib;

import java.util.ArrayList;
import java.util.List;

/**
 * A message in the conversation history.
 */
public record Message(Role role, List<ContentBlock> content) {

    public Message {
        content = List.copyOf(content);
    }

    /**
     * Create a user message with a single text block.
     */
    public static Message user(String text) {
        return new Message(Role.USER, List.of(new TextBlock(text)));
    }

    /**
     * Create an assistant message with the given content blocks.
     */
    public static Message assistant(List<ContentBlock> blocks) {
        return new Message(Role.ASSISTANT, List.copyOf(blocks));
    }

    /**
     * Create an assistant message with a single text block.
     */
    public static Message assistant(String text) {
        return new Message(Role.ASSISTANT, List.of(new TextBlock(text)));
    }

    /**
     * Extract all text from text blocks in this message.
     */
    public String extractText() {
        StringBuilder sb = new StringBuilder();
        for (ContentBlock block : content) {
            if (block instanceof TextBlock tb) {
                sb.append(tb.text());
            }
        }
        return sb.toString();
    }
}
