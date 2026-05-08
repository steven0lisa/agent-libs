//! Grep tool for searching patterns in files.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolContext, ToolError, ToolResult};

/// Maximum number of matches to return.
const MAX_MATCHES: usize = 100;

/// Maximum number of lines per file to check.
const MAX_LINES_PER_FILE: usize = 5000;

/// Maximum file size to read (1 MB).
const MAX_FILE_SIZE: u64 = 1_000_000;

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

/// Tool to search for patterns in files.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for patterns in files using regular expressions."
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
                    "description": "The pattern to search for (supports basic regex)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (defaults to working directory)"
                },
                "include": {
                    "type": "string",
                    "description": "File name pattern to include (e.g., '*.rs', '*.py')"
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Whether to ignore case when matching",
                    "default": false
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

        let include_pattern = input["include"].as_str();
        let ignore_case = input["ignore_case"].as_bool().unwrap_or(false);

        let search_pattern = if ignore_case {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };

        // Check if regex-like patterns are used (basic support)
        let use_regex = pattern.contains('^')
            || pattern.contains('$')
            || pattern.contains('|')
            || pattern.contains('(')
            || pattern.contains('[')
            || pattern.contains('+')
            || pattern.contains('?');

        let mut matches: Vec<String> = Vec::new();
        let mut files_searched: usize = 0;

        if search_path.is_file() {
            search_file(
                &search_path,
                &search_pattern,
                include_pattern,
                ignore_case,
                use_regex,
                &mut matches,
                &mut files_searched,
            );
        } else if search_path.is_dir() {
            search_dir(
                &search_path,
                &search_pattern,
                include_pattern,
                ignore_case,
                use_regex,
                &mut matches,
                &mut files_searched,
            );
        } else {
            return Ok(ToolResult::error(format!(
                "Path '{}' does not exist",
                search_path.display()
            )));
        }

        if matches.is_empty() {
            Ok(ToolResult::success(format!(
                "No matches found for '{}' in '{}' (searched {} files)",
                pattern,
                search_path.display(),
                files_searched
            )))
        } else {
            let mut result = format!(
                "Found {} matches in {} files (searched {} files):\n\n",
                matches.len(),
                matches.iter().filter(|m| !m.is_empty()).count(),
                files_searched
            );
            // Limit output
            for (i, m) in matches.iter().enumerate() {
                if i >= MAX_MATCHES {
                    result.push_str(&format!(
                        "\n... and more (truncated at {} matches)",
                        MAX_MATCHES
                    ));
                    break;
                }
                result.push_str(m);
                result.push('\n');
            }
            Ok(ToolResult::success(result))
        }
    }
}

/// Search a single file for the pattern.
fn search_file(
    path: &Path,
    pattern: &str,
    include_pattern: Option<&str>,
    ignore_case: bool,
    use_regex: bool,
    matches: &mut Vec<String>,
    files_searched: &mut usize,
) {
    // Check include filter
    if let Some(inc) = include_pattern {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if !glob_match(inc, name) {
                return;
            }
        }
    }

    // Skip binary-like files and large files
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let binary_exts = [
            "exe", "bin", "so", "dylib", "dll", "o", "a", "lib", "png", "jpg",
            "jpeg", "gif", "bmp", "ico", "webp", "mp3", "mp4", "avi", "mov",
            "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "pdf", "doc", "docx",
            "xls", "xlsx", "ppt", "pptx", "woff", "woff2", "ttf", "eot", "class",
            "jar", "war", "pyc", "pyd", "wasm",
        ];
        if binary_exts.contains(&ext.to_lowercase().as_str()) {
            return;
        }
    }

    // Check file size
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > MAX_FILE_SIZE {
            return;
        }
    }

    *files_searched += 1;

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);
    let path_str = path.display().to_string();
    let mut file_had_matches = false;

    for (line_num, line_result) in reader.lines().enumerate() {
        if line_num >= MAX_LINES_PER_FILE {
            break;
        }
        if matches.len() >= MAX_MATCHES {
            break;
        }

        let line = match line_result {
            Ok(l) => l,
            Err(_) => break, // Probably binary, stop
        };

        let line_to_search = if ignore_case {
            line.to_lowercase()
        } else {
            line.clone()
        };

        let found = if use_regex {
            simple_regex_match(pattern, &line_to_search)
        } else {
            line_to_search.contains(pattern)
        };

        if found {
            if !file_had_matches {
                file_had_matches = true;
            }
            // Trim long lines
            let display_line = if line.len() > 200 {
                format!("{}...", &line[..200])
            } else {
                line
            };
            matches.push(format!("{}:{}: {}", path_str, line_num + 1, display_line));
        }
    }
}

/// Recursively search a directory for the pattern.
fn search_dir(
    dir: &Path,
    pattern: &str,
    include_pattern: Option<&str>,
    ignore_case: bool,
    use_regex: bool,
    matches: &mut Vec<String>,
    files_searched: &mut usize,
) {
    if matches.len() >= MAX_MATCHES {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    // Sort for deterministic output
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        if matches.len() >= MAX_MATCHES {
            return;
        }

        let path = entry.path();

        // Skip certain directories
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if SKIP_DIRS.contains(&name) || name.starts_with('.') {
                    continue;
                }
            }
            search_dir(
                &path,
                pattern,
                include_pattern,
                ignore_case,
                use_regex,
                matches,
                files_searched,
            );
        } else if path.is_file() {
            search_file(
                &path,
                pattern,
                include_pattern,
                ignore_case,
                use_regex,
                matches,
                files_searched,
            );
        }
    }
}

/// Simple glob-style matching for file names.
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

/// Very simple regex matching supporting ^, $, |, ., *, +, ?
/// This is a basic implementation for common patterns.
fn simple_regex_match(pattern: &str, text: &str) -> bool {
    // Handle alternation (|)
    if pattern.contains('|') {
        let wrapped = if pattern.starts_with('(') && pattern.ends_with(')') {
            &pattern[1..pattern.len() - 1]
        } else {
            pattern
        };
        return wrapped.split('|').any(|alt| simple_regex_match(alt.trim(), text));
    }

    // Handle anchors
    let pattern = if let Some(p) = pattern.strip_prefix('^') {
        if let Some(p) = p.strip_suffix('$') {
            // Exact match: ^...$
            return text == p;
        }
        // Must match from start
        return text.starts_with(p);
    } else if let Some(p) = pattern.strip_suffix('$') {
        return text.ends_with(p);
    } else {
        pattern
    };

    // For simple patterns, use contains
    // Replace . with any char wildcard for basic support
    if !pattern.contains('.') && !pattern.contains('+') && !pattern.contains('?') {
        return text.contains(pattern);
    }

    // Basic wildcard matching with . support
    regex_wildcard_match(pattern, text)
}

/// Match a pattern with . (any char) and * (zero or more) wildcards.
fn regex_wildcard_match(pattern: &str, text: &str) -> bool {
    // Convert regex wildcards to simple matching
    // . matches any single char, .* matches any sequence
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();

    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;

    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 2][0];
        }
    }

    for i in 1..=p.len() {
        for j in 1..=t.len() {
            if p[i - 1] == '.' || p[i - 1] == t[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else if p[i - 1] == '*' {
                dp[i][j] = dp[i - 2][j]; // Zero occurrences
                if p[i - 2] == '.' || p[i - 2] == t[j - 1] {
                    dp[i][j] = dp[i][j] || dp[i][j - 1]; // One or more
                }
            } else if p[i - 1] == '+' {
                if p.len() >= 2 {
                    let prev = p[i - 2];
                    if prev == '.' || prev == t[j - 1] {
                        dp[i][j] = dp[i - 1][j - 1] || dp[i][j - 1];
                    }
                }
            } else if p[i - 1] == '?' {
                dp[i][j] = dp[i - 1][j] || (j > 0 && (p[i - 2] == '.' || p[i - 2] == t[j - 1]) && dp[i - 2][j - 1]);
            }
        }
    }

    if dp[p.len()][t.len()] {
        return true;
    }

    // If not an exact match, try substring search with the pattern
    // This handles the case where the regex should match anywhere in the text
    for start in 0..t.len() {
        let mut dp2 = vec![vec![false; t.len() - start + 1]; p.len() + 1];
        dp2[0][0] = true;

        for i in 1..=p.len() {
            if p[i - 1] == '*' {
                dp2[i][0] = dp2.get(i - 2).and_then(|r| r.get(0)).copied().unwrap_or(false);
            }
        }

        for i in 1..=p.len() {
            for j in 1..=t.len() - start {
                if p[i - 1] == '.' || p[i - 1] == t[start + j - 1] {
                    dp2[i][j] = dp2[i - 1][j - 1];
                } else if p[i - 1] == '*' {
                    dp2[i][j] = dp2[i - 2][j];
                    if p[i - 2] == '.' || p[i - 2] == t[start + j - 1] {
                        dp2[i][j] = dp2[i][j] || dp2[i][j - 1];
                    }
                }
            }
        }

        if dp2[p.len()][t.len() - start] {
            return true;
        }
    }

    false
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
        }
    }

    #[tokio::test]
    async fn test_grep_simple_pattern() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        fs::write(work_dir.join("test.txt"), "hello world\nfoo bar\nhello rust").unwrap();

        let tool = GrepTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "hello"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("hello world"));
        assert!(result.content.contains("hello rust"));
    }

    #[tokio::test]
    async fn test_grep_ignore_case() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        fs::write(work_dir.join("test.txt"), "Hello World\nFOO BAR").unwrap();

        let tool = GrepTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "hello", "ignore_case": true});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_grep_include_filter() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        fs::write(work_dir.join("test.rs"), "fn main() {}").unwrap();
        fs::write(work_dir.join("test.py"), "def main():").unwrap();

        let tool = GrepTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "main", "include": "*.rs"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("test.rs"));
        assert!(!result.content.contains("test.py"));
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        fs::write(work_dir.join("test.txt"), "hello world").unwrap();

        let tool = GrepTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "nonexistent"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("No matches found"));
    }

    #[tokio::test]
    async fn test_grep_subdirectory() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();
        let sub = work_dir.join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.txt"), "found it").unwrap();

        let tool = GrepTool;
        let ctx = make_context(work_dir);
        let input = json!({"pattern": "found"});

        let result = tool.call(input, &ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("found it"));
        assert!(result.content.contains("nested.txt"));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.py"));
        assert!(glob_match("test.*", "test.rs"));
        assert!(glob_match("?.txt", "a.txt"));
        assert!(!glob_match("?.txt", "ab.txt"));
        assert!(glob_match("*", "anything"));
    }
}
