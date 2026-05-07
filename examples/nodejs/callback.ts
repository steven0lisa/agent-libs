/** Example: Use a callback to log every step of the agent's execution. */

import { Agent, resolveConfig, Event } from '../src/index.js';

async function main() {
  function onEvent(event: Event) {
    if (event.type === 'tool_use_start') {
      console.log(`[Callback] Tool '${event.data.name}' called`);
    } else if (event.type === 'tool_use_end') {
      const result = event.data.result as { isError: boolean };
      const status = result.isError ? 'ERROR' : 'OK';
      console.log(`[Callback] Tool '${event.data.name}' finished: ${status}`);
    } else if (event.type === 'error') {
      console.log(`[Callback] Error: ${event.data.message}`);
    }
  }

  const config = resolveConfig({
    apiKey: process.env.ANTHROPIC_AUTH_TOKEN || '',
    callback: onEvent,
  });

  const agent = new Agent(config);

  for await (const event of agent.run('Read README.md and summarize it')) {
    if (event.type === 'message_delta') {
      process.stdout.write(event.data.text as string);
    } else if (event.type === 'complete') {
      console.log('\n\n[Complete]', event.data.final_content);
    }
  }
}

main().catch(console.error);
