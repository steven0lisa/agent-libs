//! Curl tool with whitelist/blacklist security.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::config::Pattern;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::utils::security::check_security_policy;

/// Tool to make HTTP requests with security policy enforcement.
pub struct CurlTool {
    whitelist: Vec<Pattern>,
    blacklist: Vec<Pattern>,
    client: reqwest::Client,
}

impl CurlTool {
    /// Create a new CurlTool with optional whitelist and blacklist.
    pub fn new(whitelist: Vec<Pattern>, blacklist: Vec<Pattern>) -> Self {
        Self {
            whitelist,
            blacklist,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Tool for CurlTool {
    fn name(&self) -> &str {
        "curl"
    }

    fn description(&self) -> &str {
        "Make an HTTP request."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {"type": "string"},
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"],
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "additionalProperties": {"type": "string"}
                },
                "body": {"type": "string"},
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "default": 30000
                }
            },
            "required": ["url"]
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap_or("");
        let method = input["method"].as_str().unwrap_or("GET");
        let body = input["body"].as_str();
        let timeout_ms = input["timeout"].as_u64().unwrap_or(30_000);

        // Parse headers
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(headers_obj) = input["headers"].as_object() {
            for (key, value) in headers_obj {
                if let Ok(header_name) = key.parse::<reqwest::header::HeaderName>() {
                    if let Ok(header_value) = value.as_str().unwrap_or("").parse::<reqwest::header::HeaderValue>() {
                        headers.insert(header_name, header_value);
                    }
                }
            }
        }

        // Security check - enforced at execution time, not disclosed in prompt
        let (allowed, reason) = check_security_policy(
            url,
            &self.whitelist,
            &self.blacklist,
            true,
        );
        if !allowed {
            return Ok(ToolResult::error(format!(
                "URL blocked by security policy: {}",
                reason
            )));
        }

        let method = match method.to_uppercase().as_str() {
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            _ => reqwest::Method::GET,
        };

        let mut request = self.client.request(method, url).headers(headers);
        if let Some(body_str) = body {
            request = request.body(body_str.to_string());
        }

        let response = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            request.send(),
        )
        .await
        .map_err(|_| ToolError(format!("Request timed out after {}ms", timeout_ms)))?
        .map_err(|e| ToolError(format!("HTTP error: {}", e)))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| ToolError(format!("Failed to read response: {}", e)))?;

        let content = format!("Status: {}\n\n{}", status, text);
        Ok(ToolResult {
            content,
            is_error: !status.is_success(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_curl_get() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = CurlTool::new(vec![], vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"url": "https://httpbin.org/get"});

        let result = tool.call(input, &ctx).await;
        // May fail in test environment without network, so we just check it doesn't panic
        // In a real environment with network, this would succeed
        match result {
            Ok(r) => {
                // If it succeeds, check structure
                assert!(r.content.contains("Status:"));
            }
            Err(e) => {
                // Network errors are acceptable in tests
                assert!(e.0.contains("HTTP error") || e.0.contains("timed out"));
            }
        }
    }

    #[tokio::test]
    async fn test_curl_blacklist() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let blacklist = vec![Pattern::wildcard("*internal*")];
        let tool = CurlTool::new(vec![], blacklist);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"url": "http://internal.example.com/api"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("blocked by security policy"));
    }

    #[tokio::test]
    async fn test_curl_whitelist_priority() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let whitelist = vec![Pattern::wildcard("*safe.example.com*")];
        let blacklist = vec![Pattern::wildcard("*example.com*")];
        let tool = CurlTool::new(whitelist, blacklist);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"url": "http://safe.example.com/api"});

        let result = tool.call(input, &ctx).await;
        // Should be allowed because whitelist matches - but may fail due to network
        match result {
            Ok(r) => {
                // If it succeeds, it should not be a security block
                assert!(!r.content.contains("blocked by security policy"));
            }
            Err(e) => {
                // Network errors are acceptable - the key is it wasn't blocked
                assert!(!e.0.contains("blocked by security policy"));
            }
        }
    }

    #[tokio::test]
    async fn test_curl_post_with_body() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = CurlTool::new(vec![], vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({
            "url": "https://httpbin.org/post",
            "method": "POST",
            "body": "{\"key\": \"value\"}",
            "headers": {
                "Content-Type": "application/json"
            }
        });

        let result = tool.call(input, &ctx).await;
        // May fail in test environment without network
        match result {
            Ok(r) => {
                assert!(r.content.contains("Status:"));
            }
            Err(e) => {
                assert!(e.0.contains("HTTP error") || e.0.contains("timed out"));
            }
        }
    }
}
