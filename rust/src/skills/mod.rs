//! Skill system -- load and invoke skill instructions from SKILL.md files.
//!
//! Skills are collections of instructions stored in `~/.claude/skills/<name>/SKILL.md`.
//! They provide specialized capabilities for specific tasks.

pub mod loader;
pub mod substitution;
pub mod tool;
pub mod types;
pub mod yaml;

pub use loader::SkillLoader;
pub use tool::SkillTool;
pub use types::{SkillInfo, SkillLoaderOptions, SkillMetadata};
