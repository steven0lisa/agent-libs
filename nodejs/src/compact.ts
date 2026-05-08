/** Auto compact mechanism for conversation history compression. */

import { AnthropicClient } from './client.js';
import { AgentConfig } from './config.js';
import { ContentBlock, isToolUse, isText, Message, userMessage } from './types.js';

/** Maximum consecutive compact failures before giving up. */
const MAX_CONSECUTIVE_FAILURES = 3;

/** Minimum messages to preserve during compaction. */
const MIN_PRESERVED_MESSAGES = 5;

/** Characters per token (rough estimation for English/mixed text). */
const CHARS_PER_TOKEN = 4.0;

/** Maximum output tokens for the summary. */
const MAX_SUMMARY_TOKENS = 4096;

const COMPACT_SYSTEM_PROMPT = `You are a conversation summarizer. Your task is to create a concise but comprehensive summary of the conversation history.

Rules:
1. Preserve all key facts, decisions, and outcomes
2. Keep track of file operations performed (files read, written, updated)
3. Note any errors encountered and how they were resolved
4. Maintain the context needed for the assistant to continue the conversation
5. Keep the summary as concise as possible while retaining all important information
6. Do NOT include any meta-commentary about the summarization process

Output a single coherent summary paragraph or bullet points that captures the essence of the conversation.`;

/**
 * Estimate the token count for a list of messages.
 * Uses a simple heuristic of ~4 characters per token.
 */
export function estimateTokens(messages: Message[]): number {
  let totalChars = 0;
  for (const msg of messages) {
    for (const block of msg.content) {
      if (isText(block)) {
        totalChars += block.text.length;
      } else if (isToolUse(block)) {
        totalChars += block.name.length;
        totalChars += JSON.stringify(block.input).length;
      } else if (block.type === 'tool_result') {
        totalChars += block.content.length;
      }
      // thinking blocks are typically not sent to API, skip
    }
  }
  return Math.floor(totalChars / CHARS_PER_TOKEN);
}

/**
 * Check if the conversation history should be compacted.
 */
export function shouldCompact(
  messages: Message[],
  contextWindowSize: number,
  thresholdPct: number,
): boolean {
  if (messages.length === 0) return false;

  const threshold = Math.floor(contextWindowSize * thresholdPct);
  const estimated = estimateTokens(messages);

  return estimated >= threshold;
}

/**
 * Find the boundary index for compaction.
 *
 * We want to keep the most recent messages intact. The boundary
 * determines where we split: messages before the boundary get
 * summarized, messages from the boundary onward are preserved.
 *
 * We ensure:
 * 1. At least MIN_PRESERVED_MESSAGES are kept
 * 2. Tool use/result pairs are not split
 */
function findCompactBoundary(messages: Message[]): number {
  if (messages.length <= MIN_PRESERVED_MESSAGES) return 0;

  let boundary = messages.length - MIN_PRESERVED_MESSAGES;

  // Ensure we don't split a tool_use/result pair
  if (boundary > 0) {
    const msg = messages[boundary];
    if (msg.role === 'user') {
      const hasToolResult = msg.content.some((b) => b.type === 'tool_result');
      if (hasToolResult) {
        // Move boundary back to include the tool_use pair
        while (boundary > 0) {
          boundary--;
          const prevMsg = messages[boundary];
          if (prevMsg.role === 'assistant') {
            const hasToolUse = prevMsg.content.some(isToolUse);
            if (hasToolUse) break;
          }
        }
      }
    }
  }

  return Math.max(0, boundary);
}

/**
 * Format messages into a readable text for summarization.
 */
function formatMessagesForSummary(messages: Message[]): string {
  const parts: string[] = [];
  for (const msg of messages) {
    const roleLabel = msg.role === 'user' ? 'User' : 'Assistant';
    for (const block of msg.content) {
      if (isText(block)) {
        parts.push(`[${roleLabel}]: ${block.text}`);
      } else if (isToolUse(block)) {
        parts.push(
          `[${roleLabel} Tool Use (${block.name})]: ${JSON.stringify(block.input)}`,
        );
      } else if (block.type === 'tool_result') {
        const status = block.is_error ? 'Error' : 'Success';
        parts.push(
          `[User Tool Result (${status})]: ${block.content.slice(0, 2000)}`,
        );
      }
    }
  }
  return parts.join('\n');
}

/**
 * Compact the conversation history by summarizing older messages.
 *
 * Returns the new message history with older messages replaced by a summary,
 * or null if compaction fails.
 */
export async function compactConversation(
  client: AnthropicClient,
  messages: Message[],
): Promise<Message[] | null> {
  if (messages.length <= MIN_PRESERVED_MESSAGES) return messages;

  const boundary = findCompactBoundary(messages);
  if (boundary === 0) return messages;

  const oldMessages = messages.slice(0, boundary);
  const recentMessages = messages.slice(boundary);

  // Format old messages for summarization
  const conversationText = formatMessagesForSummary(oldMessages);

  if (!conversationText.trim()) return messages;

  // Create the summarization request
  const summaryMessages: Message[] = [
    userMessage(
      `Please summarize the following conversation history concisely, ` +
        `preserving all key facts, decisions, file operations, and outcomes:\n\n` +
        `${conversationText}`,
    ),
  ];

  try {
    const response = await client.sendMessages(
      summaryMessages,
      COMPACT_SYSTEM_PROMPT,
      [],
    );

    // Extract the summary text
    let summaryText = '';
    const contentBlocks = response.content ?? [];
    for (const block of contentBlocks) {
      if (block.type === 'text' && block.text) {
        summaryText += block.text;
      }
    }

    if (!summaryText.trim()) {
      return null;
    }

    // Build new message history: summary + recent messages
    const compactMessage = userMessage(
      `[Conversation History Summary]\n\n` +
        `${summaryText}\n\n` +
        `[End of Summary - Recent conversation continues below]`,
    );

    const newHistory = [compactMessage, ...recentMessages];

    return newHistory;
  } catch (e) {
    return null;
  }
}

/**
 * Check if auto compact is needed and perform it.
 *
 * Returns { history, failures } with the updated message history and
 * consecutive failure count.
 */
export async function autoCompactIfNeeded(
  config: Required<AgentConfig>,
  client: AnthropicClient,
  messages: Message[],
  consecutiveFailures: number,
): Promise<{ history: Message[]; failures: number }> {
  if (!config.autoCompact) {
    return { history: messages, failures: consecutiveFailures };
  }

  // Circuit breaker: stop trying after too many consecutive failures
  if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
    return { history: messages, failures: consecutiveFailures };
  }

  if (
    !shouldCompact(
      messages,
      config.contextWindowSize,
      config.autoCompactThresholdPct,
    )
  ) {
    return { history: messages, failures: consecutiveFailures };
  }

  // Attempt compaction
  const result = await compactConversation(client, messages);

  if (result !== null) {
    return { history: result, failures: 0 };
  } else {
    return { history: messages, failures: consecutiveFailures + 1 };
  }
}
