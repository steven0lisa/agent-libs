/** Example: Register a custom tool with the agent. */

import {
  Agent, resolveConfig, ITool, ToolContext, successResult, errorResult,
} from '../src/index.js';

class CalculatorTool implements ITool {
  readonly name = 'calculator';
  readonly description = 'Perform basic arithmetic: add, subtract, multiply, divide.';
  readonly isReadOnly = true;

  readonly inputSchema = {
    type: 'object',
    properties: {
      operation: { type: 'string', enum: ['add', 'subtract', 'multiply', 'divide'] },
      a: { type: 'number' },
      b: { type: 'number' },
    },
    required: ['operation', 'a', 'b'],
  };

  async call(input: Record<string, unknown>, _ctx: ToolContext) {
    const operation = input.operation as string;
    const a = Number(input.a);
    const b = Number(input.b);

    if (operation === 'divide' && b === 0) {
      return errorResult('Cannot divide by zero');
    }

    let result = 0;
    switch (operation) {
      case 'add': result = a + b; break;
      case 'subtract': result = a - b; break;
      case 'multiply': result = a * b; break;
      case 'divide': result = a / b; break;
      default: return errorResult(`Unknown operation: ${operation}`);
    }
    return successResult(String(result));
  }
}

async function main() {
  const config = resolveConfig({ apiKey: process.env.ANTHROPIC_AUTH_TOKEN || '' });
  const agent = new Agent(config);
  agent.registerTool(new CalculatorTool());

  for await (const event of agent.run('What is 123 multiplied by 456?')) {
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
