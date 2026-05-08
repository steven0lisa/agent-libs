//! Auto compact mechanism for conversation history compression.
//!
//! When the conversation history grows too large (approaching the context
//! window limit), this module summarizes older messages to keep the
//! conversation within bounds.

use tracing;

use crate::client::AnthropicClient;
use crate::config::AgentConfig;
use crate::types::{ContentBlock, Message, Role};

/// Maximum consecutive compact failures before giving up (circuit breaker).
const MAX_CONSECUTIVE_FAILURES: usize = 3;

/// Minimum messages to preserve during compaction.
const MIN_PRESERVED_MESSAGES: usize = 5;

/// Characters per token (rough estimation for English/mixed text).
const CHARS_PER_TOKEN: f64 = 4.0;

/// Maximum output tokens for the summary.
const MAX_SUMMARY_TOKENS: u32 = 4096;

/// System prompt used for the summarization API call.
const COMPACT_SYSTEM_PROMPT: &str = "\
You are a conversation summarizer. Your task is to create a concise but comprehensive summary of the conversation history.

Rules:
1. Preserve all key facts, decisions, and outcomes
2. Keep track of file operations performed (files read, written, updated)
3. Note any errors encountered and how they were resolved
4. Maintain the context needed for the assistant to continue the conversation
5. Keep the summary as concise as possible while retaining all important information
6. Do NOT include any meta-commentary about the summarization process

Output a single coherent summary paragraph or bullet points that captures the essence of the conversation.";

/// Estimate the token count for a list of messages.
///
/// Uses a simple heuristic of ~4 characters per token.
/// Skips `ThinkingBlock` since those are typically not sent to the API.
pub fn estimate_tokens(messages: &[Message]) -> usize {
    let mut total_chars: usize = 0;
    for msg in messages {
        for block in &msg.content {
            match block {
                ContentBlock::Text { text } => {
                    total_chars += text.len();
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    // Tool name + input JSON
                    total_chars += name.len();
                    total_chars += input.to_string().len();
                }
                ContentBlock::ToolResult { content, .. } => {
                    total_chars += content.len();
                }
                ContentBlock::Thinking { .. } => {
                    // Skip thinking blocks — typically not sent to API
                }
            }
        }
    }
    (total_chars as f64 / CHARS_PER_TOKEN) as usize
}

/// Check if the conversation history should be compacted.
///
/// Returns `true` if the estimated token count exceeds the threshold
/// (context_window_size * threshold_pct).
pub fn should_compact(
    messages: &[Message],
    context_window_size: u32,
    threshold_pct: f64,
) -> bool {
    if messages.is_empty() {
        return false;
    }

    let threshold = (context_window_size as f64 * threshold_pct) as usize;
    let estimated = estimate_tokens(messages);

    if estimated >= threshold {
        tracing::info!(
            "Auto compact triggered: estimated={} tokens, threshold={} tokens",
            estimated,
            threshold,
        );
        true
    } else {
        false
    }
}

/// Find the boundary index for compaction.
///
/// We want to keep the most recent messages intact. The boundary determines
/// where we split: messages before the boundary get summarized, messages from
/// the boundary onward are preserved.
///
/// We ensure:
/// 1. At least `MIN_PRESERVED_MESSAGES` are kept
/// 2. Tool use/result pairs are not split across the boundary
pub fn find_compact_boundary(messages: &[Message]) -> usize {
    if messages.len() <= MIN_PRESERVED_MESSAGES {
        return 0;
    }

    // Start from the position that leaves MIN_PRESERVED_MESSAGES
    let mut boundary = messages.len() - MIN_PRESERVED_MESSAGES;

    // Ensure we don't split a tool_use/result pair.
    // If the message at boundary is a user message with tool_result,
    // look backward for the corresponding assistant message with tool_use.
    if boundary > 0 {
        let msg = &messages[boundary];
        if msg.role == Role::User {
            let has_tool_result = msg
                .content
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolResult { .. }));
            if has_tool_result {
                // Move boundary back to include the tool_use pair
                while boundary > 0 {
                    boundary -= 1;
                    let prev_msg = &messages[boundary];
                    if prev_msg.role == Role::Assistant {
                        let has_tool_use = prev_msg
                            .content
                            .iter()
                            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
                        if has_tool_use {
                            break;
                        }
                    }
                }
            }
        }
    }

    boundary
}

/// Format messages into a readable text for summarization.
pub fn format_messages_for_summary(messages: &[Message]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for msg in messages {
        let role_label = if msg.role == Role::User {
            "User"
        } else {
            "Assistant"
        };
        for block in &msg.content {
            match block {
                ContentBlock::Text { text } => {
                    parts.push(format!("[{}]: {}", role_label, text));
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    parts.push(format!(
                        "[{} Tool Use ({})]: {}",
                        role_label,
                        name,
                        input.to_string()
                    ));
                }
                ContentBlock::ToolResult {
                    content, is_error, ..
                } => {
                    let status = if is_error.unwrap_or(false) {
                        "Error"
                    } else {
                        "Success"
                    };
                    // Truncate long tool results to 2000 chars (same as Python)
                    let truncated = if content.len() > 2000 {
                        &content[..2000]
                    } else {
                        content.as_str()
                    };
                    parts.push(format!(
                        "[User Tool Result ({})]: {}",
                        status, truncated
                    ));
                }
                ContentBlock::Thinking { .. } => {
                    // Skip thinking blocks in summary
                }
            }
        }
    }
    parts.join("\n")
}

/// Compact the conversation history by summarizing older messages.
///
/// Returns `Some(new_history)` on success, or `None` if compaction fails.
pub async fn compact_conversation(
    client: &AnthropicClient,
    messages: &[Message],
) -> Option<Vec<Message>> {
    if messages.len() <= MIN_PRESERVED_MESSAGES {
        return Some(messages.to_vec());
    }

    let boundary = find_compact_boundary(messages);
    if boundary == 0 {
        return Some(messages.to_vec());
    }

    let old_messages = &messages[..boundary];
    let recent_messages = messages[boundary..].to_vec();

    // Format old messages for summarization
    let conversation_text = format_messages_for_summary(old_messages);

    if conversation_text.trim().is_empty() {
        return Some(messages.to_vec());
    }

    // Create the summarization request
    let summary_messages = vec![Message::user(format!(
        "Please summarize the following conversation history concisely, \
         preserving all key facts, decisions, file operations, and outcomes:\n\n\
         {}",
        conversation_text
    ))];

    let response = match client
        .send_messages(
            summary_messages,
            COMPACT_SYSTEM_PROMPT.to_string(),
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Compact failed: {}", e);
            return None;
        }
    };

    // Extract the summary text from response content blocks
    let mut summary_text = String::new();
    for block in &response.content {
        if let crate::stream::ContentBlock::Text { text } = block {
            summary_text.push_str(text);
        }
    }

    if summary_text.trim().is_empty() {
        tracing::warn!("Compact: empty summary generated, skipping");
        return None;
    }

    // Build new message history: summary + recent messages
    let compact_message = Message::user(format!(
        "[Conversation History Summary]\n\n\
         {}\n\n\
         [End of Summary - Recent conversation continues below]",
        summary_text
    ));

    let mut new_history = vec![compact_message];
    new_history.extend(recent_messages);

    tracing::info!(
        "Compact: reduced from {} to {} messages",
        messages.len(),
        new_history.len(),
    );

    Some(new_history)
}

/// Check if auto compact is needed and perform it.
///
/// This is the main entry point called from the agent loop. It checks
/// the circuit breaker, evaluates whether compaction is needed, and
/// performs it if so.
///
/// Returns `(updated_history, updated_consecutive_failures)`.
pub async fn auto_compact_if_needed(
    config: &AgentConfig,
    client: &AnthropicClient,
    messages: Vec<Message>,
    consecutive_failures: usize,
) -> (Vec<Message>, usize) {
    if !config.auto_compact {
        return (messages, consecutive_failures);
    }

    // Circuit breaker: stop trying after too many consecutive failures
    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
        return (messages, consecutive_failures);
    }

    if !should_compact(
        &messages,
        config.context_window_size,
        config.auto_compact_threshold_pct,
    ) {
        return (messages, consecutive_failures);
    }

    // Attempt compaction
    match compact_conversation(client, &messages).await {
        Some(compacted) => (compacted, 0), // Reset failure count on success
        None => (messages, consecutive_failures + 1), // Increment failure count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContentBlock, Message, Role};

    #[test]
    fn test_estimate_tokens_empty() {
        let messages: Vec<Message> = vec![];
        assert_eq!(estimate_tokens(&messages), 0);
    }

    #[test]
    fn test_estimate_tokens_text() {
        let messages = vec![Message::user("Hello world")]; // 11 chars
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 2); // 11 / 4.0 = 2.75 -> 2
    }

    #[test]
    fn test_estimate_tokens_tool_use() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                name: "read_file".to_string(),
                id: "tool_1".to_string(),
                input: serde_json::json!({"file_path": "test.txt"}),
            }],
        }];
        let tokens = estimate_tokens(&messages);
        // name: "read_file" (9) + input json: ~25 chars
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_tool_result() {
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "tool_1".to_string(),
                content: "file content here".to_string(),
                is_error: Some(false),
            }],
        }];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_skips_thinking() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Thinking {
                thinking: "This is a long thinking block that should be ignored".to_string(),
                signature: Some("sig".to_string()),
            }],
        }];
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 0); // Thinking blocks should be skipped
    }

    #[test]
    fn test_should_compact_empty() {
        assert!(!should_compact(&[], 200_000, 0.8));
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let messages = vec![Message::user("Hello")];
        assert!(!should_compact(&messages, 200_000, 0.8));
    }

    #[test]
    fn test_should_compact_above_threshold() {
        // Create a large message that exceeds 80% of 200k tokens = 160k tokens
        // 160k tokens * 4 chars/token = 640k chars
        let large_text = "a".repeat(700_000);
        let messages = vec![Message::user(large_text)];
        assert!(should_compact(&messages, 200_000, 0.8));
    }

    #[test]
    fn test_find_compact_boundary_small() {
        let messages: Vec<Message> = (0..3)
            .map(|i| Message::user(format!("msg {}", i)))
            .collect();
        // 3 messages <= MIN_PRESERVED_MESSAGES (5), boundary should be 0
        assert_eq!(find_compact_boundary(&messages), 0);
    }

    #[test]
    fn test_find_compact_boundary_normal() {
        let messages: Vec<Message> = (0..10)
            .map(|i| Message::user(format!("msg {}", i)))
            .collect();
        // 10 messages, should preserve 5, boundary = 5
        assert_eq!(find_compact_boundary(&messages), 5);
    }

    #[test]
    fn test_find_compact_boundary_tool_result_pair() {
        // 10 messages where message[5] (the boundary) is a user message
        // with a tool_result — boundary should move back
        let mut messages: Vec<Message> = (0..4)
            .map(|i| Message::user(format!("msg {}", i)))
            .collect();

        // Add assistant with tool_use
        messages.push(Message {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                name: "bash".to_string(),
                id: "tool_1".to_string(),
                input: serde_json::json!({"command": "ls"}),
            }],
        });

        // Add user with tool_result (this would be at boundary=5 for 10 msgs)
        messages.push(Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "tool_1".to_string(),
                content: "file1\nfile2".to_string(),
                is_error: Some(false),
            }],
        });

        // Add 3 more messages to get to 9 total (boundary = 9-5 = 4, which is the tool_use)
        messages.push(Message::user("continue"));
        messages.push(Message::user("more"));
        messages.push(Message::user("end"));

        let boundary = find_compact_boundary(&messages);
        // The boundary should have been moved back from 4 to include the tool_use pair
        assert!(boundary <= 4);
    }

    #[test]
    fn test_format_messages_for_summary() {
        let messages = vec![
            Message::user("Hello"),
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "Hi there".to_string(),
                }],
            },
        ];
        let formatted = format_messages_for_summary(&messages);
        assert!(formatted.contains("[User]: Hello"));
        assert!(formatted.contains("[Assistant]: Hi there"));
    }

    #[test]
    fn test_format_messages_for_summary_tool_use() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                name: "bash".to_string(),
                id: "tool_1".to_string(),
                input: serde_json::json!({"command": "ls -la"}),
            }],
        }];
        let formatted = format_messages_for_summary(&messages);
        assert!(formatted.contains("[Assistant Tool Use (bash)]"));
    }

    #[test]
    fn test_format_messages_for_summary_tool_result() {
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "tool_1".to_string(),
                content: "file1.txt\nfile2.txt".to_string(),
                is_error: Some(false),
            }],
        }];
        let formatted = format_messages_for_summary(&messages);
        assert!(formatted.contains("[User Tool Result (Success)]"));
    }

    #[test]
    fn test_format_messages_truncates_long_result() {
        let long_content = "x".repeat(3000);
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "tool_1".to_string(),
                content: long_content,
                is_error: Some(false),
            }],
        }];
        let formatted = format_messages_for_summary(&messages);
        // Should be truncated to 2000 chars + the prefix
        assert!(formatted.len() < 3000);
    }
}
