/** Core types for AgentLib. */

export type Role = 'user' | 'assistant';

export type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'tool_use'; name: string; id: string; input: Record<string, unknown> }
  | { type: 'tool_result'; tool_use_id: string; content: string; is_error?: boolean }
  | { type: 'thinking'; thinking: string; signature?: string };

export function isToolUse(block: ContentBlock): block is Extract<ContentBlock, { type: 'tool_use' }> {
  return block.type === 'tool_use';
}

export function isText(block: ContentBlock): block is Extract<ContentBlock, { type: 'text' }> {
  return block.type === 'text';
}

export interface Message {
  role: Role;
  content: ContentBlock[];
}

export function userMessage(text: string): Message {
  return { role: 'user', content: [{ type: 'text', text }] };
}

export function assistantMessage(blocks: ContentBlock[]): Message {
  return { role: 'assistant', content: blocks };
}

export type EventType =
  | 'turn_start'
  | 'message_start'
  | 'message_delta'
  | 'thinking_delta'
  | 'message_end'
  | 'tool_use_start'
  | 'tool_use_end'
  | 'error'
  | 'complete';

export interface Event {
  type: EventType;
  data: Record<string, unknown>;
}

export function turnStartEvent(turn: number): Event {
  return { type: 'turn_start', data: { turn } };
}

export function messageStartEvent(): Event {
  return { type: 'message_start', data: {} };
}

export function messageDeltaEvent(text: string): Event {
  return { type: 'message_delta', data: { text } };
}

export function thinkingDeltaEvent(thinking: string): Event {
  return { type: 'thinking_delta', data: { thinking } };
}

export function messageEndEvent(): Event {
  return { type: 'message_end', data: {} };
}

export function toolUseStartEvent(name: string, id: string, input: Record<string, unknown>): Event {
  return { type: 'tool_use_start', data: { name, id, input } };
}

export function toolUseEndEvent(name: string, id: string, result: ToolResult): Event {
  return { type: 'tool_use_end', data: { name, id, result } };
}

export function errorEvent(message: string): Event {
  return { type: 'error', data: { message } };
}

export function completeEvent(finalContent: string): Event {
  return { type: 'complete', data: { final_content: finalContent } };
}

export interface ToolResult {
  content: string;
  isError: boolean;
}

export function successResult(content: string): ToolResult {
  return { content, isError: false };
}

export function errorResult(content: string): ToolResult {
  return { content, isError: true };
}
