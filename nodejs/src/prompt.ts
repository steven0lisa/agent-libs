/** System prompt builder. */

import { ITool } from './config.js';
import type { SkillInfo } from './skills/types.js';

const DEFAULT_SYSTEM_PROMPT = `You are a helpful software engineering assistant. You have access to tools that let you interact with the file system and execute commands.

## Tool Use

You will be provided with a set of tools. When you need to perform an action, use the appropriate tool by outputting a tool_use block.

Tool use format:
- Each tool has a name and input parameters defined by a JSON schema
- To use a tool, output a content block with type "tool_use"
- The tool_use block must include: name (tool name), id (unique identifier), input (parameters as JSON object)

After using a tool, the user message will contain a tool_result block with the execution result.
- If the result indicates success, continue with your analysis or next steps
- If the result indicates an error, analyze the error and decide whether to retry, use a different approach, or ask the user for clarification

## File Operations

When working with files:
- Always read a file before editing it
- When editing files, use update_file tool with old_string/new_string for precise changes
- Use write_file to create new files
- Use read_file to inspect file contents

## Command Execution

When executing commands via bash:
- Prefer read-only commands for exploration (ls, grep, find, cat, etc.)
- Be careful with destructive commands (rm, dd, etc.)
- Always describe what the command does before executing it
`;

const SUBAGENT_PROMPT = `

## Subagent Capability

You can create subagents to handle independent tasks in parallel. A subagent is a separate agent instance that shares your context but operates independently.

When to use subagents:
- When you need to explore multiple approaches simultaneously
- When a task can be cleanly decomposed into independent sub-tasks
- When you want to parallelize read-only exploration

How to create a subagent:
- Use the "subagent" tool with a "task" describing what the subagent should do
- The subagent will execute with its own tool budget
- The subagent inherits your current context (files read, tool state, etc.)
- Results are returned as tool_result when the subagent completes
`;

export function buildSystemPrompt(
  tools: Map<string, ITool>,
  customPrompt?: string,
  enableSubagent?: boolean,
  subagentMaxTurns?: number,
  skills?: SkillInfo[]
): string {
  const toolDescriptions: string[] = [];
  for (const tool of tools.values()) {
    toolDescriptions.push(
      `### ${tool.name}\nDescription: ${tool.description}\nInput Schema: ${JSON.stringify(tool.inputSchema)}`
    );
  }

  let prompt = DEFAULT_SYSTEM_PROMPT + '\n\n## Available Tools\n\n' + toolDescriptions.join('\n\n');

  if (enableSubagent) {
    prompt += SUBAGENT_PROMPT.replace('tool budget', `tool budget (max ${subagentMaxTurns} turns)`);
  }

  if (skills && skills.length > 0) {
    prompt += '\n\n## Available Skills\n\n';
    prompt += skills.map(s =>
      `### ${s.metadata.name}\nDescription: ${s.metadata.description || ''}${s.metadata.when_to_use ? `\nWhen to use: ${s.metadata.when_to_use}` : ''}`
    ).join('\n\n');
    prompt += '\n\nTo use a skill, call the `skill` tool with the skill name and optional arguments.';
  }

  if (customPrompt) {
    prompt += `\n\n## Custom Instructions\n\n${customPrompt}`;
  }

  return prompt;
}
