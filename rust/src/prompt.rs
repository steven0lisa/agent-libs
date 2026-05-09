//! System prompt builder.

use std::collections::HashMap;
use std::sync::Arc;

use crate::tool::Tool;

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful software engineering assistant. You have access to tools that let you interact with the file system and execute commands.

## Tool Use

You will be provided with a set of tools. When you need to perform an action, use the appropriate tool by outputting a tool_use block.

Tool use format:
- Each tool has a name and input parameters defined by a JSON schema
- To use a tool, output a content block with type "tool_use"
- The tool_use block must include: name (tool name), id (unique identifier), input (parameters as JSON object)

After using a tool, the user message will contain a tool_result block with the execution result.
- If the result indicates success, continue with your analysis or next steps
- If the result indicates an error, analyze the error and decide whether to retry, use a different approach, or ask the user for clarification

## File Operations

When working with files:
- Always read a file before editing it
- When editing files, use update_file tool with old_string/new_string for precise changes
- Use write_file to create new files
- Use read_file to inspect file contents
- All file operations are restricted to the working directory and its subdirectories

## Command Execution

When executing commands via bash:
- Prefer read-only commands for exploration (ls, grep, find, cat, etc.)
- Be careful with destructive commands (rm, dd, etc.)
- Always describe what the command does before executing it
"#;

const SUBAGENT_PROMPT: &str = r#"

## Subagent Capability

You can create subagents to handle independent tasks in parallel. A subagent is a separate agent instance that shares your context but operates independently.

When to use subagents:
- When you need to explore multiple approaches simultaneously
- When a task can be cleanly decomposed into independent sub-tasks
- When you want to parallelize read-only exploration

How to create a subagent:
- Use the `subagent` tool with a `task` describing what the subagent should do
- The subagent will execute with its own tool budget (max turns: {max_turns})
- The subagent inherits your current context (files read, tool state, etc.)
- Results are returned as tool_result when the subagent completes

Important:
- Subagents run independently and do not modify your state
- File operations in subagents are still restricted to the working directory
- Prefer subagents for exploration; keep modifications in the main agent
"#;

/// Build the system prompt with tool descriptions.
pub fn build_system_prompt(
    tools: &HashMap<String, Arc<dyn Tool>>,
    custom_prompt: Option<&str>,
    enable_subagent: bool,
    subagent_max_turns: usize,
) -> String {
    let mut tool_descriptions = Vec::new();
    for (name, tool) in tools {
        tool_descriptions.push(format!(
            "### {}\nDescription: {}\nInput Schema: {}",
            name,
            tool.description(),
            tool.input_schema()
        ));
    }

    let tools_section = format!("\n\n## Available Tools\n\n{}", tool_descriptions.join("\n\n"));

    let mut prompt = DEFAULT_SYSTEM_PROMPT.to_string();
    prompt.push_str(&tools_section);

    if enable_subagent {
        prompt.push_str(&SUBAGENT_PROMPT.replace("{max_turns}", &subagent_max_turns.to_string()));
    }

    if let Some(custom) = custom_prompt {
        prompt.push_str(&format!("\n\n## Custom Instructions\n\n{}", custom));
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestTool;

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn input_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                }
            })
        }

        async fn call(
            &self,
            _input: serde_json::Value,
            _ctx: &crate::tool::ToolContext,
        ) -> Result<crate::tool::ToolResult, crate::tool::ToolError> {
            Ok(crate::tool::ToolResult::success("ok"))
        }
    }

    #[test]
    fn test_build_system_prompt_basic() {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        tools.insert("test_tool".to_string(), Arc::new(TestTool));

        let prompt = build_system_prompt(&tools, None, false, 50);

        assert!(prompt.contains("You are a helpful software engineering assistant"));
        assert!(prompt.contains("test_tool"));
        assert!(prompt.contains("A test tool"));
        assert!(!prompt.contains("Subagent"));
    }

    #[test]
    fn test_build_system_prompt_with_subagent() {
        let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

        let prompt = build_system_prompt(&tools, None, true, 50);

        assert!(prompt.contains("Subagent Capability"));
        assert!(prompt.contains("50"));
    }

    #[test]
    fn test_build_system_prompt_with_custom() {
        let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

        let prompt = build_system_prompt(&tools, Some("Be extra careful"), false, 50);

        assert!(prompt.contains("Custom Instructions"));
        assert!(prompt.contains("Be extra careful"));
    }

    #[test]
    fn test_build_system_prompt_tools_section() {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        tools.insert("test_tool".to_string(), Arc::new(TestTool));

        let prompt = build_system_prompt(&tools, None, false, 50);

        assert!(prompt.contains("## Available Tools"));
        assert!(prompt.contains("### test_tool"));
        assert!(prompt.contains("Input Schema"));
    }

    #[test]
    fn test_build_system_prompt_no_skills_section() {
        let tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

        let prompt = build_system_prompt(&tools, None, false, 50);

        // Skills should NOT appear in system prompt (moved to user message injection)
        assert!(!prompt.contains("Available Skills"));
    }
}
