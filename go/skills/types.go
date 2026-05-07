package skills

// Metadata represents skill frontmatter metadata from a SKILL.md file.
type Metadata struct {
	Name         string   `yaml:"name"`
	Description  string   `yaml:"description,omitempty"`
	WhenToUse    string   `yaml:"when_to_use,omitempty"`
	AllowedTools []string `yaml:"allowed_tools,omitempty"`
	Model        string   `yaml:"model,omitempty"`
	Context      string   `yaml:"context,omitempty"`
	Version      string   `yaml:"version,omitempty"`
}

// SkillInfo holds a discovered skill with its metadata, content, and file locations.
type SkillInfo struct {
	Metadata Metadata
	Content  string
	FilePath string
	DirPath  string
}

// LoaderOptions configures the skill loader's discovery behavior.
type LoaderOptions struct {
	SkillsDir            string
	IncludeProjectSkills bool
	ProjectDir           string
}
