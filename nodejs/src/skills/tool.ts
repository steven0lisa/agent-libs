/** SkillTool — ITool implementation that loads skills and returns their content. */

import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { SkillLoader } from './loader.js';
import { substituteVariables } from './substitution.js';
import type { SkillInfo } from './types.js';

export class SkillTool implements ITool {
  readonly name = 'skill';
  readonly description = 'Load a skill and get its instructions. Skills provide specialized capabilities for specific tasks.';
  readonly isReadOnly = true;

  private loader: SkillLoader;

  constructor(loader: SkillLoader) {
    this.loader = loader;
  }

  readonly inputSchema = {
    type: 'object',
    properties: {
      skill: {
        type: 'string',
        description: 'The name of the skill to load',
      },
      args: {
        type: 'string',
        description: 'Optional arguments passed to the skill via $ARGUMENTS variable substitution',
      },
    },
    required: ['skill'],
  };

  async call(input: Record<string, unknown>, _context: ToolContext) {
    const skillName = input.skill as string;
    const args = input.args as string | undefined;

    if (!skillName) {
      return errorResult('"skill" is required');
    }

    const skill = this.loader.findByName(skillName);
    if (!skill) {
      const available = this.loader.discoverAll().map(s => s.metadata.name).join(', ');
      return errorResult(
        `Skill not found: "${skillName}". Available skills: ${available || '(none)'}`
      );
    }

    const processedContent = substituteVariables(skill.content, {
      args,
      skillDir: skill.dirPath,
    });

    const result = buildSkillResult(skill, processedContent);
    return successResult(result);
  }
}

function buildSkillResult(skill: SkillInfo, content: string): string {
  const parts: string[] = [];
  parts.push(`## Skill: ${skill.metadata.name}`);

  if (skill.metadata.description) {
    parts.push(`Description: ${skill.metadata.description}`);
  }
  if (skill.metadata.when_to_use) {
    parts.push(`When to use: ${skill.metadata.when_to_use}`);
  }

  parts.push('', content);
  return parts.join('\n');
}
