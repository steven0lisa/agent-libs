//! SkillTool -- Tool implementation that loads skills and returns their content.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::skills::loader::SkillLoader;
use crate::skills::substitution::substitute_variables;
use crate::skills::types::SkillInfo;
use crate::tool::{Tool, ToolContext, ToolError, ToolResult};
use crate::types::{ContentBlock, Message, Role};

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

    fn description(&self) -> String {
        let mut desc = "Load a skill and get its instructions. \
             Skills provide specialized capabilities for specific tasks.".to_string();
        let skills = self.loader.discover_all();
        let invocable: Vec<_> = skills.iter().filter(|s| s.metadata.user_invocable).collect();
        if !invocable.is_empty() {
            desc.push_str("\n\nAvailable skills:\n");
            for skill in &invocable {
                let line = if skill.metadata.description.is_empty() {
                    format!("- {}", skill.metadata.name)
                } else {
                    format!("- {}: {}", skill.metadata.name, skill.metadata.description)
                };
                desc.push_str(&line);
                desc.push('\n');
            }
        }
        desc
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

        let full_content = build_skill_injection_content(&skill, &processed_content);
        let brief = format!("Skill loaded: {}", skill.metadata.name);
        let injection_msg = Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text: full_content }],
        };
        Ok(ToolResult::success_with_messages(brief, vec![injection_msg]))
    }
}

/// Build the formatted injection content from a skill.
fn build_skill_injection_content(skill: &SkillInfo, content: &str) -> String {
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
            "---\nname: test-skill\ndescription: A test skill\nwhen_to_use: When testing\nuser_invocable: true\n---\n\nDo something with $ARGUMENTS",
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
        // Content is now a brief confirmation
        assert_eq!(result.content, "Skill loaded: test-skill");
        // Skill content is injected via new_messages
        assert_eq!(result.new_messages.len(), 1);
        let msg = &result.new_messages[0];
        assert_eq!(msg.role, Role::User);
        if let ContentBlock::Text { text } = &msg.content[0] {
            assert!(text.contains("Skill: test-skill"));
            assert!(text.contains("Description: A test skill"));
            assert!(text.contains("When to use: When testing"));
            assert!(text.contains("Do something with"));
        } else {
            panic!("Expected Text content block");
        }
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
        // Check args substitution in injected message
        if let ContentBlock::Text { text } = &result.new_messages[0].content[0] {
            assert!(text.contains("Do something with some args here"));
        } else {
            panic!("Expected Text content block");
        }
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
        // Check hint in injected message
        if let ContentBlock::Text { text } = &result.new_messages[0].content[0] {
            assert!(text.contains("only use these tools: bash, read_file"));
        } else {
            panic!("Expected Text content block");
        }
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
        // Check fork hint in injected message
        if let ContentBlock::Text { text } = &result.new_messages[0].content[0] {
            assert!(text.contains("should be executed in a fork context"));
        } else {
            panic!("Expected Text content block");
        }
    }

    #[tokio::test]
    async fn test_skill_tool_description_lists_skills() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create two user-invocable skills
        let skill_dir1 = dir.path().join("skill-a");
        fs::create_dir_all(&skill_dir1).unwrap();
        fs::write(
            skill_dir1.join("SKILL.md"),
            "---\nname: skill-a\ndescription: First skill\nuser_invocable: true\n---\n\nContent A",
        )
        .unwrap();

        let skill_dir2 = dir.path().join("skill-b");
        fs::create_dir_all(&skill_dir2).unwrap();
        fs::write(
            skill_dir2.join("SKILL.md"),
            "---\nname: skill-b\ndescription: Second skill\nuser_invocable: true\n---\n\nContent B",
        )
        .unwrap();

        // Create a non-invocable skill
        let skill_dir3 = dir.path().join("skill-c");
        fs::create_dir_all(&skill_dir3).unwrap();
        fs::write(
            skill_dir3.join("SKILL.md"),
            "---\nname: skill-c\ndescription: Hidden skill\nuser_invocable: false\n---\n\nContent C",
        )
        .unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(dir.path().to_path_buf()),
            include_project_skills: false,
            project_dir: None,
        }));
        let tool = SkillTool::new(loader);

        let desc = tool.description();
        assert!(desc.contains("Available skills:"));
        assert!(desc.contains("skill-a: First skill"));
        assert!(desc.contains("skill-b: Second skill"));
        assert!(!desc.contains("skill-c"));
    }
}
