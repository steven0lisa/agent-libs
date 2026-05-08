/** Grep tool – search for patterns in files using regular expressions. */

import { readdirSync, statSync, readFileSync } from 'fs';
import { join, relative } from 'path';
import { ITool, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { resolveSafePath } from '../utils/security.js';

const MAX_RESULTS = 100;

/** Recursively walk a directory, yielding relative file paths. */
function* walkDir(dir: string, base: string): Generator<string> {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* walkDir(fullPath, base);
    } else if (entry.isFile()) {
      yield relative(base, fullPath);
    }
  }
}

/** Convert a simple glob include pattern (e.g. "*.ts") to a RegExp. */
function globToRegex(glob: string): RegExp {
  const escaped = glob
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*/g, '.*')
    .replace(/\?/g, '.');
  return new RegExp('^' + escaped + '$');
}

export class GrepTool implements ITool {
  readonly name = 'grep';
  readonly description = 'Search for patterns in files using regular expressions.';
  readonly isReadOnly = true;

  readonly inputSchema = {
    type: 'object',
    properties: {
      pattern: { type: 'string', description: 'Regular expression pattern to search for' },
      path: { type: 'string', description: 'Directory or file path to search in (relative to workDir)' },
      include: { type: 'string', description: 'Glob pattern to filter files, e.g. "*.ts"' },
      ignoreCase: { type: 'boolean', description: 'Case-insensitive search (default: false)' },
    },
    required: ['pattern'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const pattern = input.pattern as string;
    if (!pattern) return errorResult('pattern is required');

    const searchPath = (input.path as string) || '.';
    const include = input.include as string | undefined;
    const ignoreCase = (input.ignoreCase as boolean) ?? false;

    try {
      const resolved = resolveSafePath(searchPath, context.workDir);
      const stat = statSync(resolved);
      const flags = ignoreCase ? 'i' : '';
      const regex = new RegExp(pattern, flags);
      const includeRegex = include ? globToRegex(include) : null;

      const results: string[] = [];

      if (stat.isFile()) {
        this.grepFile(resolved, context.workDir, regex, results);
      } else if (stat.isDirectory()) {
        for (const relPath of walkDir(resolved, resolved)) {
          if (results.length >= MAX_RESULTS) break;
          if (includeRegex && !includeRegex.test(relPath)) continue;
          const fullPath = join(resolved, relPath);
          this.grepFile(fullPath, resolved, regex, results);
        }
      }

      if (results.length === 0) {
        return successResult('No matches found.');
      }

      return successResult(results.join('\n'));
    } catch (e) {
      return errorResult(`Grep failed: ${e}`);
    }
  }

  private grepFile(
    filePath: string,
    baseDir: string,
    regex: RegExp,
    results: string[],
  ): void {
    let content: string;
    try {
      content = readFileSync(filePath, 'utf-8');
    } catch {
      return;
    }

    const relPath = relative(baseDir, filePath);
    const lines = content.split('\n');

    for (let i = 0; i < lines.length; i++) {
      if (results.length >= MAX_RESULTS) return;
      if (regex.test(lines[i])) {
        results.push(`${relPath}:${i + 1}: ${lines[i]}`);
        // Reset lastIndex for non-global regex
        regex.lastIndex = 0;
      }
    }
  }
}
