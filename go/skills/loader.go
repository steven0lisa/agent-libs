package skills

import (
	"os"
	"path/filepath"
)

// Loader discovers and caches SKILL.md files from configured directories.
type Loader struct {
	options LoaderOptions
	cache   []SkillInfo
}

// NewLoader creates a new SkillLoader with the given options.
// If SkillsDir is empty, it defaults to $HOME/.claude/skills.
// If ProjectDir is empty, it defaults to the current working directory.
func NewLoader(options LoaderOptions) *Loader {
	if options.SkillsDir == "" {
		home, err := os.UserHomeDir()
		if err == nil {
			options.SkillsDir = filepath.Join(home, ".claude", "skills")
		}
	}
	if options.ProjectDir == "" {
		wd, err := os.Getwd()
		if err == nil {
			options.ProjectDir = wd
		}
	}
	return &Loader{
		options: options,
	}
}

// DiscoverAll scans all configured skill directories and returns the list
// of discovered skills. Results are cached; call ClearCache to force a re-scan.
func (l *Loader) DiscoverAll() []SkillInfo {
	if l.cache != nil {
		return l.cache
	}

	var skills []SkillInfo
	scannedDirs := make(map[string]bool)

	// User skills directory (higher priority — scanned first).
	l.scanDirectory(l.options.SkillsDir, &skills, scannedDirs)

	// Project skills directory (optional, lower priority).
	if l.options.IncludeProjectSkills && l.options.ProjectDir != "" {
		projectSkillsDir := filepath.Join(l.options.ProjectDir, ".claude", "skills")
		if projectSkillsDir != l.options.SkillsDir {
			l.scanDirectory(projectSkillsDir, &skills, scannedDirs)
		}
	}

	l.cache = skills
	return skills
}

// FindByName finds a skill by name. Returns nil if not found.
func (l *Loader) FindByName(name string) *SkillInfo {
	skills := l.DiscoverAll()
	for _, s := range skills {
		if s.Metadata.Name == name {
			return &s
		}
	}
	return nil
}

// ClearCache clears the cached skill list, forcing a re-scan on the next call.
func (l *Loader) ClearCache() {
	l.cache = nil
}

// scanDirectory scans a single directory for skill directories.
// Each skill is a subdirectory containing a SKILL.md file.
func (l *Loader) scanDirectory(dir string, result *[]SkillInfo, scannedDirs map[string]bool) {
	if dir == "" {
		return
	}

	entries, err := os.ReadDir(dir)
	if err != nil {
		return // Silently skip invalid directories.
	}

	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		skillDir := filepath.Join(dir, entry.Name())
		skillFile := filepath.Join(skillDir, "SKILL.md")

		if scannedDirs[skillDir] {
			continue
		}
		scannedDirs[skillDir] = true

		info, err := os.Stat(skillFile)
		if err != nil || info.IsDir() {
			continue
		}

		meta, content, err := ParseSkillFile(skillFile)
		if err != nil {
			continue // Silently skip invalid skills.
		}

		// Use directory name as fallback if metadata name is empty.
		name := meta.Name
		if name == "" {
			name = entry.Name()
		}
		if name == "" {
			continue
		}

		meta.Name = name
		*result = append(*result, SkillInfo{
			Metadata: meta,
			Content:  content,
			FilePath: skillFile,
			DirPath:  skillDir,
		})
	}
}
