//! Types for the skill system.

use std::path::PathBuf;

/// Metadata parsed from a skill's YAML frontmatter.
#[derive(Debug, Clone, Default)]
pub struct SkillMetadata {
    /// Skill name used to invoke it.
    pub name: String,
    /// Short description of what the skill does.
    pub description: String,
    /// Guidance on when this skill should be used.
    pub when_to_use: String,
    /// Tools this skill is allowed to use (empty = no restriction).
    pub allowed_tools: Vec<String>,
    /// Preferred model for this skill.
    pub model: String,
    /// Skill context mode (inline or fork).
    pub context: String,
    /// Skill version.
    pub version: String,
}

/// Information about a discovered skill.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    /// Parsed frontmatter metadata.
    pub metadata: SkillMetadata,
    /// Skill content after stripping frontmatter.
    pub content: String,
    /// Absolute path to the SKILL.md file.
    pub file_path: PathBuf,
    /// Absolute path to the skill directory.
    pub dir_path: PathBuf,
}

/// Options for the SkillLoader.
#[derive(Debug, Clone)]
pub struct SkillLoaderOptions {
    /// Directory to scan for skills (default: ~/.claude/skills).
    pub skills_dir: Option<PathBuf>,
    /// Whether to also scan project-local skills.
    pub include_project_skills: bool,
    /// Project directory to scan for .claude/skills/ (default: cwd).
    pub project_dir: Option<PathBuf>,
}
