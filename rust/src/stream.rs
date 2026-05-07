//! SSE stream parsing for Anthropic API responses.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use serde::{Deserialize, Serialize};

/// Events from the Anthropic API stream.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Message started.
    MessageStart { message: ApiMessage },
    /// Content block started.
    ContentBlockStart { index: usize, content_block: ContentBlock },
    /// Content block delta.
    ContentBlockDelta { index: usize, delta: ContentDelta },
    /// Content block ended.
    ContentBlockStop { index: usize },
    /// Message delta.
    MessageDelta { stop_reason: Option<String>, usage: Option<Usage> },
    /// Message ended.
    MessageStop,
    /// Ping event.
    Ping,
}

/// API message structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

/// Content block in API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { name: String, id: String, input: serde_json::Value },
    Thinking { thinking: String, signature: Option<String> },
}

/// Delta types for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}

/// Token usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// SSE event from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Type alias for the byte stream from reqwest.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>;

/// Parse SSE lines into StreamEvents.
pub struct SseStream {
    lines: ByteStream,
    buffer: String,
}

impl SseStream {
    /// Create a new SSE stream from a byte stream.
    pub fn new(
        stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    ) -> Self {
        Self {
            lines: Box::pin(stream),
            buffer: String::new(),
        }
    }

    /// Parse a single SSE data line into a StreamEvent.
    fn parse_event(data: &str) -> Option<StreamEvent> {
        let event: SseEvent = serde_json::from_str(data).ok()?;
        match event.event_type.as_str() {
            "message_start" => {
                let message: ApiMessage = serde_json::from_value(event.data.get("message")?.clone()).ok()?;
                Some(StreamEvent::MessageStart { message })
            }
            "content_block_start" => {
                let index = event.data.get("index")?.as_u64()? as usize;
                let content_block: ContentBlock = serde_json::from_value(
                    event.data.get("content_block")?.clone(),
                )
                .ok()?;
                Some(StreamEvent::ContentBlockStart { index, content_block })
            }
            "content_block_delta" => {
                let index = event.data.get("index")?.as_u64()? as usize;
                let delta: ContentDelta = serde_json::from_value(
                    event.data.get("delta")?.clone(),
                )
                .ok()?;
                Some(StreamEvent::ContentBlockDelta { index, delta })
            }
            "content_block_stop" => {
                let index = event.data.get("index")?.as_u64()? as usize;
                Some(StreamEvent::ContentBlockStop { index })
            }
            "message_delta" => {
                let stop_reason = event.data.get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                let usage = event.data.get("usage")
                    .and_then(|u| serde_json::from_value(u.clone()).ok());
                Some(StreamEvent::MessageDelta { stop_reason, usage })
            }
            "message_stop" => Some(StreamEvent::MessageStop),
            "ping" => Some(StreamEvent::Ping),
            _ => None,
        }
    }
}

impl Stream for SseStream {
    type Item = Result<StreamEvent, crate::error::AgentError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.lines.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    let chunk = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&chunk);

                    // Process complete lines
                    while let Some(pos) = self.buffer.find('\n') {
                        let line = self.buffer[..pos].trim().to_string();
                        self.buffer = self.buffer[pos + 1..].to_string();

                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                return Poll::Ready(None);
                            }
                            if let Some(event) = Self::parse_event(data) {
                                return Poll::Ready(Some(Ok(event)));
                            }
                        }
                    }
                    // Continue polling for more data
                    continue;
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(crate::error::AgentError::ApiError(
                        e.to_string(),
                    ))));
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}
