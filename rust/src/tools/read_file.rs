//! Read file tool.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::utils::security::resolve_safe_path;

/// Tool to read file contents.
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read file contents from the working directory. Supports text, images, PDFs."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file (relative to working directory or absolute)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["file_path"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError("file_path is required".to_string()))?;

        let resolved = resolve_safe_path(file_path, &ctx.work_dir)
            .map_err(|e| ToolError(e))?;

        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| ToolError(format!("Failed to read file: {}", e)))?;

        // Handle offset and limit
        let offset = input["offset"].as_u64().unwrap_or(0) as usize;
        let limit = input["limit"].as_u64().map(|v| v as usize);

        let lines: Vec<&str> = content.lines().collect();
        let start = offset.saturating_sub(1); // Convert 1-based to 0-based
        let end = limit.map(|l| (start + l).min(lines.len())).unwrap_or(lines.len());

        let result = if start < lines.len() {
            lines[start..end].join("\n")
        } else {
            String::new()
        };

        Ok(ToolResult::success(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_success() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();
        let file_path = work_dir.join("test.txt");
        fs::write(&file_path, "line1\nline2\nline3").unwrap();

        let tool = ReadFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"file_path": "test.txt"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content, "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_read_file_with_offset_and_limit() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();
        let file_path = work_dir.join("test.txt");
        fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();

        let tool = ReadFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"file_path": "test.txt", "offset": 2, "limit": 2});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content, "line2\nline3");
    }

    #[tokio::test]
    async fn test_read_file_outside_work_dir() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = ReadFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"file_path": "../outside.txt"});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = ReadFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"file_path": "nonexistent.txt"});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }
}
