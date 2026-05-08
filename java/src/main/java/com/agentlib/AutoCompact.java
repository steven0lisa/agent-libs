package com.agentlib;

import java.util.ArrayList;
import java.util.List;

/**
 * Auto compact mechanism for conversation history compression.
 */
public class AutoCompact {

    private static final int MAX_CONSECUTIVE_FAILURES = 3;
    private static final int MIN_PRESERVED_MESSAGES = 5;
    private static final double CHARS_PER_TOKEN = 4.0;
    private static final int MAX_SUMMARY_TOKENS = 4096;

    private static final String COMPACT_SYSTEM_PROMPT =
        "You are a conversation summarizer. Your task is to create a concise but comprehensive summary of the conversation history.\n\n"
        + "Rules:\n"
        + "1. Preserve all key facts, decisions, and outcomes\n"
        + "2. Keep track of file operations performed (files read, written, updated)\n"
        + "3. Note any errors encountered and how they were resolved\n"
        + "4. Maintain the context needed for the assistant to continue the conversation\n"
        + "5. Keep the summary as concise as possible while retaining all important information\n"
        + "6. Do NOT include any meta-commentary about the summarization process\n\n"
        + "Output a single coherent summary paragraph or bullet points that captures the essence of the conversation.";

    private AutoCompact() {
        // Utility class
    }

    /**
     * Estimate the token count for a list of messages.
     * Uses a simple heuristic of ~4 characters per token.
     */
    public static int estimateTokens(List<Message> messages) {
        long totalChars = 0;
        for (Message msg : messages) {
            for (ContentBlock block : msg.content()) {
                if (block instanceof TextBlock tb) {
                    totalChars += tb.text().length();
                } else if (block instanceof ToolUseBlock tub) {
                    totalChars += tub.name().length();
                    totalChars += tub.input().toString().length();
                } else if (block instanceof ToolResultBlock trb) {
                    totalChars += trb.content().length();
                }
                // ThinkingBlock is typically not sent to API, skip
            }
        }
        return (int) (totalChars / CHARS_PER_TOKEN);
    }

    /**
     * Check if the conversation history should be compacted.
     */
    public static boolean shouldCompact(List<Message> messages, int contextWindowSize, double thresholdPct) {
        if (messages == null || messages.isEmpty()) {
            return false;
        }

        int threshold = (int) (contextWindowSize * thresholdPct);
        int estimated = estimateTokens(messages);

        return estimated >= threshold;
    }

    /**
     * Find the boundary index for compaction.
     *
     * We want to keep the most recent messages intact. The boundary
     * determines where we split: messages before the boundary get
     * summarized, messages from the boundary onward are preserved.
     *
     * We ensure:
     * 1. At least MIN_PRESERVED_MESSAGES are kept
     * 2. Tool use/result pairs are not split
     */
    private static int findCompactBoundary(List<Message> messages) {
        if (messages.size() <= MIN_PRESERVED_MESSAGES) {
            return 0;
        }

        // Start from the position that leaves MIN_PRESERVED_MESSAGES
        int boundary = messages.size() - MIN_PRESERVED_MESSAGES;

        // Ensure we don't split a tool_use/result pair
        // If the message at boundary is a user message with tool_result,
        // look backward for the corresponding assistant message with tool_use
        if (boundary > 0) {
            Message msg = messages.get(boundary);
            if (msg.role() == Role.USER) {
                boolean hasToolResult = msg.content().stream()
                    .anyMatch(b -> b instanceof ToolResultBlock);
                if (hasToolResult) {
                    // Move boundary back to include the tool_use pair
                    while (boundary > 0) {
                        boundary--;
                        Message prevMsg = messages.get(boundary);
                        if (prevMsg.role() == Role.ASSISTANT) {
                            boolean hasToolUse = prevMsg.content().stream()
                                .anyMatch(b -> b instanceof ToolUseBlock);
                            if (hasToolUse) {
                                break;
                            }
                        }
                    }
                }
            }
        }

        return Math.max(0, boundary);
    }

    /**
     * Format messages into a readable text for summarization.
     */
    private static String formatMessagesForSummary(List<Message> messages) {
        StringBuilder sb = new StringBuilder();
        for (Message msg : messages) {
            String roleLabel = msg.role() == Role.USER ? "User" : "Assistant";
            for (ContentBlock block : msg.content()) {
                if (block instanceof TextBlock tb) {
                    sb.append("[").append(roleLabel).append("]: ").append(tb.text()).append("\n");
                } else if (block instanceof ToolUseBlock tub) {
                    sb.append("[").append(roleLabel).append(" Tool Use (")
                      .append(tub.name()).append(")]: ")
                      .append(tub.input().toString()).append("\n");
                } else if (block instanceof ToolResultBlock trb) {
                    String status = Boolean.TRUE.equals(trb.isError()) ? "Error" : "Success";
                    String content = trb.content();
                    if (content.length() > 2000) {
                        content = content.substring(0, 2000);
                    }
                    sb.append("[User Tool Result (").append(status).append(")]: ")
                      .append(content).append("\n");
                }
            }
        }
        return sb.toString();
    }

    /**
     * Compact the conversation history by summarizing older messages.
     *
     * Returns the new message history with older messages replaced by a summary,
     * or null if compaction fails.
     */
    public static List<Message> compactConversation(AnthropicClient client, AgentConfig config, List<Message> messages) {
        if (messages.size() <= MIN_PRESERVED_MESSAGES) {
            return messages;
        }

        int boundary = findCompactBoundary(messages);
        if (boundary == 0) {
            return messages;
        }

        List<Message> oldMessages = messages.subList(0, boundary);
        List<Message> recentMessages = messages.subList(boundary, messages.size());

        // Format old messages for summarization
        String conversationText = formatMessagesForSummary(oldMessages);

        if (conversationText.isBlank()) {
            return messages;
        }

        // Create the summarization request
        List<Message> summaryMessages = List.of(
            Message.user(
                "Please summarize the following conversation history concisely, "
                + "preserving all key facts, decisions, file operations, and outcomes:\n\n"
                + conversationText
            )
        );

        try {
            Message response = client.sendMessages(summaryMessages, COMPACT_SYSTEM_PROMPT, List.of());

            // Extract the summary text
            String summaryText = response.extractText();

            if (summaryText == null || summaryText.isBlank()) {
                return null;
            }

            // Build new message history: summary + recent messages
            Message compactMessage = Message.user(
                "[Conversation History Summary]\n\n"
                + summaryText + "\n\n"
                + "[End of Summary - Recent conversation continues below]"
            );

            List<Message> newHistory = new ArrayList<>();
            newHistory.add(compactMessage);
            newHistory.addAll(recentMessages);

            return newHistory;

        } catch (Exception e) {
            return null;
        }
    }

    /**
     * Check if auto compact is needed and perform it.
     *
     * Returns a CompactResult with updated history and failure count.
     */
    public static CompactResult autoCompactIfNeeded(
        AnthropicClient client,
        AgentConfig config,
        List<Message> messages,
        int consecutiveFailures
    ) {
        if (!config.autoCompact()) {
            return new CompactResult(messages, consecutiveFailures);
        }

        // Circuit breaker: stop trying after too many consecutive failures
        if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
            return new CompactResult(messages, consecutiveFailures);
        }

        if (!shouldCompact(messages, config.contextWindowSize(), config.autoCompactThresholdPct())) {
            return new CompactResult(messages, consecutiveFailures);
        }

        // Attempt compaction
        List<Message> result = compactConversation(client, config, messages);

        if (result != null) {
            return new CompactResult(result, 0); // Reset failure count on success
        } else {
            return new CompactResult(messages, consecutiveFailures + 1);
        }
    }

    /**
     * Result record for auto compact operations.
     */
    public record CompactResult(List<Message> history, int failures) {}
}
