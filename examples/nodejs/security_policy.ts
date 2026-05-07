/** Example: Use whitelist/blacklist security policies for bash and curl. */

import { Agent, resolveConfig, Pattern } from '../src/index.js';

async function main() {
  const config = resolveConfig({
    apiKey: process.env.ANTHROPIC_AUTH_TOKEN || '',
    // Bash: whitelist allows git and ls; blacklist blocks rm.
    bashWhitelist: [
      new Pattern('git *', 'wildcard'),
      new Pattern('ls *', 'wildcard'),
    ],
    bashBlacklist: [
      new Pattern('rm *', 'wildcard'),
      new Pattern('^dd\\s', 'regex'),
    ],
    // Curl: only allow example.com subdomains.
    curlWhitelist: [
      new Pattern('*.example.com/*', 'wildcard'),
    ],
    curlBlacklist: [
      new Pattern('evil\\.com|malicious\\.org', 'regex'),
    ],
  });

  const agent = new Agent(config);

  for await (const event of agent.run('Show me the git status of this repository')) {
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
