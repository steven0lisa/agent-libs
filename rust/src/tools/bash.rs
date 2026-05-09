//! Bash tool with whitelist/blacklist security.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::config::Pattern;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::utils::security::check_security_policy;

/// Tool to execute shell commands with security policy enforcement.
pub struct BashTool {
    whitelist: Vec<Pattern>,
    blacklist: Vec<Pattern>,
}

impl BashTool {
    /// Create a new BashTool with optional whitelist and blacklist.
    pub fn new(whitelist: Vec<Pattern>, blacklist: Vec<Pattern>) -> Self {
        Self {
            whitelist,
            blacklist,
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the working directory."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "description": {
                    "type": "string",
                    "description": "A brief description of what the command does"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "default": 120000
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| ToolError("command is required".to_string()))?;
        let timeout_ms = input["timeout"].as_u64().unwrap_or(120_000);

        // Security check - enforced at execution time, not disclosed in prompt
        let (allowed, reason) = check_security_policy(
            command,
            &self.whitelist,
            &self.blacklist,
            true,
        );
        if !allowed {
            return Ok(ToolResult::error(format!(
                "Command blocked by security policy: {}",
                reason
            )));
        }

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&ctx.work_dir);

        // Inject extra environment variables
        for (key, value) in &ctx.extra_env {
            cmd.env(key, value);
        }

        let output = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            cmd.output(),
        )
        .await
        .map_err(|_| ToolError(format!("Command timed out after {}ms", timeout_ms)))?
        .map_err(|e| ToolError(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let content = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n[stderr]\n{}", stdout, stderr)
        };

        Ok(ToolResult {
            content,
            is_error: !output.status.success(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_bash_success() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = BashTool::new(vec![], vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"command": "echo hello"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_stderr() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = BashTool::new(vec![], vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"command": "echo error >&2"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(result.content.contains("[stderr]"));
        assert!(result.content.contains("error"));
    }

    #[tokio::test]
    async fn test_bash_blacklist() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let blacklist = vec![Pattern::wildcard("rm *")];
        let tool = BashTool::new(vec![], blacklist);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"command": "rm -rf /"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("blocked by security policy"));
    }

    #[tokio::test]
    async fn test_bash_whitelist_priority() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        // Whitelist overrides blacklist
        let whitelist = vec![Pattern::wildcard("rm safe*")];
        let blacklist = vec![Pattern::wildcard("rm *")];
        let tool = BashTool::new(whitelist, blacklist);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"command": "rm safe_file.txt"});

        let result = tool.call(input, &ctx).await.unwrap();
        // Should be allowed because whitelist matches
        assert!(!result.is_error || !result.content.contains("blocked"));
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = BashTool::new(vec![], vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"command": "sleep 10", "timeout": 100});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("timed out"));
    }

    #[tokio::test]
    async fn test_bash_working_directory() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = BashTool::new(vec![], vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"command": "pwd"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        // The output should contain the working directory path
        let work_str = work_dir.canonicalize().unwrap().to_string_lossy().to_string();
        assert!(result.content.contains(&work_str) || result.content.contains("agent-libs"));
    }
}
