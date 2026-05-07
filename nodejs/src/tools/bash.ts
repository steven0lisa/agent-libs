/** Bash tool with security policy. */

import { spawn } from 'child_process';
import { ITool, Pattern, ToolContext } from '../config.js';
import { ToolResult, successResult, errorResult } from '../types.js';
import { checkSecurityPolicy } from '../utils/security.js';

export class BashTool implements ITool {
  readonly name = 'bash';
  readonly description = 'Execute a shell command in the working directory.';
  private whitelist: Pattern[];
  private blacklist: Pattern[];

  constructor(whitelist?: Pattern[], blacklist?: Pattern[]) {
    this.whitelist = whitelist || [];
    this.blacklist = blacklist || [];
  }

  readonly inputSchema = {
    type: 'object',
    properties: {
      command: { type: 'string', description: 'The shell command' },
      description: { type: 'string' },
      timeout: { type: 'integer', default: 120000 },
    },
    required: ['command'],
  };

  async call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult> {
    const command = input.command as string;
    const timeoutMs = (input.timeout as number) ?? 120_000;

    const { allowed, reason } = checkSecurityPolicy(
      command,
      this.whitelist,
      this.blacklist,
      true
    );
    if (!allowed) {
      return errorResult(`Command blocked by security policy: ${reason}`);
    }

    return new Promise<ToolResult>((resolve) => {
      const child = spawn('sh', ['-c', command], {
        cwd: context.workDir,
      });

      let stdout = '';
      let stderr = '';
      let timedOut = false;

      const timer = setTimeout(() => {
        timedOut = true;
        child.kill('SIGKILL');
      }, timeoutMs);

      child.stdout?.on('data', (data: Buffer) => { stdout += data.toString(); });
      child.stderr?.on('data', (data: Buffer) => { stderr += data.toString(); });

      child.on('close', (code: number | null) => {
        clearTimeout(timer);
        if (timedOut) {
          resolve(errorResult(`Command timed out after ${timeoutMs}ms`));
          return;
        }
        const output = stderr ? `${stdout}\n[stderr]\n${stderr}` : stdout;
        resolve(code === 0 ? successResult(output) : errorResult(output));
      });

      child.on('error', (err: Error) => {
        clearTimeout(timer);
        resolve(errorResult(`Failed to execute: ${err.message}`));
      });
    });
  }
}
