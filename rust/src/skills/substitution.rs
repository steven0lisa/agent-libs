//! Variable substitution for skill content.

use std::env;

/// Substitute variables in skill content:
///
/// - `$ARGUMENTS`              -> user-provided arguments
/// - `${CLAUDE_SKILL_DIR}`     -> skill directory path
/// - `${ENV:VAR_NAME}`         -> environment variable value
///
/// Unrecognized `${...}` patterns are left as-is.
pub fn substitute_variables(
    content: &str,
    args: Option<&str>,
    skill_dir: Option<&str>,
) -> String {
    let mut result = content.to_string();

    // Substitute $ARGUMENTS (without braces)
    if let Some(args_val) = args {
        // Replace all occurrences of $ARGUMENTS that aren't part of ${...}
        result = replace_dollar_arguments(&result, args_val);
    }

    // Substitute ${CLAUDE_SKILL_DIR}
    if let Some(dir) = skill_dir {
        result = result.replace("${CLAUDE_SKILL_DIR}", dir);
    }

    // Substitute ${ENV:VAR_NAME}
    result = replace_env_vars(&result);

    result
}

/// Replace `$ARGUMENTS` (when not preceded by `{`) with the given value.
fn replace_dollar_arguments(text: &str, args: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let bytes = text.as_bytes();
    let target = "$ARGUMENTS";
    let target_len = target.len();

    while i < bytes.len() {
        if bytes[i] == b'$' && bytes.len() - i >= target_len
            && &text[i..i + target_len] == target
        {
            // Check it's not part of ${...}
            if i == 0 || bytes[i - 1] != b'{' {
                out.push_str(args);
                i += target_len;
                continue;
            }
        }
        out.push(text[i..].chars().next().unwrap_or(' '));
        i += text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }

    out
}

/// Replace all `${ENV:VAR_NAME}` patterns with their environment variable values.
fn replace_env_vars(text: &str) -> String {
    // Use a simple scanning approach
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while !rest.is_empty() {
        if let Some(start) = rest.find("${ENV:") {
            // Push everything before the pattern
            result.push_str(&rest[..start]);

            let after_start = &rest[start + 6..]; // skip "${ENV:"
            if let Some(end) = after_start.find('}') {
                let var_name = &after_start[..end];
                let value = env::var(var_name).unwrap_or_default();
                result.push_str(&value);
                // Continue after the closing }
                rest = &after_start[end + 1..];
            } else {
                // No closing brace, push remaining
                result.push_str(&rest[start..]);
                break;
            }
        } else {
            result.push_str(rest);
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_arguments() {
        let content = "Use these args: $ARGUMENTS";
        let result = substitute_variables(content, Some("foo bar"), None);
        assert_eq!(result, "Use these args: foo bar");
    }

    #[test]
    fn test_substitute_skill_dir() {
        let content = "From ${CLAUDE_SKILL_DIR}";
        let result = substitute_variables(content, None, Some("/tmp/skills/my-skill"));
        assert_eq!(result, "From /tmp/skills/my-skill");
    }

    #[test]
    fn test_substitute_env_var() {
        let content = "Home is ${ENV:HOME}";
        let result = substitute_variables(content, None, None);
        assert!(!result.contains("${ENV:HOME}"));
        assert!(!result.is_empty());
    }

    #[test]
    fn test_substitute_all() {
        let content = "Args: $ARGUMENTS, Dir: ${CLAUDE_SKILL_DIR}";
        let result = substitute_variables(content, Some("hello"), Some("/skills/mine"));
        assert_eq!(result, "Args: hello, Dir: /skills/mine");
    }

    #[test]
    fn test_no_substitutions() {
        let content = "Plain text without variables";
        let result = substitute_variables(content, None, None);
        assert_eq!(result, "Plain text without variables");
    }

    #[test]
    fn test_unknown_env_var() {
        let content = "Unknown: ${ENV:DOES_NOT_EXIST_XYZ}";
        let result = substitute_variables(content, None, None);
        assert_eq!(result, "Unknown: ");
    }

    #[test]
    fn test_env_var_in_braces_not_confused() {
        let content = "${ARGUMENTS}";
        let result = substitute_variables(content, Some("val"), None);
        // ${ARGUMENTS} should not be replaced since it's ${...} not plain $ARGUMENTS
        assert_eq!(result, "${ARGUMENTS}");
    }
}
