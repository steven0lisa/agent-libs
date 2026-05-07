//! Tool trait and related types.

use async_trait::async_trait;
use serde_json::Value;

use crate::types::Message;

/// Context provided to tools during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Working directory for file operations.
    pub work_dir: std::path::PathBuf,
    /// Current message history.
    pub message_history: Vec<Message>,
}

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The result content.
    pub content: String,
    /// Whether the result represents an error.
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful tool result.
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
        }
    }

    /// Create an error tool result.
    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
        }
    }
}

/// Error from tool execution.
#[derive(Debug, Clone)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

/// Definition of a tool for the Anthropic API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Trait for tools that can be registered with an Agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &str;

    /// Tool description.
    fn description(&self) -> &str;

    /// JSON schema for tool input.
    fn input_schema(&self) -> Value;

    /// Whether this tool only reads data (affects concurrency).
    fn is_read_only(&self) -> bool {
        false
    }

    /// Execute the tool with the given input and context.
    async fn call(&self, input: Value, context: &ToolContext) -> Result<ToolResult, ToolError>;
}

use serde::{Deserialize, Serialize};
