import { describe, it, before, after } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';

// Import skill system modules
import { parseFrontmatter, parseSkillFile } from '../../../nodejs/src/skills/yaml.js';
import { substituteVariables } from '../../../nodejs/src/skills/substitution.js';
import { SkillLoader } from '../../../nodejs/src/skills/loader.js';
import { SkillTool } from '../../../nodejs/src/skills/tool.js';
import { buildSystemPrompt } from '../../../nodejs/src/prompt.js';
import type { SkillInfo } from '../../../nodejs/src/skills/types.js';

// -------------------------------------------------------------------------
// YAML Frontmatter Parser
// -------------------------------------------------------------------------
describe('parseFrontmatter', () => {
  it('should parse name and description', () => {
    const raw = `---
name: my-skill
description: A test skill
---`;
    const result = parseFrontmatter(raw);
    assert.equal(result.name, 'my-skill');
    assert.equal(result.description, 'A test skill');
  });

  it('should handle boolean values', () => {
    const raw = `---
name: test
enabled: true
disabled: false
---`;
    const result = parseFrontmatter(raw);
    assert.equal(result.name, 'test');
  });

  it('should handle quoted values', () => {
    const raw = `---
name: "quoted-name"
description: 'single quoted'
---`;
    const result = parseFrontmatter(raw);
    assert.equal(result.name, 'quoted-name');
    assert.equal(result.description, 'single quoted');
  });

  it('should handle when_to_use field', () => {
    const raw = `---
name: test
when_to_use: Use this when testing
---`;
    const result = parseFrontmatter(raw);
    assert.equal(result.when_to_use, 'Use this when testing');
  });

  it('should return empty metadata for no frontmatter', () => {
    const result = parseFrontmatter('Just some content');
    assert.deepEqual(result, {});
  });

  it('should handle empty input', () => {
    const result = parseFrontmatter('');
    assert.deepEqual(result, {});
  });
});

// -------------------------------------------------------------------------
// Variable Substitution
// -------------------------------------------------------------------------
describe('substituteVariables', () => {
  it('should replace $ARGUMENTS', () => {
    const result = substituteVariables('echo $ARGUMENTS', { args: 'hello world' });
    assert.equal(result, 'echo hello world');
  });

  it('should replace ${CLAUDE_SKILL_DIR}', () => {
    const result = substituteVariables('cd ${CLAUDE_SKILL_DIR}', { skillDir: '/skills/test' });
    assert.equal(result, 'cd /skills/test');
  });

  it('should replace ${ENV:...}', () => {
    process.env.TEST_VAR = 'test_value';
    const result = substituteVariables('echo ${ENV:TEST_VAR}', {});
    assert.equal(result, 'echo test_value');
  });

  it('should handle empty env vars gracefully', () => {
    const result = substituteVariables('echo ${ENV:NONEXISTENT}', {});
    assert.equal(result, 'echo ');
  });

  it('should handle multiple substitutions', () => {
    const result = substituteVariables(
      '$ARGUMENTS at ${CLAUDE_SKILL_DIR} home=${ENV:HOME}',
      { args: 'foo', skillDir: '/skills/dir' }
    );
    assert.match(result, /^foo at \/skills\/dir home=/);
  });

  it('should handle no matches (pass through)', () => {
    const result = substituteVariables('plain text no variables', {});
    assert.equal(result, 'plain text no variables');
  });
});

// -------------------------------------------------------------------------
// SkillLoader
// -------------------------------------------------------------------------
describe('SkillLoader', () => {
  const testDir = join(tmpdir(), `skill-test-${Date.now()}`);
  const skillName = 'test-skill';
  const skillPath = join(testDir, skillName, 'SKILL.md');

  before(async () => {
    await mkdir(join(testDir, skillName), { recursive: true });
    await writeFile(skillPath, `---
name: ${skillName}
description: Test skill for loader tests
---

# Test Content`);
  });

  after(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  it('should discover skills from directory', () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const skills = loader.discoverAll();
    assert.equal(skills.length, 1);
    assert.equal(skills[0].metadata.name, skillName);
    assert.equal(skills[0].metadata.description, 'Test skill for loader tests');
  });

  it('should find skill by name', () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const skill = loader.findByName(skillName);
    assert.ok(skill);
    assert.equal(skill.metadata.name, skillName);
  });

  it('should return undefined for unknown skill', () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const skill = loader.findByName('nonexistent');
    assert.equal(skill, undefined);
  });

  it('should cache results', () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const skills1 = loader.discoverAll();
    const skills2 = loader.discoverAll();
    assert.equal(skills1, skills2); // same reference
  });

  it('should clear cache', () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    loader.discoverAll();
    loader.clearCache();
    const skills = loader.discoverAll();
    assert.equal(skills.length, 1);
  });

  it('should handle empty directory', () => {
    const emptyDir = join(tmpdir(), `empty-skills-${Date.now()}`);
    const loader = new SkillLoader({ skillsDir: emptyDir });
    const skills = loader.discoverAll();
    assert.equal(skills.length, 0);
  });
});

// -------------------------------------------------------------------------
// SkillTool
// -------------------------------------------------------------------------
describe('SkillTool', () => {
  const testDir = join(tmpdir(), `skill-tool-test-${Date.now()}`);
  const skillName = 'tool-test-skill';

  before(async () => {
    await mkdir(join(testDir, skillName), { recursive: true });
    await writeFile(join(testDir, skillName, 'SKILL.md'), `---
name: ${skillName}
description: Tool test skill
when_to_use: For testing skill tool
---

# Tool Test

echo $ARGUMENTS
dir: ` + '${CLAUDE_SKILL_DIR}');
  });

  after(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  it('should load and process a skill', async () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const tool = new SkillTool(loader);

    const result = await tool.call({ skill: skillName, args: 'hello' }, {
      workDir: '/tmp',
      messageHistory: [],
    });

    assert.equal(result.isError, false);
    assert.match(result.content, /## Skill: tool-test-skill/);
    assert.match(result.content, /Description: Tool test skill/);
    assert.match(result.content, /echo hello/);
    assert.match(result.content, /dir: .*tool-test-skill/);
  });

  it('should return error for unknown skill', async () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const tool = new SkillTool(loader);

    const result = await tool.call({ skill: 'nonexistent' }, {
      workDir: '/tmp',
      messageHistory: [],
    });

    assert.equal(result.isError, true);
    assert.match(result.content, /Skill not found/);
  });

  it('should return error when skill name is missing', async () => {
    const loader = new SkillLoader({ skillsDir: testDir });
    const tool = new SkillTool(loader);

    const result = await tool.call({}, {
      workDir: '/tmp',
      messageHistory: [],
    });

    assert.equal(result.isError, true);
  });
});

// -------------------------------------------------------------------------
// System Prompt Integration
// -------------------------------------------------------------------------
describe('buildSystemPrompt with skills', () => {
  it('should inject skill listing when skills provided', () => {
    const tools = new Map();
    const skills: SkillInfo[] = [{
      metadata: { name: 'test-skill', description: 'A test' },
      content: '# test',
      filePath: '/tmp/SKILL.md',
      dirPath: '/tmp',
    }];

    const prompt = buildSystemPrompt(tools, undefined, false, 50, skills);
    assert.match(prompt, /## Available Skills/);
    assert.match(prompt, /test-skill/);
    assert.match(prompt, /A test/);
    assert.match(prompt, /skill` tool/);
  });

  it('should not inject skill section when no skills', () => {
    const tools = new Map();
    const prompt = buildSystemPrompt(tools);
    assert.doesNotMatch(prompt, /## Available Skills/);
  });

  it('should not inject skill section when empty array', () => {
    const tools = new Map();
    const prompt = buildSystemPrompt(tools, undefined, false, 50, []);
    assert.doesNotMatch(prompt, /## Available Skills/);
  });
});
