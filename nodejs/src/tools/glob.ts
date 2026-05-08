/** Glob tool – find files matching a glob pattern. */

import { readdirSync, statSync } from 'fs';
import { join, relative } from 'path';
import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { resolveSafePath } from '../utils/security.js';

/** Recursively walk a directory, yielding relative file paths. */
function* walkDir(dir: string): Generator<string> {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* walkDir(fullPath);
    } else if (entry.isFile()) {
      yield fullPath;
    }
  }
}

/**
 * Convert a minimatch-style glob pattern to a RegExp.
 * Supports:
 *   **  – match any path segments
 *   *   – match any characters except /
 *   ?   – match single character except /
 */
function globPatternToRegex(pattern: string): RegExp {
  const parts = pattern.split('**');
  const regexParts = parts.map((part) => {
    return part
      .replace(/[.+^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '[^/]*')
      .replace(/\?/g, '[^/]');
  });
  const regexStr = regexParts.join('.*');
  return new RegExp('^' + regexStr + '$');
}

export class GlobTool implements ITool {
  readonly name = 'glob';
  readonly description = 'Find files matching a glob pattern.';
  readonly isReadOnly = true;

  readonly inputSchema = {
    type: 'object',
    properties: {
      pattern: { type: 'string', description: 'Glob pattern to match files (e.g. "**/*.ts", "src/*.js")' },
      path: { type: 'string', description: 'Directory to search in (relative to workDir)' },
    },
    required: ['pattern'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const pattern = input.pattern as string;
    if (!pattern) return errorResult('pattern is required');

    const searchPath = (input.path as string) || '.';

    try {
      const resolved = resolveSafePath(searchPath, context.workDir);
      const stat = statSync(resolved);

      if (!stat.isDirectory()) {
        return errorResult(`'${searchPath}' is not a directory`);
      }

      const regex = globPatternToRegex(pattern);
      const matches: string[] = [];

      for (const filePath of walkDir(resolved)) {
        const relPath = relative(resolved, filePath);
        if (regex.test(relPath)) {
          matches.push(relPath);
        }
      }

      matches.sort();

      if (matches.length === 0) {
        return successResult('No files matched the pattern.');
      }

      return successResult(matches.join('\n'));
    } catch (e) {
      return errorResult(`Glob failed: ${e}`);
    }
  }
}
