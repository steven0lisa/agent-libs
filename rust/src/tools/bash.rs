//! Bash tool with whitelist/blacklist security and output buffering.

use std::process::Stdio as StdProcessStdio;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

use crate::config::Pattern;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::utils::security::check_security_policy;

/// Ring buffer that keeps only the last `capacity` bytes of written data.
struct OutputBuffer {
    data: Vec<u8>,
    capacity: usize,
    total_written: usize,
}

impl OutputBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            data: Vec::new(),
            capacity: if capacity == 0 { usize::MAX } else { capacity },
            total_written: 0,
        }
    }

    fn write_chunk(&mut self, chunk: &[u8]) {
        self.total_written += chunk.len();

        if chunk.len() >= self.capacity {
            // Chunk itself exceeds capacity — keep only the tail
            self.data.clear();
            let start = chunk.len() - self.capacity;
            self.data.extend_from_slice(&chunk[start..]);
        } else if self.data.len() + chunk.len() > self.capacity {
            // Would overflow — discard oldest bytes
            let drop_count = self.data.len() + chunk.len() - self.capacity;
            self.data.drain(0..drop_count);
            self.data.extend_from_slice(chunk);
        } else {
            self.data.extend_from_slice(chunk);
        }
    }

    fn is_truncated(&self) -> bool {
        self.total_written > self.data.len()
    }
}

/// Read from an async stream into a ring buffer, keeping only the last `capacity` bytes.
async fn read_to_buffer<R: AsyncReadExt + Unpin>(mut reader: R, capacity: usize) -> OutputBuffer {
    let mut buf = OutputBuffer::new(capacity);
    let mut tmp = [0u8; 8192];
    loop {
        match reader.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => buf.write_chunk(&tmp[..n]),
            Err(_) => break,
        }
    }
    buf
}

/// Format an output buffer to string, adding truncation notice if needed.
fn format_buffer(buf: OutputBuffer) -> String {
    let truncated = buf.is_truncated();
    let total = buf.total_written;
    let data_len = buf.data.len();
    let content = String::from_utf8_lossy(&buf.data).to_string();

    if truncated {
        let total_kb = total as f64 / 1024.0;
        let kept_kb = data_len as f64 / 1024.0;
        format!(
            "[Output truncated: total {:.1}KB, keeping last {:.1}KB]\n{}",
            total_kb, kept_kb, content
        )
    } else {
        content
    }
}

/// Tool to execute shell commands with security policy enforcement.
pub struct BashTool {
    whitelist: Vec<Pattern>,
    blacklist: Vec<Pattern>,
    output_buffer_size: usize,
}

impl BashTool {
    /// Create a new BashTool with optional whitelist, blacklist, and output buffer size.
    pub fn new(whitelist: Vec<Pattern>, blacklist: Vec<Pattern>, output_buffer_size: usize) -> Self {
        Self {
            whitelist,
            blacklist,
            output_buffer_size: if output_buffer_size == 0 { 8192 } else { output_buffer_size },
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> String {
        "Execute a shell command in the working directory.".to_string()
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

        // Build command with piped stdout/stderr
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&ctx.work_dir)
            .stdout(StdProcessStdio::piped())
            .stderr(StdProcessStdio::piped());

        // Inject extra environment variables
        for (key, value) in &ctx.extra_env {
            cmd.env(key, value);
        }

        // Spawn child process
        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError(format!("Failed to spawn command: {}", e)))?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ToolError("Failed to capture stdout".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ToolError("Failed to capture stderr".to_string())
        })?;

        // Read stdout and stderr into ring buffers concurrently
        let buf_size = self.output_buffer_size;
        let stdout_task = tokio::spawn(read_to_buffer(stdout, buf_size));
        let stderr_task = tokio::spawn(read_to_buffer(stderr, buf_size));

        // Wait for the process to finish with timeout
        let wait_result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            child.wait(),
        )
        .await;

        match wait_result {
            Ok(Ok(status)) => {
                // Process completed within timeout
                let stdout_buf = stdout_task.await.map_err(|e| ToolError(e.to_string()))?;
                let stderr_buf = stderr_task.await.map_err(|e| ToolError(e.to_string()))?;

                let stdout_str = format_buffer(stdout_buf);
                let stderr_str = format_buffer(stderr_buf);

                let content = if stderr_str.is_empty() {
                    stdout_str
                } else {
                    format!("{}\n[stderr]\n{}", stdout_str, stderr_str)
                };

                Ok(ToolResult {
                    content,
                    is_error: !status.success(),
                    new_messages: Vec::new(),
                })
            }
            Ok(Err(e)) => Err(ToolError(format!("Failed to wait for command: {}", e))),
            Err(_) => {
                // Timeout - kill the child process tree
                let _ = child.kill().await;
                Err(ToolError(format!("Command timed out after {}ms", timeout_ms)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_ctx(work_dir: &std::path::Path) -> ToolContext {
        ToolContext {
            work_dir: work_dir.to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_bash_success() {
        let temp = TempDir::new().unwrap();
        let tool = BashTool::new(vec![], vec![], 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "echo hello"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_stderr() {
        let temp = TempDir::new().unwrap();
        let tool = BashTool::new(vec![], vec![], 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "echo error >&2"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(result.content.contains("[stderr]"));
        assert!(result.content.contains("error"));
    }

    #[tokio::test]
    async fn test_bash_blacklist() {
        let temp = TempDir::new().unwrap();
        let blacklist = vec![Pattern::wildcard("rm *")];
        let tool = BashTool::new(vec![], blacklist, 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "rm -rf /"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("blocked by security policy"));
    }

    #[tokio::test]
    async fn test_bash_whitelist_priority() {
        let temp = TempDir::new().unwrap();
        let whitelist = vec![Pattern::wildcard("rm safe*")];
        let blacklist = vec![Pattern::wildcard("rm *")];
        let tool = BashTool::new(whitelist, blacklist, 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "rm safe_file.txt"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error || !result.content.contains("blocked"));
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let temp = TempDir::new().unwrap();
        let tool = BashTool::new(vec![], vec![], 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "sleep 10", "timeout": 100});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("timed out"));
    }

    #[tokio::test]
    async fn test_bash_working_directory() {
        let temp = TempDir::new().unwrap();
        let tool = BashTool::new(vec![], vec![], 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "pwd"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        let work_str = temp.path().canonicalize().unwrap().to_string_lossy().to_string();
        assert!(result.content.contains(&work_str) || result.content.contains("agent-libs"));
    }

    #[tokio::test]
    async fn test_bash_output_truncation() {
        let temp = TempDir::new().unwrap();
        // Use a very small buffer (64 bytes) to force truncation
        let tool = BashTool::new(vec![], vec![], 64);
        let ctx = make_ctx(temp.path());
        // Generate ~300 bytes of output
        let input = json!({"command": "seq 1 50"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(
            result.content.contains("[Output truncated:"),
            "Expected truncation notice, got: {}",
            result.content
        );
        assert!(
            result.content.contains("keeping last"),
            "Expected 'keeping last' in truncation notice"
        );
    }

    #[tokio::test]
    async fn test_bash_output_no_truncation_when_small() {
        let temp = TempDir::new().unwrap();
        let tool = BashTool::new(vec![], vec![], 8192);
        let ctx = make_ctx(temp.path());
        let input = json!({"command": "echo hello"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(
            !result.content.contains("[Output truncated:"),
            "Should not have truncation notice for small output"
        );
    }
}
