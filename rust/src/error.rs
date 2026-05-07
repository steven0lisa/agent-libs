//! Error types for the agent library.

use thiserror::Error;

/// Errors that can occur during agent execution.
#[derive(Debug, Error, Clone)]
pub enum AgentError {
    /// API request failed.
    #[error("API error: {0}")]
    ApiError(String),

    /// Tool was not found in the registry.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Maximum number of turns reached.
    #[error("Maximum turns reached")]
    MaxTurnsReached,

    /// Maximum duration exceeded.
    #[error("Maximum duration exceeded")]
    MaxDurationExceeded,

    /// Agent was stopped.
    #[error("Agent stopped")]
    Stopped,

    /// Invalid input or configuration.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(String),

    /// Security policy violation.
    #[error("Security policy violation: {0}")]
    SecurityViolation(String),

    /// Subagent error.
    #[error("Subagent error: {0}")]
    SubagentError(String),
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::IoError(err.to_string())
    }
}

impl From<reqwest::Error> for AgentError {
    fn from(err: reqwest::Error) -> Self {
        AgentError::ApiError(err.to_string())
    }
}
