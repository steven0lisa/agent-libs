/** Write file tool. */

import { mkdir, writeFile } from 'fs/promises';
import { dirname } from 'path';
import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { resolveSafePath } from '../utils/security.js';

export class WriteFileTool implements ITool {
  readonly name = 'write_file';
  readonly description = 'Write content to a file.';
  readonly isReadOnly = false;

  readonly inputSchema = {
    type: 'object',
    properties: {
      file_path: { type: 'string' },
      content: { type: 'string' },
    },
    required: ['file_path', 'content'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const filePath = input.file_path as string;
    const content = input.content as string;

    try {
      const resolved = resolveSafePath(filePath, context.workDir);
      await mkdir(dirname(resolved), { recursive: true });
      await writeFile(resolved, content, 'utf-8');
      return successResult(`File written: ${resolved}`);
    } catch (e) {
      return errorResult(`Failed to write file: ${e}`);
    }
  }
}
