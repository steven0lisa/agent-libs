/** Subagent tool. */

import { Agent } from '../agent.js';
import { AgentConfig, ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { EventType } from '../types.js';

export class SubAgentTool implements ITool {
  readonly name = 'subagent';
  readonly description = 'Create a subagent to handle an independent task.';
  readonly isReadOnly = true;
  private parentConfig: AgentConfig;
  private parentHistory: Message[];
  private parentTools: Map<string, ITool>;

  constructor(parentConfig: AgentConfig, parentHistory: Message[], parentTools: Map<string, ITool>) {
    this.parentConfig = parentConfig;
    this.parentHistory = parentHistory;
    this.parentTools = parentTools;
  }

  readonly inputSchema = {
    type: 'object',
    properties: {
      task: { type: 'string', description: 'Description of the task for the subagent' },
    },
    required: ['task'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const task = input.task as string;
    if (!task) return errorResult('task is required');

    const subConfig: AgentConfig = {
      apiKey: this.parentConfig.apiKey,
      baseUrl: this.parentConfig.baseUrl,
      model: this.parentConfig.model,
      workDir: this.parentConfig.workDir,
      maxTokens: this.parentConfig.maxTokens,
      maxTurns: this.parentConfig.subagentMaxTurns ?? 50,
      systemPrompt: this.parentConfig.systemPrompt,
      timeoutMs: this.parentConfig.timeoutMs,
      stream: false,
    };

    const subagent = new Agent(subConfig);
    for (const [name, tool] of this.parentTools) {
      if (name !== 'subagent') subagent.registerTool(tool);
    }

    let finalContent = '';
    try {
      for await (const event of subagent.run(task)) {
        if (event.type === 'complete') {
          finalContent = (event.data.final_content as string) || '';
        } else if (event.type === 'error') {
          return errorResult(`Subagent error: ${event.data.message}`);
        }
      }
    } catch (e) {
      return errorResult(`Subagent failed: ${e}`);
    }

    return successResult(`Subagent completed. Result:\n${finalContent}`);
  }
}

import { Message } from '../types.js';
