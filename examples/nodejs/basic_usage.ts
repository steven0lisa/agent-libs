/** Basic usage example: Run the agent with a simple prompt. */

import { Agent, AgentState, resolveConfig } from '../src/index.js';

async function main() {
  // The caller must set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY.
  const config = resolveConfig({
    apiKey: process.env.ANTHROPIC_AUTH_TOKEN || '',
    baseUrl: process.env.ANTHROPIC_BASE_URL,
    model: process.env.ANTHROPIC_MODEL,
  });

  const agent = new Agent(config);

  for await (const event of agent.run('Please list the files in the current directory')) {
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
