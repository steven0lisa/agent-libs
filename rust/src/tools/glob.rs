//! Glob tool for finding files matching a pattern.

use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolContext, ToolError, ToolResult};

/// Maximum number of results to return.
const MAX_RESULTS: usize = 1000;

/// Maximum directory depth to traverse.
const MAX_DEPTH: usize = 20;

/// Directories to skip during traversal.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".svn",
    ".hg",
    "vendor",
    "dist",
    "build",
    ".next",
    ".cache",
];

/// Tool to find files matching a glob pattern.
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> String {
        "Find files matching a glob pattern.".to_string()
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g., '**/*.rs', 'src/**/*.ts', '*.txt')"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to working directory)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError("pattern is required".to_string()))?;

        let search_path = input["path"]
            .as_str()
            .map(|p| {
                let p = PathBuf::from(p);
                if p.is_absolute() {
                    p
                } else {
                    ctx.work_dir.join(p)
                }
            })
            .unwrap_or_else(|| ctx.work_dir.clone());

        if !search_path.exists() {
            return Ok(ToolResult::error(format!(
                "Path '{}' does not exist",
                search_path.display()
            )));
        }

        if !search_path.is_dir() {
            return Ok(ToolResult::error(format!(
                "Path '{}' is not a directory",
                search_path.display()
            )));
        }

        let mut results: Vec<String> = Vec::new();
        glob_search(&search_path, pattern, &mut results, 0);

        if results.is_empty() {
            Ok(ToolResult::success(format!(
                "No files matching '{}' found in '{}'",
                pattern,
                search_path.display()
            )))
        } else {
            let mut output = format!("Found {} files:\n\n", results.len());
            for (i, path) in results.iter().enumerate() {
                if i >= MAX_RESULTS {
                    output.push_str(&format!(
                        "\n... and more (truncated at {} results)",
                        MAX_RESULTS
                    ));
                    break;
                }
                output.push_str(path);
                output.push('\n');
            }
            Ok(ToolResult::success(output))
        }
    }
}

/// Search directory tree for files matching the glob pattern.
fn glob_search(dir: &Path, pattern: &str, results: &mut Vec<String>, depth: usize) {
    if depth > MAX_DEPTH || results.len() >= MAX_RESULTS {
        return;
    }

    // Parse the glob pattern into components
    let parts = split_glob(pattern);

    walk_and_match(dir, &parts, results, depth);
}

/// Split a glob pattern into components.
/// e.g., "**/*.rs" -> ["**", "*.rs"]
/// e.g., "src/**/*.ts" -> ["src", "**", "*.ts"]
fn split_glob(pattern: &str) -> Vec<String> {
    // Normalize separators
    let normalized = pattern.replace('\\', "/");
    let parts: Vec<String> = normalized
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if parts.is_empty() {
        vec!["*".to_string()]
    } else {
        parts
    }
}

/// Recursively walk the directory tree and match against glob components.
fn walk_and_match(dir: &Path, parts: &[String], results: &mut Vec<String>, depth: usize) {
    if results.len() >= MAX_RESULTS || depth > MAX_DEPTH {
        return;
    }

    if parts.is_empty() {
        // We've matched all parts, add the directory
        results.push(dir.display().to_string());
        return;
    }

    let current_part = &parts[0];
    let remaining = &parts[1..];

    if current_part == "**" {
        // ** matches zero or more directories
        // First, try matching remaining parts from current directory (zero directories)
        walk_and_match(dir, remaining, results, depth + 1);

        // Then, try descending into each subdirectory
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                // Continue with ** in subdirectory
                walk_and_match(&path, parts, results, depth + 1);
            }
        }
    } else {
        // Match current directory entries against current_part
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if path.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                // Check if directory name matches
                if glob_match(current_part, &name_str) {
                    if remaining.is_empty() {
                        results.push(format!("{}/", path.display()));
                    } else {
                        walk_and_match(&path, remaining, results, depth + 1);
                    }
                }
            } else if path.is_file() {
                // Check if file name matches (only if this is the last part or followed by **)
                if remaining.is_empty() && glob_match(current_part, &name_str) {
                    results.push(path.display().to_string());
                }
            }
        }
    }
}

/// Check if a directory should be skipped.
fn should_skip_dir(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        SKIP_DIRS.contains(&name) || name.starts_with('.')
    } else {
        false
    }
}

/// Glob-style matching for file/directory names.
/// Supports * (any sequence) and ? (any single char).
fn glob_match(pattern: &str, text: &str) -> bool {
    let p_chars: Vec<char> = pattern.chars().collect();
    let t_chars: Vec<char> = text.chars().collect();
    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;

    while t_idx < t_chars.len() {
        if p_idx < p_chars.len()
            && (p_chars[p_idx] == '?' || p_chars[p_idx] == t_chars[t_idx])
        {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < p_chars.len() && p_chars[p_idx] == '*' {
            star_idx = Some(p_idx);
            p_idx += 1;
            match_idx = t_idx;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            t_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < p_chars.len() && p_chars[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == p_chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_context(work_dir: &std::path::Path) -> ToolContext {
        ToolContext {
            work_dir: work_dir.to_path_buf(),
            message_history: vec![],
            allowed_read_dirs: vec![],
            allowed_write_dirs: vec![],
            extra_env: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_glob_star_rs() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        fs::write(work_dir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(work_dir.join("lib.rs"), "fn lib() {}").unwrap();
        fs::write(work_dir.join("readme.md"), "# readme").unwrap();

        let tool = GlobTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "*.rs"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("main.rs"));
        assert!(result.content.contains("lib.rs"));
        assert!(!result.content.contains("readme.md"));
    }

    #[tokio::test]
    async fn test_glob_double_star() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        let sub = work_dir.join("src");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("main.rs"), "fn main() {}").unwrap();

        let tool = GlobTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "**/*.rs"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("main.rs"));
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        fs::write(work_dir.join("test.txt"), "hello").unwrap();

        let tool = GlobTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "*.rs"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("No files matching"));
    }

    #[tokio::test]
    async fn test_glob_with_path() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        let sub = work_dir.join("src");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
        fs::write(work_dir.join("top.txt"), "top level").unwrap();

        let tool = GlobTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "*.rs", "path": "src"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("main.rs"));
    }

    #[test]
    fn test_glob_match_function() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.py"));
        assert!(glob_match("test.*", "test.rs"));
        assert!(glob_match("test.*", "test.txt"));
        assert!(glob_match("*.test.*", "a.test.rs"));
        assert!(glob_match("?", "a"));
        assert!(!glob_match("?", "ab"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("foo*", "foobar"));
        assert!(glob_match("*bar", "foobar"));
    }
}
