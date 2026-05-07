/** Update file tool. */

import { readFile, writeFile } from 'fs/promises';
import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { resolveSafePath } from '../utils/security.js';

export class UpdateFileTool implements ITool {
  readonly name = 'update_file';
  readonly description = 'Update a file by replacing old_string with new_string.';
  readonly isReadOnly = false;

  readonly inputSchema = {
    type: 'object',
    properties: {
      file_path: { type: 'string' },
      old_string: { type: 'string' },
      new_string: { type: 'string' },
      replace_all: { type: 'boolean', default: false },
    },
    required: ['file_path', 'old_string', 'new_string'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const filePath = input.file_path as string;
    const oldStr = input.old_string as string;
    const newStr = input.new_string as string;
    const replaceAll = (input.replace_all as boolean) ?? false;

    try {
      const resolved = resolveSafePath(filePath, context.workDir);
      const content = await readFile(resolved, 'utf-8');
      const newContent = replaceAll
        ? content.split(oldStr).join(newStr)
        : content.replace(oldStr, newStr);

      if (newContent === content) {
        return errorResult('old_string not found in file');
      }

      await writeFile(resolved, newContent, 'utf-8');
      return successResult(`File updated: ${resolved}`);
    } catch (e) {
      return errorResult(`Failed to update file: ${e}`);
    }
  }
}
