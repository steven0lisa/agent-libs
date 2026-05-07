//! Anthropic API client.

use std::time::Duration;

use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::stream::{ContentBlock, SseStream, StreamEvent, Usage};
use crate::tool::ToolDefinition;
use crate::types::Message;

/// Anthropic API client.
pub struct AnthropicClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: u32,
}

impl AnthropicClient {
    /// Create a new AnthropicClient from config.
    pub fn new(config: &AgentConfig) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&config.api_key).unwrap_or(HeaderValue::from_static("")),
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static("2023-06-01"),
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
        }
    }

    /// Send a streaming request and return SSE events.
    pub async fn stream_messages(
        &self,
        messages: Vec<Message>,
        system: String,
        tools: Vec<ToolDefinition>,
    ) -> Result<impl Stream<Item = Result<StreamEvent, AgentError>> + '_, AgentError> {
        let request = ApiRequest {
            model: self.model.clone(),
            messages: messages.into_iter().map(|m| self.message_to_value(m)).collect(),
            system,
            tools: tools.into_iter().map(|t| self.tool_to_value(t)).collect(),
            max_tokens: self.max_tokens,
            stream: true,
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(AgentError::from)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AgentError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let stream = response.bytes_stream();
        Ok(SseStream::new(stream))
    }

    /// Send a non-streaming request and return the response.
    pub async fn send_messages(
        &self,
        messages: Vec<Message>,
        system: String,
        tools: Vec<ToolDefinition>,
    ) -> Result<ApiResponse, AgentError> {
        let request = ApiRequest {
            model: self.model.clone(),
            messages: messages.into_iter().map(|m| self.message_to_value(m)).collect(),
            system,
            tools: tools.into_iter().map(|t| self.tool_to_value(t)).collect(),
            max_tokens: self.max_tokens,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(AgentError::from)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AgentError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let api_response: ApiResponse = response.json().await.map_err(AgentError::from)?;
        Ok(api_response)
    }

    fn message_to_value(&self, message: Message) -> Value {
        let content: Vec<Value> = message
            .content
            .into_iter()
            .map(|block| match block {
                crate::types::ContentBlock::Text { text } => {
                    json!({"type": "text", "text": text})
                }
                crate::types::ContentBlock::ToolUse { name, id, input } => {
                    json!({"type": "tool_use", "name": name, "id": id, "input": input})
                }
                crate::types::ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => {
                    let mut result = json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": content,
                    });
                    if let Some(err) = is_error {
                        result["is_error"] = json!(err);
                    }
                    result
                }
                crate::types::ContentBlock::Thinking { thinking, signature } => {
                    let mut result = json!({"type": "thinking", "thinking": thinking});
                    if let Some(sig) = signature {
                        result["signature"] = json!(sig);
                    }
                    result
                }
            })
            .collect();

        json!({
            "role": match message.role {
                crate::types::Role::User => "user",
                crate::types::Role::Assistant => "assistant",
            },
            "content": content,
        })
    }

    fn tool_to_value(&self, tool: ToolDefinition) -> Value {
        json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
        })
    }
}

/// API request structure.
#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<Value>,
    system: String,
    tools: Vec<Value>,
    max_tokens: u32,
    stream: bool,
}

/// API response structure.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContentBlock as LibContentBlock, Message, Role};

    #[test]
    fn test_message_to_value_text() {
        let config = AgentConfig::default();
        let client = AnthropicClient::new(&config);

        let msg = Message {
            role: Role::User,
            content: vec![LibContentBlock::Text {
                text: "hello".to_string(),
            }],
        };

        let value = client.message_to_value(msg);
        assert_eq!(value["role"], "user");
        assert_eq!(value["content"][0]["type"], "text");
        assert_eq!(value["content"][0]["text"], "hello");
    }

    #[test]
    fn test_message_to_value_tool_use() {
        let config = AgentConfig::default();
        let client = AnthropicClient::new(&config);

        let msg = Message {
            role: Role::Assistant,
            content: vec![LibContentBlock::ToolUse {
                name: "read_file".to_string(),
                id: "tool_1".to_string(),
                input: serde_json::json!({"file_path": "test.txt"}),
            }],
        };

        let value = client.message_to_value(msg);
        assert_eq!(value["role"], "assistant");
        assert_eq!(value["content"][0]["type"], "tool_use");
        assert_eq!(value["content"][0]["name"], "read_file");
    }

    #[test]
    fn test_message_to_value_tool_result() {
        let config = AgentConfig::default();
        let client = AnthropicClient::new(&config);

        let msg = Message {
            role: Role::User,
            content: vec![LibContentBlock::ToolResult {
                tool_use_id: "tool_1".to_string(),
                content: "file content".to_string(),
                is_error: Some(false),
            }],
        };

        let value = client.message_to_value(msg);
        assert_eq!(value["content"][0]["type"], "tool_result");
        assert_eq!(value["content"][0]["tool_use_id"], "tool_1");
        assert_eq!(value["content"][0]["is_error"], false);
    }
}
