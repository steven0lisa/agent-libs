/** Minimal YAML frontmatter parser for SKILL.md files. */

import { readFileSync } from 'fs';
import { SkillMetadata } from './types.js';

export function parseFrontmatter(raw: string): Partial<SkillMetadata> {
  const regex = /^---\n([\s\S]*?)\n---\n?/;
  const match = raw.match(regex);
  if (!match) return {};

  const yamlContent = match[1];
  const metadata: Record<string, unknown> = {};

  for (const line of yamlContent.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;

    const colonIndex = trimmed.indexOf(':');
    if (colonIndex === -1) continue;

    const key = trimmed.slice(0, colonIndex).trim() as keyof SkillMetadata;
    let value: string = trimmed.slice(colonIndex + 1).trim();

    if (!key || !value) continue;

    // Handle array values
    if (value.startsWith('- ')) {
      metadata[key] = value.slice(2).split(',').map(s => s.trim());
      continue;
    }

    // Remove surrounding quotes
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }

    // Try boolean
    if (value === 'true') { metadata[key] = true; continue; }
    if (value === 'false') { metadata[key] = false; continue; }

    // Try number
    const num = Number(value);
    if (!isNaN(num) && value !== '') { metadata[key] = num; continue; }

    metadata[key] = value;
  }

  // Normalize allowed_tools if it was parsed as an array
  if (metadata.allowed_tools && Array.isArray(metadata.allowed_tools)) {
    const arr = metadata.allowed_tools as string[];
    metadata.allowed_tools = arr.flatMap(s => s.split(',').map(x => x.trim()));
  }

  return metadata as Partial<SkillMetadata>;
}

export function parseSkillFile(filePath: string): { metadata: Partial<SkillMetadata>; content: string } {
  const raw = readFileSync(filePath, 'utf-8');
  const metadata = parseFrontmatter(raw);

  // Remove frontmatter, keep the rest
  const regex = /^---\n[\s\S]*?\n---\n?/;
  const content = raw.replace(regex, '').trim();

  return { metadata, content };
}
