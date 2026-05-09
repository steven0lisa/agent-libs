//! Skill loader -- discovers and caches SKILL.md files from disk.

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::skills::types::{SkillInfo, SkillLoaderOptions};
use crate::skills::yaml::parse_skill_file;

/// Discovers and caches skills from configured directories.
///
/// Uses interior mutability so that discovery methods can be called
/// through a shared `&Self` reference (required by Tool::call).
#[derive(Debug)]
pub struct SkillLoader {
    cache: Mutex<Option<Vec<SkillInfo>>>,
    options: SkillLoaderOptionsResolved,
}

#[derive(Debug, Clone)]
struct SkillLoaderOptionsResolved {
    skills_dir: PathBuf,
    include_project_skills: bool,
    project_dir: PathBuf,
}

/// Returns the default user skills directory: `~/.claude/skills`.
fn default_skills_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| "~".to_string());
    PathBuf::from(home).join(".claude").join("skills")
}

/// Returns the current working directory, or `.` as fallback.
fn default_project_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

impl SkillLoader {
    /// Create a new SkillLoader with optional options.
    pub fn new(options: Option<SkillLoaderOptions>) -> Self {
        let opts = options.unwrap_or(SkillLoaderOptions {
            skills_dir: None,
            include_project_skills: false,
            project_dir: None,
        });

        Self {
            cache: Mutex::new(None),
            options: SkillLoaderOptionsResolved {
                skills_dir: opts.skills_dir.unwrap_or_else(default_skills_dir),
                include_project_skills: opts.include_project_skills,
                project_dir: opts.project_dir.unwrap_or_else(default_project_dir),
            },
        }
    }

    /// Discover all skills from configured directories. Results are cached.
    pub fn discover_all(&self) -> Vec<SkillInfo> {
        let mut cache = self.cache.lock().expect("SkillLoader cache lock poisoned");
        if let Some(ref skills) = *cache {
            return skills.clone();
        }

        let mut skills: Vec<SkillInfo> = Vec::new();
        let mut scanned_dirs: HashSet<PathBuf> = HashSet::new();

        // User skills dir (higher priority -- scanned first so duplicates are skipped)
        Self::scan_directory(&self.options.skills_dir, &mut skills, &mut scanned_dirs);

        // Project skills dir (optional)
        if self.options.include_project_skills {
            let project_skills_dir = self.options.project_dir.join(".claude").join("skills");
            if project_skills_dir != self.options.skills_dir {
                Self::scan_directory(&project_skills_dir, &mut skills, &mut scanned_dirs);
            }
        }

        *cache = Some(skills.clone());
        skills
    }

    /// Find a skill by name. Returns `None` if not found.
    pub fn find_by_name(&self, name: &str) -> Option<SkillInfo> {
        let skills = self.discover_all();
        skills.into_iter().find(|s| s.metadata.name == name)
    }

    /// Clear the cached skill list.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().expect("SkillLoader cache lock poisoned");
        *cache = None;
    }

    /// Scan a single directory for skills (subdirectories containing SKILL.md).
    fn scan_directory(dir: &Path, result: &mut Vec<SkillInfo>, scanned_dirs: &mut HashSet<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if !file_type.is_dir() {
                continue;
            }

            let skill_dir = entry.path();
            let skill_file = skill_dir.join("SKILL.md");

            if !skill_file.exists() {
                continue;
            }

            if !scanned_dirs.insert(skill_dir.clone()) {
                continue; // Already scanned
            }

            match parse_skill_file(&skill_file) {
                Ok((metadata, content)) => {
                    let name = if metadata.name.is_empty() {
                        entry.file_name().to_string_lossy().to_string()
                    } else {
                        metadata.name.clone()
                    };

                    let skill = SkillInfo {
                        metadata: crate::skills::types::SkillMetadata {
                            name,
                            description: metadata.description,
                            when_to_use: metadata.when_to_use,
                            allowed_tools: metadata.allowed_tools,
                            model: metadata.model,
                            context: metadata.context,
                            version: metadata.version,
                            user_invocable: metadata.user_invocable,
                        },
                        content,
                        file_path: skill_file,
                        dir_path: skill_dir,
                    };

                    result.push(skill);
                }
                Err(_) => {
                    // Silently skip invalid skills
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_no_skills_dir() {
        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(PathBuf::from("/nonexistent/path")),
            include_project_skills: false,
            project_dir: None,
        }));
        let skills = loader.discover_all();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_discover_skills() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: A test\n---\n\nContent here",
        )
        .unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(skills_dir),
            include_project_skills: false,
            project_dir: None,
        }));

        let skills = loader.discover_all();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].metadata.name, "test-skill");
        assert_eq!(skills[0].metadata.description, "A test");
        assert_eq!(skills[0].content, "Content here");
    }

    #[test]
    fn test_discover_skills_caching() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("skill-a");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: skill-a\n---\n\nContent",
        )
        .unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(skills_dir.clone()),
            include_project_skills: false,
            project_dir: None,
        }));

        // First call should scan
        assert_eq!(loader.discover_all().len(), 1);

        // Add another skill
        let skill_dir2 = skills_dir.join("skill-b");
        fs::create_dir_all(&skill_dir2).unwrap();
        fs::write(
            skill_dir2.join("SKILL.md"),
            "---\nname: skill-b\n---\n\nContent",
        )
        .unwrap();

        // Second call should return cached result (still 1)
        assert_eq!(loader.discover_all().len(), 1);

        // After clearing cache, should be 2
        loader.clear_cache();
        assert_eq!(loader.discover_all().len(), 2);
    }

    #[test]
    fn test_find_by_name() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\n---\n\nContent",
        )
        .unwrap();

        let loader = SkillLoader::new(Some(SkillLoaderOptions {
            skills_dir: Some(skills_dir),
            include_project_skills: false,
            project_dir: None,
        }));

        let found = loader.find_by_name("my-skill");
        assert!(found.is_some());
        assert_eq!(found.unwrap().metadata.name, "my-skill");

        let not_found = loader.find_by_name("nonexistent");
        assert!(not_found.is_none());
    }
}
