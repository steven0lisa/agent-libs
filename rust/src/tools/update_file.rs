//! Update file tool.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::utils::security::resolve_safe_path;

/// Tool to update a file by replacing old_string with new_string.
pub struct UpdateFileTool;

#[async_trait]
impl Tool for UpdateFileTool {
    fn name(&self) -> &str {
        "update_file"
    }

    fn description(&self) -> &str {
        "Update a file by replacing old_string with new_string."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "old_string": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                },
                "replace_all": {
                    "type": "boolean",
                    "default": false,
                    "description": "Replace all occurrences"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn call(
        &self,
        input: Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError("file_path is required".to_string()))?;
        let old_str = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError("old_string is required".to_string()))?;
        let new_str = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError("new_string is required".to_string()))?;
        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        let resolved = resolve_safe_path(file_path, &ctx.work_dir)
            .map_err(|e| ToolError(e))?;

        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| ToolError(format!("Failed to read file: {}", e)))?;

        let new_content = if replace_all {
            content.replace(old_str, new_str)
        } else {
            content.replacen(old_str, new_str, 1)
        };

        if new_content == content {
            return Err(ToolError("old_string not found in file".to_string()));
        }

        tokio::fs::write(&resolved, new_content)
            .await
            .map_err(|e| ToolError(format!("Failed to write file: {}", e)))?;

        Ok(ToolResult::success(format!(
            "File updated: {}",
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
    async fn test_update_file_single_replace() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();
        fs::write(work_dir.join("test.txt"), "hello world hello").unwrap();

        let tool = UpdateFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
        };
        let input = json!({
            "file_path": "test.txt",
            "old_string": "hello",
            "new_string": "hi"
        });

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);

        let content = fs::read_to_string(work_dir.join("test.txt")).unwrap();
        assert_eq!(content, "hi world hello");
    }

    #[tokio::test]
    async fn test_update_file_replace_all() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();
        fs::write(work_dir.join("test.txt"), "hello world hello").unwrap();

        let tool = UpdateFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
        };
        let input = json!({
            "file_path": "test.txt",
            "old_string": "hello",
            "new_string": "hi",
            "replace_all": true
        });

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);

        let content = fs::read_to_string(work_dir.join("test.txt")).unwrap();
        assert_eq!(content, "hi world hi");
    }

    #[tokio::test]
    async fn test_update_file_not_found() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = UpdateFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
        };
        let input = json!({
            "file_path": "nonexistent.txt",
            "old_string": "old",
            "new_string": "new"
        });

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_file_old_string_not_found() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();
        fs::write(work_dir.join("test.txt"), "hello world").unwrap();

        let tool = UpdateFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
        };
        let input = json!({
            "file_path": "test.txt",
            "old_string": "notfound",
            "new_string": "new"
        });

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("not found"));
    }

    #[tokio::test]
    async fn test_update_file_outside_work_dir() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let tool = UpdateFileTool;
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
        };
        let input = json!({
            "file_path": "../outside.txt",
            "old_string": "old",
            "new_string": "new"
        });

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }
}
