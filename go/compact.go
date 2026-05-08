package agentlib

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
)

// Maximum consecutive compact failures before giving up
const maxConsecutiveFailures = 3

// Minimum messages to preserve during compaction
const minPreservedMessages = 5

// Characters per token (rough estimation for English/mixed text)
const charsPerToken = 4.0

// Maximum output tokens for the summary
const maxSummaryTokens = 4096

const compactSystemPrompt = `You are a conversation summarizer. Your task is to create a concise but comprehensive summary of the conversation history.

Rules:
1. Preserve all key facts, decisions, and outcomes
2. Keep track of file operations performed (files read, written, updated)
3. Note any errors encountered and how they were resolved
4. Maintain the context needed for the assistant to continue the conversation
5. Keep the summary as concise as possible while retaining all important information
6. Do NOT include any meta-commentary about the summarization process

Output a single coherent summary paragraph or bullet points that captures the essence of the conversation.`

// EstimateTokens estimates the token count for a list of messages.
// Uses a simple heuristic of ~4 characters per token.
func EstimateTokens(messages []Message) int {
	totalChars := 0
	for _, msg := range messages {
		for _, block := range msg.Content {
			switch b := block.(type) {
			case TextBlock:
				totalChars += len(b.Text)
			case *TextBlock:
				totalChars += len(b.Text)
			case ToolUseBlock:
				totalChars += len(b.Name)
				totalChars += len(b.Input)
			case *ToolUseBlock:
				totalChars += len(b.Name)
				totalChars += len(b.Input)
			case ToolResultBlock:
				totalChars += len(b.Content)
			case *ToolResultBlock:
				totalChars += len(b.Content)
			// ThinkingBlock is typically not sent to API, skip
			}
		}
	}
	return int(float64(totalChars) / charsPerToken)
}

// ShouldCompact checks if the conversation history should be compacted.
func ShouldCompact(messages []Message, contextWindowSize int, thresholdPct float64) bool {
	if len(messages) == 0 {
		return false
	}

	threshold := int(float64(contextWindowSize) * thresholdPct)
	estimated := EstimateTokens(messages)

	if estimated >= threshold {
		log.Printf("Auto compact triggered: estimated=%d tokens, threshold=%d tokens", estimated, threshold)
		return true
	}
	return false
}

// findCompactBoundary finds the boundary index for compaction.
//
// We want to keep the most recent messages intact. The boundary
// determines where we split: messages before the boundary get
// summarized, messages from the boundary onward are preserved.
//
// We ensure:
// 1. At least minPreservedMessages are kept
// 2. Tool use/result pairs are not split
func findCompactBoundary(messages []Message) int {
	if len(messages) <= minPreservedMessages {
		return 0
	}

	// Start from the position that leaves MIN_PRESERVED_MESSAGES
	boundary := len(messages) - minPreservedMessages

	// Ensure we don't split a tool_use/result pair
	// If the message at boundary is a user message with tool_result,
	// look backward for the corresponding assistant message with tool_use
	if boundary > 0 {
		msg := messages[boundary]
		if msg.Role == RoleUser {
			hasToolResult := false
			for _, block := range msg.Content {
				if _, ok := block.(ToolResultBlock); ok {
					hasToolResult = true
					break
				}
				if _, ok := block.(*ToolResultBlock); ok {
					hasToolResult = true
					break
				}
			}
			if hasToolResult {
				// Move boundary back to include the tool_use pair
				for boundary > 0 {
					boundary--
					prevMsg := messages[boundary]
					if prevMsg.Role == RoleAssistant {
						hasToolUse := false
						for _, block := range prevMsg.Content {
							if _, ok := block.(ToolUseBlock); ok {
								hasToolUse = true
								break
							}
							if _, ok := block.(*ToolUseBlock); ok {
								hasToolUse = true
								break
							}
						}
						if hasToolUse {
							break
						}
					}
				}
			}
		}
	}

	if boundary < 0 {
		boundary = 0
	}
	return boundary
}

// formatMessagesForSummary formats messages into a readable text for summarization.
func formatMessagesForSummary(messages []Message) string {
	var parts []string
	for _, msg := range messages {
		roleLabel := "User"
		if msg.Role == RoleAssistant {
			roleLabel = "Assistant"
		}
		for _, block := range msg.Content {
			switch b := block.(type) {
			case TextBlock:
				parts = append(parts, fmt.Sprintf("[%s]: %s", roleLabel, b.Text))
			case *TextBlock:
				parts = append(parts, fmt.Sprintf("[%s]: %s", roleLabel, b.Text))
			case ToolUseBlock:
				parts = append(parts, fmt.Sprintf("[%s Tool Use (%s)]: %s", roleLabel, b.Name, string(b.Input)))
			case *ToolUseBlock:
				parts = append(parts, fmt.Sprintf("[%s Tool Use (%s)]: %s", roleLabel, b.Name, string(b.Input)))
			case ToolResultBlock:
				status := "Success"
				if b.IsError != nil && *b.IsError {
					status = "Error"
				}
				content := b.Content
				if len(content) > 2000 {
					content = content[:2000]
				}
				parts = append(parts, fmt.Sprintf("[User Tool Result (%s)]: %s", status, content))
			case *ToolResultBlock:
				status := "Success"
				if b.IsError != nil && *b.IsError {
					status = "Error"
				}
				content := b.Content
				if len(content) > 2000 {
					content = content[:2000]
				}
				parts = append(parts, fmt.Sprintf("[User Tool Result (%s)]: %s", status, content))
			}
		}
	}
	return strings.Join(parts, "\n")
}

// CompactConversation compacts the conversation history by summarizing older messages.
//
// Returns the new message history with older messages replaced by a summary,
// or nil if compaction fails.
func CompactConversation(ctx context.Context, client *APIClient, config Config, messages []Message) ([]Message, bool) {
	if len(messages) <= minPreservedMessages {
		return messages, true
	}

	boundary := findCompactBoundary(messages)
	if boundary == 0 {
		return messages, true
	}

	oldMessages := messages[:boundary]
	recentMessages := messages[boundary:]

	// Format old messages for summarization
	conversationText := formatMessagesForSummary(oldMessages)

	if strings.TrimSpace(conversationText) == "" {
		return messages, true
	}

	// Create the summarization request
	summaryText := fmt.Sprintf(
		"Please summarize the following conversation history concisely, "+
			"preserving all key facts, decisions, file operations, and outcomes:\n\n%s",
		conversationText,
	)
	summaryMessages := []Message{UserMessage(summaryText)}

	req := APIRequest{
		Model:     config.Model,
		Messages:  summaryMessages,
		System:    compactSystemPrompt,
		Tools:     nil,
		MaxTokens: maxSummaryTokens,
		Stream:    false,
	}

	resp, err := client.SendMessages(ctx, req)
	if err != nil {
		log.Printf("Compact failed: %v", err)
		return nil, false
	}

	// Extract the summary text from response content blocks
	var summaryBuilder strings.Builder
	for _, block := range resp.Content {
		blockType, _ := block["type"].(string)
		if blockType == "text" {
			text, _ := block["text"].(string)
			summaryBuilder.WriteString(text)
		}
	}

	summary := summaryBuilder.String()
	if strings.TrimSpace(summary) == "" {
		log.Println("Compact: empty summary generated, skipping")
		return nil, false
	}

	// Build new message history: summary + recent messages
	compactMessage := UserMessage(
		"[Conversation History Summary]\n\n" +
			summary + "\n\n" +
			"[End of Summary - Recent conversation continues below]",
	)

	// Make a copy of recent messages to avoid aliasing
	newHistory := make([]Message, 0, 1+len(recentMessages))
	newHistory = append(newHistory, compactMessage)
	newHistory = append(newHistory, recentMessages...)

	log.Printf("Compact: reduced from %d to %d messages", len(messages), len(newHistory))

	return newHistory, true
}

// AutoCompactIfNeeded checks if auto compact is needed and performs it.
// Returns (updated_history, updated_consecutive_failures).
func AutoCompactIfNeeded(ctx context.Context, config Config, client *APIClient, messages []Message, consecutiveFailures int) ([]Message, int) {
	if !config.AutoCompact {
		return messages, consecutiveFailures
	}

	// Circuit breaker: stop trying after too many consecutive failures
	if consecutiveFailures >= maxConsecutiveFailures {
		return messages, consecutiveFailures
	}

	if !ShouldCompact(messages, config.ContextWindowSize, config.AutoCompactThresholdPct) {
		return messages, consecutiveFailures
	}

	// Attempt compaction
	compacted, ok := CompactConversation(ctx, client, config, messages)
	if ok && compacted != nil {
		return compacted, 0 // Reset failure count on success
	}

	return messages, consecutiveFailures + 1
}

// ensure json.RawMessage is available (used indirectly via ToolUseBlock.Input)
var _ json.RawMessage
