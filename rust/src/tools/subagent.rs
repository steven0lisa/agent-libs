//! Subagent tool for creating child agents.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent::Agent;
use crate::config::AgentConfig;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::types::Message;

/// Tool to create a subagent that handles an independent task.
///
/// The subagent forks the parent agent's context (message history, tools,
/// working directory) and runs with its own turn budget.
pub struct SubAgentTool {
    parent_config: AgentConfig,
    parent_history: Vec<Message>,
}

impl SubAgentTool {
    /// Create a new SubAgentTool.
    pub fn new(parent_config: AgentConfig, parent_history: Vec<Message>) -> Self {
        Self {
            parent_config,
            parent_history,
        }
    }
}

#[async_trait]
impl Tool for SubAgentTool {
    fn name(&self) -> &str {
        "subagent"
    }

    fn description(&self) -> String {
        "Create a subagent to handle an independent task. \
         The subagent shares your context but operates independently \
         with its own tool budget.".to_string()
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Description of the task for the subagent"
                }
            },
            "required": ["task"]
        })
    }

    async fn call(
        &self,
        input: Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let task = input["task"]
            .as_str()
            .ok_or_else(|| ToolError("task is required".to_string()))?;

        if task.is_empty() {
            return Err(ToolError("task cannot be empty".to_string()));
        }

        // Fork parent config with reduced max_turns
        let sub_config = AgentConfig {
            api_key: self.parent_config.api_key.clone(),
            base_url: self.parent_config.base_url.clone(),
            model: self.parent_config.model.clone(),
            work_dir: self.parent_config.work_dir.clone(),
            max_tokens: self.parent_config.max_tokens,
            max_turns: self.parent_config.subagent_max_turns,
            max_duration_ms: self.parent_config.max_duration_ms,
            system_prompt: self.parent_config.system_prompt.clone(),
            timeout_ms: self.parent_config.timeout_ms,
            stream: false, // Subagents run non-streaming for simplicity
            output_format: self.parent_config.output_format,
            enable_subagent: false, // Prevent recursive subagents
            subagent_max_turns: self.parent_config.subagent_max_turns,
            bash_whitelist: self.parent_config.bash_whitelist.clone(),
            bash_blacklist: self.parent_config.bash_blacklist.clone(),
            curl_whitelist: self.parent_config.curl_whitelist.clone(),
            curl_blacklist: self.parent_config.curl_blacklist.clone(),
            enable_skills: false,
            skills_dir: None,
            include_project_skills: false,
            skills_project_dir: None,
            auto_compact: self.parent_config.auto_compact,
            context_window_size: self.parent_config.context_window_size,
            auto_compact_threshold_pct: self.parent_config.auto_compact_threshold_pct,
            allowed_read_dirs: self.parent_config.allowed_read_dirs.clone(),
            allowed_write_dirs: self.parent_config.allowed_write_dirs.clone(),
            extra_env: self.parent_config.extra_env.clone(),
            bash_output_buffer_size: self.parent_config.bash_output_buffer_size,
        };

        // Create subagent with forked context
        let mut subagent = Agent::new(sub_config);

        // Copy parent's message history for context
        subagent.set_message_history(self.parent_history.clone());

        // Run subagent - collect all events
        let mut final_content = String::new();
        let mut error_message = String::new();

        match subagent.run(task).await {
            Ok(mut rx) => {
                while let Some(event) = rx.recv().await {
                    match event {
                        crate::types::Event::Complete { final_content: content } => {
                            final_content = content;
                        }
                        crate::types::Event::Error { message } => {
                            error_message = message;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                return Ok(ToolResult::error(format!("Subagent failed: {}", e)));
            }
        }

        if !error_message.is_empty() {
            return Ok(ToolResult::error(format!(
                "Subagent error: {}",
                error_message
            )));
        }

        Ok(ToolResult::success(format!(
            "Subagent completed. Result:\n{}",
            final_content
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_subagent_empty_task() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let config = AgentConfig {
            work_dir: work_dir.clone(),
            ..Default::default()
        };

        let tool = SubAgentTool::new(config, vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({"task": ""});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subagent_missing_task() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().to_path_buf();

        let config = AgentConfig {
            work_dir: work_dir.clone(),
            ..Default::default()
        };

        let tool = SubAgentTool::new(config, vec![]);
        let ctx = ToolContext {
            work_dir: work_dir.clone(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };
        let input = json!({});

        let result = tool.call(input, &ctx).await;
        assert!(result.is_err());
    }
}
