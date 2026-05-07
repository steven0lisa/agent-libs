/** Skill system types. */

export interface SkillMetadata {
  name: string;
  description?: string;
  when_to_use?: string;
  allowed_tools?: string[];
  model?: string;
  context?: 'inline' | 'fork';
  version?: string;
}

export interface SkillInfo {
  metadata: SkillMetadata;
  content: string;
  filePath: string;
  dirPath: string;
}

export interface SkillResult {
  processedContent: string;
  isError: boolean;
}

export interface SkillLoaderOptions {
  skillsDir?: string;
  includeProjectSkills?: boolean;
  projectDir?: string;
}
