//! Security utilities for path validation and command filtering.

use std::path::{Path, PathBuf};

use crate::config::Pattern;

/// Resolve a path and ensure it stays within the working directory.
///
/// Returns the resolved path if valid.
/// Returns an error if the path escapes the working directory.
pub fn resolve_safe_path(file_path: impl AsRef<Path>, work_dir: impl AsRef<Path>) -> Result<PathBuf, String> {
    let work = work_dir.as_ref().canonicalize()
        .map_err(|e| format!("Failed to canonicalize work_dir: {}", e))?;
    let target = if file_path.as_ref().is_absolute() {
        file_path.as_ref().to_path_buf()
    } else {
        work.join(file_path.as_ref())
    };
    let resolved = target.canonicalize()
        .or_else(|_| {
            // If the path doesn't exist yet (e.g., for write operations),
            // resolve the parent and append the file name
            if let Some(parent) = target.parent() {
                let parent_resolved = parent.canonicalize()?;
                Ok(parent_resolved.join(target.file_name().unwrap_or_default()))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid path",
                ))
            }
        })
        .map_err(|e| format!("Failed to resolve path: {}", e))?;

    // Check if the resolved path is within the working directory
    if !resolved.starts_with(&work) {
        return Err(format!(
            "Path '{}' escapes working directory '{}'",
            file_path.as_ref().display(),
            work.display()
        ));
    }

    Ok(resolved)
}

/// Check if text passes the whitelist/blacklist policy.
///
/// Whitelist is checked first - if matched, the text is allowed.
/// Then blacklist is checked - if matched, the text is denied.
///
/// Returns `(allowed, reason)`.
pub fn check_security_policy(
    text: &str,
    whitelist: &[Pattern],
    blacklist: &[Pattern],
    default_allow: bool,
) -> (bool, String) {
    // Check whitelist first - if matched, allow
    for pattern in whitelist {
        if pattern.matches(text) {
            return (true, format!("Matched whitelist pattern: {}", pattern.pattern));
        }
    }

    // Check blacklist - if matched, deny
    for pattern in blacklist {
        if pattern.matches(text) {
            return (false, format!("Matched blacklist pattern: {}", pattern.pattern));
        }
    }

    // Default action
    if default_allow {
        (true, "Default allow".to_string())
    } else {
        (false, "Default deny".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_safe_path_relative() {
        let temp = TempDir::new().unwrap();
        let work = temp.path();
        let file = work.join("test.txt");
        fs::write(&file, "hello").unwrap();

        let resolved = resolve_safe_path("test.txt", work).unwrap();
        assert_eq!(resolved, file.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_safe_path_subdir() {
        let temp = TempDir::new().unwrap();
        let work = temp.path();
        let subdir = work.join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file = subdir.join("test.txt");
        fs::write(&file, "hello").unwrap();

        let resolved = resolve_safe_path("subdir/test.txt", work).unwrap();
        assert_eq!(resolved, file.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_safe_path_escapes_work_dir() {
        let temp = TempDir::new().unwrap();
        let work = temp.path();

        let result = resolve_safe_path("../outside.txt", work);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes working directory"));
    }

    #[test]
    fn test_resolve_safe_path_absolute_within() {
        let temp = TempDir::new().unwrap();
        let work = temp.path();
        let file = work.join("test.txt");
        fs::write(&file, "hello").unwrap();

        let resolved = resolve_safe_path(&file, work).unwrap();
        assert_eq!(resolved, file.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_safe_path_absolute_outside() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let work = temp1.path();
        let outside = temp2.path().join("outside.txt");
        fs::write(&outside, "hello").unwrap();

        let result = resolve_safe_path(&outside, work);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_security_policy_whitelist() {
        let whitelist = vec![Pattern::wildcard("ls *")];
        let blacklist = vec![Pattern::wildcard("rm *")];

        let (allowed, reason) = check_security_policy("ls -la", &whitelist, &blacklist, true);
        assert!(allowed);
        assert!(reason.contains("whitelist"));
    }

    #[test]
    fn test_check_security_policy_blacklist() {
        let whitelist: Vec<Pattern> = vec![];
        let blacklist = vec![Pattern::wildcard("rm *")];

        let (allowed, reason) = check_security_policy("rm -rf /", &whitelist, &blacklist, true);
        assert!(!allowed);
        assert!(reason.contains("blacklist"));
    }

    #[test]
    fn test_check_security_policy_default_allow() {
        let whitelist: Vec<Pattern> = vec![];
        let blacklist: Vec<Pattern> = vec![];

        let (allowed, _) = check_security_policy("echo hello", &whitelist, &blacklist, true);
        assert!(allowed);
    }

    #[test]
    fn test_check_security_policy_default_deny() {
        let whitelist: Vec<Pattern> = vec![];
        let blacklist: Vec<Pattern> = vec![];

        let (allowed, _) = check_security_policy("echo hello", &whitelist, &blacklist, false);
        assert!(!allowed);
    }

    #[test]
    fn test_whitelist_priority_over_blacklist() {
        // If a command matches both whitelist and blacklist, whitelist wins
        let whitelist = vec![Pattern::wildcard("rm *safe*")];
        let blacklist = vec![Pattern::wildcard("rm *")];

        let (allowed, reason) = check_security_policy("rm safe_file.txt", &whitelist, &blacklist, true);
        assert!(allowed);
        assert!(reason.contains("whitelist"));
    }

    #[test]
    fn test_wildcard_pattern_matching() {
        let pattern = Pattern::wildcard("ls *");
        assert!(pattern.matches("ls -la"));
        assert!(pattern.matches("ls /tmp"));
        assert!(!pattern.matches("rm file.txt"));

        let pattern2 = Pattern::wildcard("*dangerous*");
        assert!(pattern2.matches("something_dangerous_here"));
        assert!(!pattern2.matches("safe_command"));
    }
}
