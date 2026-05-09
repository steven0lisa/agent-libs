//! Minimal YAML frontmatter parser for SKILL.md files.
//!
//! This parser handles the subset of YAML required for skill metadata
//! without pulling in a full YAML crate dependency. It supports:
//! - Strings (with optional single/double quotes)
//! - Booleans (`true` / `false`)
//! - Integers
//! - Comma-separated arrays (inline `a, b, c` or block `- a, b, c`)

use std::fs;
use std::path::Path;

use crate::skills::types::SkillMetadata;

/// Error type for YAML parsing.
#[derive(Debug, thiserror::Error)]
pub enum YamlError {
    /// IO error reading the file.
    #[error("IO error: {0}")]
    Io(String),
    /// Invalid frontmatter syntax.
    #[error("Invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
}

impl From<std::io::Error> for YamlError {
    fn from(err: std::io::Error) -> Self {
        YamlError::Io(err.to_string())
    }
}

/// Parse YAML frontmatter from raw file content.
///
/// Returns a `SkillMetadata` with default values for any missing fields.
/// Returns an empty `SkillMetadata` if no frontmatter is found.
pub fn parse_frontmatter(raw: &str) -> SkillMetadata {
    let regex_start = raw.find("---\n");
    let remainder = match regex_start {
        Some(pos) => &raw[pos + 4..],
        None => return SkillMetadata::default(),
    };

    let regex_end = remainder.find("\n---");
    let yaml_content = match regex_end {
        Some(pos) => &remainder[..pos],
        None => return SkillMetadata::default(),
    };

    let mut meta = SkillMetadata::default();
    let mut pending_key: Option<String> = None;

    for line in yaml_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Handle block array items: "- a, b, c"
        if trimmed.starts_with("- ") && pending_key.is_some() {
            let rest = trimmed[2..].trim();
            current_metadata_array_push(&mut meta, &pending_key.as_ref().unwrap(), rest);
            continue;
        }

        // Reset pending key if we encounter a non-array, non-blank line
        if !trimmed.starts_with("- ") && !trimmed.is_empty() {
            pending_key = None;
        }

        let colon_pos = match trimmed.find(':') {
            Some(pos) => pos,
            None => continue,
        };

        let key = trimmed[..colon_pos].trim();
        let mut value = trimmed[colon_pos + 1..].trim();

        if key.is_empty() {
            continue;
        }

        // If value is empty, this could be a block array key (store as pending)
        if value.is_empty() {
            pending_key = Some(key.to_string());
            continue;
        }

        // Handle inline arrays: "a, b, c" but only for allowed_tools
        if key == "allowed_tools" && value.contains(',') {
            // Check it's not a quoted string with commas inside it
            let is_quoted = (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''));
            if !is_quoted {
                let arr: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                set_metadata_field(&mut meta, key, Value::Array(arr));
                continue;
            }
        }

        // Remove surrounding quotes
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = &value[1..value.len() - 1];
        }

        // Try boolean
        if value == "true" {
            set_metadata_field(&mut meta, key, Value::Bool(true));
            continue;
        }
        if value == "false" {
            set_metadata_field(&mut meta, key, Value::Bool(false));
            continue;
        }

        // Try integer
        if let Ok(num) = value.parse::<i64>() {
            set_metadata_field(&mut meta, key, Value::Int(num));
            continue;
        }

        // Default to string
        set_metadata_field(&mut meta, key, Value::String(value.to_string()));
    }

    meta
}

/// Helper enum for parsed YAML values.
enum Value {
    String(String),
    Bool(bool),
    Int(i64),
    Array(Vec<String>),
}

/// Push comma-separated items into an array metadata field (for block-style arrays).
fn current_metadata_array_push(meta: &mut SkillMetadata, key: &str, value: &str) {
    match key {
        "allowed_tools" => {
            for part in value.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    meta.allowed_tools.push(trimmed.to_string());
                }
            }
        }
        _ => {}
    }
}

/// Set a metadata field by key name.
fn set_metadata_field(meta: &mut SkillMetadata, key: &str, value: Value) {
    match key {
        "name" => {
            if let Value::String(s) = value {
                meta.name = s;
            }
        }
        "description" => {
            if let Value::String(s) = value {
                meta.description = s;
            }
        }
        "when_to_use" => {
            if let Value::String(s) = value {
                meta.when_to_use = s;
            }
        }
        "allowed_tools" => {
            if let Value::Array(arr) = value {
                // Flatten comma-separated values within the array
                let mut flat = Vec::new();
                for item in arr {
                    for part in item.split(',') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            flat.push(trimmed.to_string());
                        }
                    }
                }
                meta.allowed_tools = flat;
            } else if let Value::String(s) = value {
                meta.allowed_tools = s.split(',').map(|s| s.trim().to_string()).collect();
            }
        }
        "model" => {
            if let Value::String(s) = value {
                meta.model = s;
            }
        }
        "context" => {
            if let Value::String(s) = value {
                meta.context = s;
            }
        }
        "version" => {
            if let Value::String(s) = value {
                meta.version = s;
            }
        }
        "user_invocable" => {
            if let Value::Bool(b) = value {
                meta.user_invocable = b;
            }
        }
        _ => {}
    }
}

/// Parse a skill file, returning its metadata and content (without frontmatter).
pub fn parse_skill_file(file_path: &Path) -> Result<(SkillMetadata, String), YamlError> {
    let raw = fs::read_to_string(file_path)?;
    let metadata = parse_frontmatter(&raw);

    // Strip frontmatter block
    let content = if raw.starts_with("---\n") {
        // Find the closing ---
        let after_first = &raw[4..];
        if let Some(end_pos) = after_first.find("\n---") {
            after_first[end_pos + 4..].trim().to_string()
        } else {
            raw.trim().to_string()
        }
    } else {
        raw.trim().to_string()
    };

    Ok((metadata, content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let raw = "Just some content\nno frontmatter here";
        let meta = parse_frontmatter(raw);
        assert_eq!(meta.name, "");
    }

    #[test]
    fn test_basic_frontmatter() {
        let raw = "---\nname: my-skill\ndescription: A test skill\n---\n\nSkill content here";
        let meta = parse_frontmatter(raw);
        assert_eq!(meta.name, "my-skill");
        assert_eq!(meta.description, "A test skill");
    }

    #[test]
    fn test_boolean_values() {
        let raw = "---\ncontext: fork\n---";
        let meta = parse_frontmatter(raw);
        assert_eq!(meta.context, "fork");
    }

    #[test]
    fn test_array_values() {
        let raw = "---\nallowed_tools: bash, read_file\n---\n\ncontent";
        let meta = parse_frontmatter(raw);
        assert_eq!(meta.allowed_tools, vec!["bash", "read_file"]);
    }

    #[test]
    fn test_block_array() {
        let raw = "---\nallowed_tools:\n- bash, read_file\n---";
        let meta = parse_frontmatter(raw);
        assert_eq!(meta.allowed_tools, vec!["bash", "read_file"]);
    }

    #[test]
    fn test_quoted_value() {
        let raw = "---\nname: \"my-skill\"\ndescription: 'a test'\n---";
        let meta = parse_frontmatter(raw);
        assert_eq!(meta.name, "my-skill");
        assert_eq!(meta.description, "a test");
    }

    #[test]
    fn test_parse_skill_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "---\nname: test-skill\n---\n\nActual content here\n").unwrap();

        let (meta, content) = parse_skill_file(&path).unwrap();
        assert_eq!(meta.name, "test-skill");
        assert_eq!(content, "Actual content here");
    }

    #[test]
    fn test_parse_skill_file_no_frontmatter() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "Just content without frontmatter").unwrap();

        let (meta, content) = parse_skill_file(&path).unwrap();
        assert_eq!(meta.name, "");
        assert_eq!(content, "Just content without frontmatter");
    }

    #[test]
    fn test_user_invocable_default() {
        let raw = "---\nname: my-skill\n---\n\ncontent";
        let meta = parse_frontmatter(raw);
        assert!(meta.user_invocable); // default is true
    }

    #[test]
    fn test_user_invocable_false() {
        let raw = "---\nname: my-skill\nuser_invocable: false\n---\n\ncontent";
        let meta = parse_frontmatter(raw);
        assert!(!meta.user_invocable);
    }

    #[test]
    fn test_user_invocable_explicit_true() {
        let raw = "---\nname: my-skill\nuser_invocable: true\n---\n\ncontent";
        let meta = parse_frontmatter(raw);
        assert!(meta.user_invocable);
    }
}
