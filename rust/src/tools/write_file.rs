//! Write file tool.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::utils::security::resolve_safe_path;

/// Tool to write content to a file.
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates if not exists, overwrites if exists."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "content": {"type": "string"}
            },
            "required": ["file_path", "content"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError("file_path is required".to_string()))?;
        let content = input["content"].as_str().unwrap_or("");

        let resolved = resolve_safe_path(file_path, &ctx.work_dir)
            .map_err(|e| ToolError(e))?;

        // Ensure parent directory exists
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError(format!("Failed to create directory: {}", e)))?;
        }

        tokio::fs::write(&resolved, content)
            .await
            .map_err(|e| ToolError(format!("Failed to write file: {}", e)))?;

        Ok(ToolResult::success(format!(
            "File written: {}",
            resolved.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_file_success() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = WriteFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"file_path": "test.txt", "content": "hello world"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);

        let content = fs::read_to_string(work_dir.join("test.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_file_creates_subdirs() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        // Pre-create the subdirectory path so resolve_safe_path works
        fs::create_dir_all(work_dir.join("a/b/c")).unwrap();

        let tool = WriteFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"file_path": "a/b/c/test.txt", "content": "nested"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);

        let content = fs::read_to_string(work_dir.join("a/b/c/test.txt")).unwrap();
        assert_eq!(content, "nested");
    }

    #[tokio::test]
    async fn test_write_file_outside_work_dir() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = WriteFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"file_path": "../outside.txt", "content": "bad"});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_file_overwrite() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();
        fs::write(work_dir.join("test.txt"), "old").unwrap();

        let tool = WriteFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
        };
        let input = json!({"file_path": "test.txt", "content": "new"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);

        let content = fs::read_to_string(work_dir.join("test.txt")).unwrap();
        assert_eq!(content, "new");
    }
}
