/** Curl tool with security policy. */

import { ITool, Pattern, ToolContext } from '../config.js';
import { successResult, errorResult } from '../types.js';
import { checkSecurityPolicy } from '../utils/security.js';

export class CurlTool implements ITool {
  readonly name = 'curl';
  readonly description = 'Make an HTTP request.';
  readonly isReadOnly = true;
  private whitelist: Pattern[];
  private blacklist: Pattern[];

  constructor(whitelist?: Pattern[], blacklist?: Pattern[]) {
    this.whitelist = whitelist || [];
    this.blacklist = blacklist || [];
  }

  readonly inputSchema = {
    type: 'object',
    properties: {
      url: { type: 'string' },
      method: { type: 'string', enum: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'], default: 'GET' },
      headers: { type: 'object' },
      body: { type: 'string' },
      timeout: { type: 'integer', default: 30000 },
    },
    required: ['url'],
  };

  async call(input: Record<string, unknown>, context: ToolContext) {
    const url = input.url as string;
    const method = (input.method as string) ?? 'GET';
    const body = input.body as string | undefined;
    const headers = input.headers as Record<string, string> | undefined;
    const timeout = (input.timeout as number) ?? 30_000;

    const { allowed, reason } = checkSecurityPolicy(
      url,
      this.whitelist,
      this.blacklist,
      true
    );
    if (!allowed) {
      return errorResult(`URL blocked by security policy: ${reason}`);
    }

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);

    try {
      const response = await fetch(url, {
        method,
        headers,
        body,
        signal: controller.signal,
      });
      const text = await response.text();
      return successResult(text);
    } catch (e) {
      return errorResult(`HTTP error: ${e}`);
    } finally {
      clearTimeout(timer);
    }
  }
}
