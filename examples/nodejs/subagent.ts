/** Example: Enable subagent support and let the model delegate tasks. */

import { Agent, resolveConfig } from '../src/index.js';

async function main() {
  const config = resolveConfig({
    apiKey: process.env.ANTHROPIC_AUTH_TOKEN || '',
    enableSubagent: true,
    subagentMaxTurns: 20,
  });

  const agent = new Agent(config);

  for await (const event of agent.run(
    'Explore this codebase using subagents to parallelize exploration'
  )) {
    if (event.type === 'message_delta') {
      process.stdout.write(event.data.text as string);
    } else if (event.type === 'complete') {
      console.log('\n\n[Complete]', event.data.final_content);
    } else if (event.type === 'error') {
      console.error('\n[Error]', event.data.message);
    }
  }
}

main().catch(console.error);
