/** Example: Manage agent lifecycle - pause, resume, and stop. */

import { Agent, resolveConfig } from '../src/index.js';

async function main() {
  const config = resolveConfig({ apiKey: process.env.ANTHROPIC_AUTH_TOKEN || '' });
  const agent = new Agent(config);

  // Schedule lifecycle changes.
  setTimeout(() => {
    console.log('\n[Lifecycle] Pausing agent...');
    agent.pause();
  }, 3000);

  setTimeout(() => {
    console.log('\n[Lifecycle] Resuming agent...');
    agent.resume();
  }, 5000);

  setTimeout(() => {
    console.log('\n[Lifecycle] Stopping agent...');
    agent.stop();
  }, 10000);

  for await (const event of agent.run('Explore the codebase and tell me about it')) {
    if (event.type === 'turn_start') {
      console.log(`\n[Turn ${event.data.turn} started]`);
    } else if (event.type === 'message_delta') {
      process.stdout.write(event.data.text as string);
    } else if (event.type === 'complete') {
      console.log('\n\n[Complete]', event.data.final_content);
    } else if (event.type === 'error') {
      console.error('\n[Error]', event.data.message);
    }
  }

  console.log(`\n[Final state] ${agent.state}`);
}

main().catch(console.error);
