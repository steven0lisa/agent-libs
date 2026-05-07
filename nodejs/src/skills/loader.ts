/** Skill loader — discovers and caches SKILL.md files from disk. */

import { existsSync, readdirSync } from 'fs';
import { join, resolve } from 'path';
import { homedir } from 'os';
import { SkillInfo, SkillLoaderOptions } from './types.js';
import { parseSkillFile } from './yaml.js';

export class SkillLoader {
  private cache: SkillInfo[] | null = null;
  private options: Required<SkillLoaderOptions>;

  constructor(options?: SkillLoaderOptions) {
    const defaultDir = join(homedir(), '.claude', 'skills');
    this.options = {
      skillsDir: options?.skillsDir || defaultDir,
      includeProjectSkills: options?.includeProjectSkills ?? false,
      projectDir: options?.projectDir || process.cwd(),
    };
  }

  /** Discover all skills from configured directories. Results are cached. */
  discoverAll(): SkillInfo[] {
    if (this.cache !== null) return this.cache;

    const skills: SkillInfo[] = [];
    const scannedDirs = new Set<string>();

    // User skills dir (higher priority — scanned first so duplicates are skipped)
    this.scanDirectory(this.options.skillsDir, skills, scannedDirs);

    // Project skills dir (optional)
    if (this.options.includeProjectSkills) {
      const projectSkillsDir = resolve(this.options.projectDir, '.claude', 'skills');
      if (projectSkillsDir !== this.options.skillsDir) {
        this.scanDirectory(projectSkillsDir, skills, scannedDirs);
      }
    }

    this.cache = skills;
    return skills;
  }

  /** Find a skill by name. Returns undefined if not found. */
  findByName(name: string): SkillInfo | undefined {
    const skills = this.discoverAll();
    return skills.find(s => s.metadata.name === name);
  }

  /** Clear the cached skill list. */
  clearCache(): void {
    this.cache = null;
  }

  private scanDirectory(dir: string, result: SkillInfo[], scannedDirs: Set<string>): void {
    if (!existsSync(dir)) return;

    const entries = readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;

      const skillDir = resolve(dir, entry.name);
      const skillFile = join(skillDir, 'SKILL.md');

      if (!existsSync(skillFile)) continue;
      if (scannedDirs.has(skillDir)) continue;
      scannedDirs.add(skillDir);

      try {
        const { metadata, content } = parseSkillFile(skillFile);
        const name = (metadata.name as string) || entry.name;
        if (!name) continue;

        result.push({
          metadata: { ...metadata, name } as SkillInfo['metadata'],
          content,
          filePath: skillFile,
          dirPath: skillDir,
        });
      } catch {
        // Silently skip invalid skills
      }
    }
  }
}
