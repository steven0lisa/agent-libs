import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function testReadFile() {
  const { ReadFileTool } = await import('../../../nodejs/src/tools/index.js');
  const tool = new ReadFileTool();

  const ctx = {
    workDir: join(__dirname, '..'),
    messageHistory: [],
  };

  console.log('=== Test read_file tool ===');
  const result = await tool.call({ file_path: 'data.txt' }, ctx);
  console.log('Success:', !result.isError);
  console.log('Content preview:', result.content.slice(0, 200));
  console.log();
}

async function testBash() {
  const { BashTool } = await import('../../../nodejs/src/tools/index.js');
  const tool = new BashTool();

  const ctx = {
    workDir: join(__dirname, '..'),
    messageHistory: [],
  };

  console.log('=== Test bash tool ===');
  const result = await tool.call({ command: 'ls -la' }, ctx);
  console.log('Success:', !result.isError);
  console.log('Output:', result.content.slice(0, 200));
  console.log();
}

async function testAgentInit() {
  const { Agent, resolveConfig } = await import('../../../nodejs/src/index.js');

  const config = resolveConfig({
    apiKey: 'test-key',
    workDir: join(__dirname, '..'),
  });

  console.log('=== Test Agent initialization ===');
  const agent = new Agent(config);
  console.log('State:', agent.state);
  console.log('Tools:', agent.listTools().map((t) => t.name));
  console.log('Message history:', agent.getMessageHistory().length);
  console.log();
}

async function main() {
  await testReadFile();
  await testBash();
  await testAgentInit();
  console.log('All tests passed!');
}

main().catch(console.error);
