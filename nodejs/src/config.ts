/** Configuration types for AgentLib. */

import { Event, Message, ToolResult } from './types.js';
import { join } from 'path';
import { homedir } from 'os';

export type OutputFormat = 'text' | 'json';

export class Pattern {
  constructor(
    public pattern: string,
    public type: 'wildcard' | 'regex' = 'wildcard'
  ) {}

  matches(text: string): boolean {
    if (this.type === 'wildcard') {
      const regex = new RegExp('^' + this.pattern.replace(/\*/g, '.*').replace(/\?/g, '.') + '$');
      return regex.test(text);
    }
    return new RegExp(this.pattern).test(text);
  }
}

export interface AgentConfig {
  apiKey: string;
  baseUrl?: string;
  model?: string;
  workDir?: string;
  maxTokens?: number;
  maxTurns?: number;
  maxDurationMs?: number;
  systemPrompt?: string;
  timeoutMs?: number;
  stream?: boolean;
  tools?: ITool[];
  outputFormat?: OutputFormat;
  callback?: (event: Event) => void;
  enableSubagent?: boolean;
  subagentMaxTurns?: number;
  bashWhitelist?: Pattern[];
  bashBlacklist?: Pattern[];
  curlWhitelist?: Pattern[];
  curlBlacklist?: Pattern[];

  /** Enable skill system. When true, skills from ~/.claude/skills/ are loaded. */
  enableSkills?: boolean;
  /** Custom skills directory (default: ~/.claude/skills). */
  skillsDir?: string;
  /** Also scan project-level skills (./.claude/skills). */
  includeProjectSkills?: boolean;
  /** Project root for project-level skills discovery. */
  skillsProjectDir?: string;
}

export const DEFAULT_CONFIG = {
  baseUrl: 'https://api.anthropic.com',
  model: 'claude-sonnet-4-6',
  maxTokens: 8192,
  maxTurns: 100,
  maxDurationMs: 0,
  timeoutMs: 120_000,
  stream: true,
  outputFormat: 'text' as OutputFormat,
  enableSubagent: false,
  subagentMaxTurns: 50,
  enableSkills: false,
};

export function resolveConfig(config: AgentConfig): Required<AgentConfig> {
  // All configuration must be provided explicitly by the caller.
  // The library does not read environment variables, to support
  // multi-tenant scenarios with different credentials per instance.
  return {
    apiKey: config.apiKey,
    baseUrl: config.baseUrl || DEFAULT_CONFIG.baseUrl,
    model: config.model || DEFAULT_CONFIG.model,
    workDir: config.workDir || process.cwd(),
    maxTokens: config.maxTokens ?? DEFAULT_CONFIG.maxTokens,
    maxTurns: config.maxTurns ?? DEFAULT_CONFIG.maxTurns,
    maxDurationMs: config.maxDurationMs ?? DEFAULT_CONFIG.maxDurationMs,
    systemPrompt: config.systemPrompt || '',
    timeoutMs: config.timeoutMs ?? DEFAULT_CONFIG.timeoutMs,
    stream: config.stream ?? DEFAULT_CONFIG.stream,
    tools: config.tools || [],
    outputFormat: config.outputFormat || DEFAULT_CONFIG.outputFormat,
    callback: config.callback ?? (() => {}),
    enableSubagent: config.enableSubagent ?? DEFAULT_CONFIG.enableSubagent,
    subagentMaxTurns: config.subagentMaxTurns ?? DEFAULT_CONFIG.subagentMaxTurns,
    bashWhitelist: config.bashWhitelist || [],
    bashBlacklist: config.bashBlacklist || [],
    curlWhitelist: config.curlWhitelist || [],
    curlBlacklist: config.curlBlacklist || [],
    enableSkills: config.enableSkills ?? DEFAULT_CONFIG.enableSkills,
    skillsDir: config.skillsDir || join(homedir(), '.claude', 'skills'),
    includeProjectSkills: config.includeProjectSkills ?? false,
    skillsProjectDir: config.skillsProjectDir || process.cwd(),
  };
}

export interface ITool {
  readonly name: string;
  readonly description: string;
  readonly inputSchema: Record<string, unknown>;
  readonly isReadOnly?: boolean;
  call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult>;
}

export interface ToolContext {
  workDir: string;
  messageHistory: Message[];
}
