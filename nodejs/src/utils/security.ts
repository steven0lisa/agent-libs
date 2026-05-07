/** Security utilities. */

import { resolve } from 'path';

export interface PatternLike {
  pattern: string;
  type: 'wildcard' | 'regex';
  matches(text: string): boolean;
}

export function resolveSafePath(filePath: string, workDir: string): string {
  const resolved = resolve(workDir, filePath);
  const workResolved = resolve(workDir);
  if (!resolved.startsWith(workResolved)) {
    throw new Error(`Path '${filePath}' escapes working directory '${workDir}'`);
  }
  return resolved;
}

export function checkSecurityPolicy(
  text: string,
  whitelist: PatternLike[],
  blacklist: PatternLike[],
  defaultAllow = true
): { allowed: boolean; reason: string } {
  for (const pattern of whitelist) {
    if (pattern.matches(text)) {
      return { allowed: true, reason: `Matched whitelist: ${pattern.pattern}` };
    }
  }
  for (const pattern of blacklist) {
    if (pattern.matches(text)) {
      return { allowed: false, reason: `Matched blacklist: ${pattern.pattern}` };
    }
  }
  return { allowed: defaultAllow, reason: defaultAllow ? 'Default allow' : 'Default deny' };
}
