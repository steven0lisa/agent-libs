import { readFile } from 'fs/promises';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

import { Agent, resolveConfig } from '../../../nodejs/src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function main() {
  const workDir = join(__dirname, '..');

  const prompt = await readFile(join(workDir, 'prompt.txt'), 'utf-8');
  console.log('=== Prompt ===');
  console.log(prompt);
  console.log();

  const data = await readFile(join(workDir, 'data.txt'), 'utf-8');
  console.log('=== Data Preview ===');
  console.log(data.split('\n').slice(0, 25).join('\n'));
  console.log('...\n');

  const apiKey =
    process.env.ANTHROPIC_AUTH_TOKEN || process.env.ANTHROPIC_API_KEY || '';
  if (!apiKey) {
    console.error('ERROR: ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY not set.');
    process.exit(1);
  }

  const config = resolveConfig({
    apiKey,
    baseUrl: process.env.ANTHROPIC_BASE_URL || 'https://api.anthropic.com',
    model: process.env.ANTHROPIC_MODEL || 'claude-sonnet-4-6',
    workDir,
    maxTurns: 50,
    callback: (event) => {
      if (event.type === 'tool_use_start') {
        console.log(`[Tool] ${event.data.name} called`);
      }
    },
  });

  const agent = new Agent(config);

  console.log('=== Agent Output ===\n');

  const enrichedPrompt = `Working directory contains a file named "data.txt".\n\n${prompt}`;

  for await (const event of agent.run(enrichedPrompt)) {
    switch (event.type) {
      case 'message_delta':
        process.stdout.write(event.data.text as string);
        break;
      case 'tool_use_start':
        console.log(`\n[Tool call: ${event.data.name}]`);
        break;
      case 'complete':
        console.log('\n\n=== Complete ===');
        console.log(event.data.final_content);
        break;
      case 'error':
        console.error('\n[Error]', event.data.message);
        break;
    }
  }

  console.log('\n=== Message History ===');
  for (const msg of agent.getMessageHistory()) {
    console.log(`[${msg.role}] ${JSON.stringify(msg.content).slice(0, 200)}`);
  }

  console.log(`\n[Final state] ${agent.state}`);
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
