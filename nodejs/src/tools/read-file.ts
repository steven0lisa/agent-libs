/** Read file tool. */

import { readFile } from 'fs/promises';
import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { resolveSafePath } from '../utils/security.js';

export class ReadFileTool implements ITool {
  readonly name = 'read_file';
  readonly description = 'Read file contents from the working directory.';
  readonly isReadOnly = true;

  readonly inputSchema = {
    type: 'object',
    properties: {
      file_path: { type: 'string', description: 'Path to the file' },
      offset: { type: 'integer' },
      limit: { type: 'integer' },
    },
    required: ['file_path'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const filePath = input.file_path as string;
    if (!filePath) return errorResult('file_path is required');

    try {
      const resolved = resolveSafePath(filePath, context.workDir);
      const content = await readFile(resolved, 'utf-8');
      return successResult(content);
    } catch (e) {
      return errorResult(`Failed to read file: ${e}`);
    }
  }
}
