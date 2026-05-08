//! Core types for the agent library.

use serde::{Deserialize, Serialize};

/// Message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Content block types used in messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text {
        text: String,
    },
    /// Tool use request from the assistant.
    ToolUse {
        name: String,
        id: String,
        input: serde_json::Value,
    },
    /// Tool result returned to the assistant.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Thinking/reasoning content.
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a user message with text content.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    /// Create an assistant message with the given content blocks.
    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: blocks,
        }
    }
}

/// Output format for agent responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text output.
    #[default]
    Text,
    /// JSON output.
    Json,
}

/// Events emitted during agent execution.
#[derive(Debug, Clone)]
pub enum Event {
    /// A new turn has started.
    TurnStart {
        turn: usize,
    },
    /// Message streaming has started.
    MessageStart,
    /// Text delta received from the model.
    MessageDelta {
        text: String,
    },
    /// Thinking delta received from the model.
    ThinkingDelta {
        thinking: String,
    },
    /// Message streaming has ended.
    MessageEnd,
    /// A tool use has started.
    ToolUseStart {
        name: String,
        id: String,
        input: serde_json::Value,
    },
    /// A tool use has completed.
    ToolUseEnd {
        name: String,
        id: String,
        result: crate::tool::ToolResult,
    },
    /// A tool result was produced.
    ToolResult {
        tool_use_id: String,
        content: String,
    },
    /// An error occurred.
    Error {
        message: String,
    },
    /// Conversation was auto-compacted.
    Compact {
        message_count: usize,
        token_estimate: usize,
    },
    /// Agent execution completed.
    Complete {
        final_content: String,
    },
}

/// Agent lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is idle, not running.
    Idle,
    /// Agent is actively running.
    Running,
    /// Agent is paused, can be resumed.
    Paused,
    /// Agent is stopping.
    Stopping,
    /// Agent has been terminated.
    Terminated,
    /// Agent completed successfully.
    Completed,
}
