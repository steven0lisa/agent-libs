/** Anthropic API client. */

import { AgentConfig } from './config.js';
import { Message } from './types.js';

export interface ToolDefinition {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

export interface APIRequest {
  model: string;
  messages: Message[];
  system?: string;
  tools?: ToolDefinition[];
  max_tokens: number;
  stream?: boolean;
}

export class AnthropicClient {
  private config: Required<AgentConfig>;

  constructor(config: Required<AgentConfig>) {
    this.config = config;
  }

  async *streamMessages(
    messages: Message[],
    system: string,
    tools: ToolDefinition[]
  ): AsyncGenerator<Record<string, unknown>> {
    const request: APIRequest = {
      model: this.config.model,
      messages,
      system,
      tools,
      max_tokens: this.config.maxTokens,
      stream: true,
    };

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.config.timeoutMs);

    try {
      const response = await fetch(`${this.config.baseUrl}/v1/messages`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': this.config.apiKey,
          'anthropic-version': '2023-06-01',
          'Accept': 'text/event-stream',
        },
        body: JSON.stringify(request),
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`);
      }

      const reader = response.body?.getReader();
      if (!reader) throw new Error('No response body');

      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() ?? '';

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') return;
            try {
              yield JSON.parse(data);
            } catch {
              // skip
            }
          }
        }
      }
    } finally {
      clearTimeout(timer);
    }
  }

  async sendMessages(
    messages: Message[],
    system: string,
    tools: ToolDefinition[],
  ): Promise<{ content: Array<{ type: string; text?: string }> }> {
    const request = {
      model: this.config.model,
      messages,
      system,
      tools,
      max_tokens: this.config.maxTokens,
      stream: false,
    };

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.config.timeoutMs);

    try {
      const response = await fetch(`${this.config.baseUrl}/v1/messages`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': this.config.apiKey,
          'anthropic-version': '2023-06-01',
        },
        body: JSON.stringify(request),
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`);
      }

      return (await response.json()) as { content: Array<{ type: string; text?: string }> };
    } finally {
      clearTimeout(timer);
    }
  }
}
