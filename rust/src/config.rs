//! Agent configuration.

use std::path::PathBuf;

use crate::types::OutputFormat;

/// Pattern for whitelist/blacklist matching.
///
/// Supports wildcard (`*`, `?`) and regex patterns.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The pattern string.
    pub pattern: String,
    /// Pattern type: "wildcard" or "regex".
    pub pattern_type: PatternType,
}

/// Type of pattern matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Wildcard pattern using `*` and `?`.
    Wildcard,
    /// Regular expression pattern.
    Regex,
}

impl Pattern {
    /// Create a new wildcard pattern.
    pub fn wildcard(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            pattern_type: PatternType::Wildcard,
        }
    }

    /// Create a new regex pattern.
    pub fn regex(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            pattern_type: PatternType::Regex,
        }
    }

    /// Check if the given text matches this pattern.
    pub fn matches(&self, text: &str) -> bool {
        match self.pattern_type {
            PatternType::Wildcard => {
                // Simple wildcard matching: * matches any sequence, ? matches any single char
                let pattern = &self.pattern;
                let mut p_idx = 0;
                let mut t_idx = 0;
                let p_chars: Vec<char> = pattern.chars().collect();
                let t_chars: Vec<char> = text.chars().collect();
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
            PatternType::Regex => {
                // Use regex crate if available, otherwise simple contains check
                // For now, use a simple substring approach as fallback
                // In production, you'd want to use the `regex` crate
                text.contains(&self.pattern)
            }
        }
    }
}

/// Configuration for an Agent.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Anthropic API base URL.
    pub base_url: String,
    /// Anthropic API key.
    pub api_key: String,
    /// Model name.
    pub model: String,
    /// Working directory for file operations.
    pub work_dir: PathBuf,
    /// Maximum tokens per response.
    pub max_tokens: u32,
    /// Maximum number of turns.
    pub max_turns: usize,
    /// Maximum duration in milliseconds (0 = no limit).
    pub max_duration_ms: u64,
    /// Custom system prompt.
    pub system_prompt: Option<String>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Whether to use streaming.
    pub stream: bool,
    /// Output format.
    pub output_format: OutputFormat,
    /// Enable subagent support.
    pub enable_subagent: bool,
    /// Maximum turns for subagents.
    pub subagent_max_turns: usize,
    /// Bash command whitelist patterns.
    pub bash_whitelist: Vec<Pattern>,
    /// Bash command blacklist patterns.
    pub bash_blacklist: Vec<Pattern>,
    /// Curl URL whitelist patterns.
    pub curl_whitelist: Vec<Pattern>,
    /// Curl URL blacklist patterns.
    pub curl_blacklist: Vec<Pattern>,
    /// Enable skill system for loading SKILL.md files.
    pub enable_skills: bool,
    /// Directory to scan for user skills (default: ~/.claude/skills).
    pub skills_dir: Option<PathBuf>,
    /// Whether to also scan project-local .claude/skills/ directory.
    pub include_project_skills: bool,
    /// Project directory for scanning project-local skills (default: cwd).
    pub skills_project_dir: Option<PathBuf>,
}

impl Default for AgentConfig {
    /// Returns a default configuration.
    ///
    /// NOTE: All settings (api_key, base_url, model, work_dir) must be
    /// provided explicitly by the caller. The library does not read
    /// environment variables, to support multi-tenant scenarios with
    /// different credentials per instance.
    fn default() -> Self {
        Self {
            base_url: "https://api.anthropic.com".to_string(),
            api_key: String::new(),
            model: "claude-sonnet-4-6".to_string(),
            work_dir: PathBuf::from("."),
            max_tokens: 8192,
            max_turns: 100,
            max_duration_ms: 0,
            system_prompt: None,
            timeout_ms: 120_000,
            stream: true,
            output_format: OutputFormat::Text,
            enable_subagent: false,
            subagent_max_turns: 50,
            bash_whitelist: Vec::new(),
            bash_blacklist: Vec::new(),
            curl_whitelist: Vec::new(),
            curl_blacklist: Vec::new(),
            enable_skills: false,
            skills_dir: None,
            include_project_skills: false,
            skills_project_dir: None,
        }
    }
}
