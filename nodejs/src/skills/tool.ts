/** SkillTool — ITool implementation that loads skills and injects their content as user messages. */

import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult, successWithMessages, Message } from '../types.js';
import { SkillLoader } from './loader.js';
import { substituteVariables } from './substitution.js';
import type { SkillInfo } from './types.js';

export class SkillTool implements ITool {
  readonly name = 'skill';
  readonly isReadOnly = true;

  private loader: SkillLoader;

  constructor(loader: SkillLoader) {
    this.loader = loader;
  }

  /** Dynamically list user-invocable skills in the description. */
  get description(): string {
    let desc = 'Load a skill and get its instructions. Skills provide specialized capabilities for specific tasks.';
    const skills = this.loader.discoverAll();
    const invocable = skills.filter(s => s.metadata.user_invocable !== false);
    if (invocable.length > 0) {
      desc += '\n\nAvailable skills:\n';
      for (const skill of invocable) {
        const line = skill.metadata.description
          ? `- ${skill.metadata.name}: ${skill.metadata.description}`
          : `- ${skill.metadata.name}`;
        desc += line + '\n';
      }
    }
    return desc;
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

    // Message injection mode: inject skill content as a user message
    const fullContent = buildSkillInjectionContent(skill, processedContent);
    const brief = `Skill loaded: ${skill.metadata.name}`;
    const injectionMsg: Message = {
      role: 'user',
      content: [{ type: 'text', text: fullContent }],
    };
    return successWithMessages(brief, [injectionMsg]);
  }
}

/** Build the formatted injection content from a skill. */
function buildSkillInjectionContent(skill: SkillInfo, content: string): string {
  const parts: string[] = [];
  parts.push(`## Skill: ${skill.metadata.name}`);

  if (skill.metadata.description) {
    parts.push(`Description: ${skill.metadata.description}`);
  }
  if (skill.metadata.when_to_use) {
    parts.push(`When to use: ${skill.metadata.when_to_use}`);
  }

  parts.push('');
  parts.push(content);

  // Add allowed_tools soft constraint hint
  if (skill.metadata.allowed_tools && skill.metadata.allowed_tools.length > 0) {
    parts.push('');
    parts.push(
      `Note: When following this skill's instructions, only use these tools: ${skill.metadata.allowed_tools.join(', ')}`
    );
  }

  // Add fork mode soft marker
  if (skill.metadata.context === 'fork') {
    parts.push('');
    parts.push('This skill should be executed in a fork context.');
  }

  return parts.join('\n');
}
