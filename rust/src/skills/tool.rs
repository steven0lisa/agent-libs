//! SkillTool -- Tool implementation that loads skills and returns their content.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::skills::loader::SkillLoader;
use crate::skills::substitution::substitute_variables;
use crate::skills::types::SkillInfo;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};

/// Tool to load a skill and get its instructions.
///
/// Skills provide specialized capabilities for specific tasks.
pub struct SkillTool {
    loader: SkillLoader,
}

impl SkillTool {
    /// Create a new SkillTool backed by the given loader.
    pub fn new(loader: SkillLoader) -> Self {
        Self { loader }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        "Load a skill and get its instructions. \
         Skills provide specialized capabilities for specific tasks."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "The name of the skill to load"
                },
                "args": {
                    "type": "string",
                    "description": "Optional arguments passed to the skill via $ARGUMENTS variable substitution"
                }
            },
            "required": ["skill"]
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let skill_name = input["skill"]
            .as_str()
            .ok_or_else(|| ToolError("\"skill\" is required".to_string()))?;

        let args = input["args"].as_str();

        // Find the skill -- loader uses interior mutability so &self is fine
        let skill = match self.loader.find_by_name(skill_name) {
            Some(s) => s,
            None => {
                let available: Vec<String> = self
                    .loader
                    .discover_all()
                    .iter()
                    .map(|s| s.metadata.name.clone())
                    .collect();

                let available_str = if available.is_empty() {
                    "(none)".to_string()
                } else {
                    available.join(", ")
                };

                return Err(ToolError(format!(
                    "Skill not found: \"{}\". Available skills: {}",
                    skill_name, available_str
                )));
            }
        };

        let processed_content = substitute_variables(
            &skill.content,
            args,
            Some(&skill.dir_path.to_string_lossy()),
        );

        let result = build_skill_result(&skill, &processed_content);
        Ok(ToolResult::success(result))
    }
}

/// Build the formatted result string from a skill.
fn build_skill_result(skill: &SkillInfo, content: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("## Skill: {}", skill.metadata.name));

    if !skill.metadata.description.is_empty() {
        parts.push(format!("Description: {}", skill.metadata.description));
    }
    if !skill.metadata.when_to_use.is_empty() {
        parts.push(format!("When to use: {}", skill.metadata.when_to_use));
    }

    parts.push(String::new());
    parts.push(content.to_string());

    // Add allowed_tools soft constraint hint
    if !skill.metadata.allowed_tools.is_empty() {
        parts.push(String::new());
        parts.push(format!(
            "Note: When following this skill's instructions, only use these tools: {}",
            skill.metadata.allowed_tools.join(", ")
        ));
    }

    // Add fork mode soft marker
    if skill.metadata.context == "fork" {
        parts.push(String::new());
        parts.push("This skill should be executed in a fork context.".to_string());
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::types::SkillLoaderOptions;
    use std::fs;
    use std::path::Path;

    fn create_test_loader(skills_dir: &Path) -> SkillLoader {
        let skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: A test skill\nwhen_to_use: When testing\n---\n\nDo something with $ARGUMENTS",
        )
        .unwrap();

        SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(skills_dir.to_path_buf()),
            include_project_skills: false,
            project_dir: None,
        }))
    }

    #[tokio::test]
    async fn test_skill_tool_basic() {
        let dir = tempfile::TempDir::new().unwrap();
        let loader = create_test_loader(dir.path());
        let tool = SkillTool::new(loader);

        let ctx = ToolContext {
            work_dir: dir.path().to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };

        let input = json!({"skill": "test-skill"});
        let result = tool.call(input, &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Skill: test-skill"));
        assert!(result.content.contains("Description: A test skill"));
        assert!(result.content.contains("When to use: When testing"));
        assert!(result.content.contains("Do something with"));
    }

    #[tokio::test]
    async fn test_skill_tool_with_args() {
        let dir = tempfile::TempDir::new().unwrap();
        let loader = create_test_loader(dir.path());
        let tool = SkillTool::new(loader);

        let ctx = ToolContext {
            work_dir: dir.path().to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };

        let input = json!({"skill": "test-skill", "args": "some args here"});
        let result = tool.call(input, &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Do something with some args here"));
    }

    #[tokio::test]
    async fn test_skill_tool_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(skills_dir),
            include_project_skills: false,
            project_dir: None,
        }));
        let tool = SkillTool::new(loader);

        let ctx = ToolContext {
            work_dir: dir.path().to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };

        let input = json!({"skill": "nonexistent"});
        let result = tool.call(input, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.0.contains("Skill not found"));
    }

    #[tokio::test]
    async fn test_skill_tool_missing_skill_param() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(skills_dir),
            include_project_skills: false,
            project_dir: None,
        }));
        let tool = SkillTool::new(loader);

        let ctx = ToolContext {
            work_dir: dir.path().to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };

        let input = json!({});
        let result = tool.call(input, &ctx).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_skill_tool_allowed_tools_hint() {
        let dir = tempfile::TempDir::new().unwrap();
        let skill_dir = dir.path().join("restricted-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: restricted-skill\nallowed_tools: bash, read_file\n---\n\nDo something",
        )
        .unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(dir.path().to_path_buf()),
            include_project_skills: false,
            project_dir: None,
        }));
        let tool = SkillTool::new(loader);

        let ctx = ToolContext {
            work_dir: dir.path().to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };

        let input = json!({"skill": "restricted-skill"});
        let result = tool.call(input, &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("only use these tools: bash, read_file"));
    }

    #[tokio::test]
    async fn test_skill_tool_fork_mode() {
        let dir = tempfile::TempDir::new().unwrap();
        let skill_dir = dir.path().join("fork-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: fork-skill\ncontext: fork\n---\n\nDo something in a fork",
        )
        .unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(dir.path().to_path_buf()),
            include_project_skills: false,
            project_dir: None,
        }));
        let tool = SkillTool::new(loader);

        let ctx = ToolContext {
            work_dir: dir.path().to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        };

        let input = json!({"skill": "fork-skill"});
        let result = tool.call(input, &ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("should be executed in a fork context"));
    }
}
