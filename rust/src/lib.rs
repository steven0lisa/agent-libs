//! AgentLib - Rust Agent library with Anthropic API integration.
//!
//! This library provides an Agent implementation that can interact with
//! the Anthropic API, execute tools, and manage agent lifecycle.
//!
//! # Example
//!
//! ```rust,no_run
//! use agentlib::{Agent, AgentConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = AgentConfig::default();
//!     let mut agent = Agent::new(config);
//!     let mut rx = agent.run("Hello!").await.unwrap();
//!
//!     while let Some(event) = rx.recv().await {
//!         println!("{:?}", event);
//!     }
//! }
//! ```

pub mod agent;
pub mod client;
pub mod config;
pub mod error;
pub mod prompt;
pub mod skills;
pub mod stream;
pub mod tool;
pub mod tools;
pub mod types;
pub mod utils;

pub use agent::Agent;
pub use client::AnthropicClient;
pub use config::{AgentConfig, Pattern, PatternType};
pub use error::AgentError;
pub use tool::{Tool, ToolContext, ToolDefinition, ToolError, ToolResult};
pub use types::{
    AgentState, ContentBlock, Event, Message, OutputFormat, Role,
};
