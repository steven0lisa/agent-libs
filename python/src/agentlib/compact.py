"""Auto compact mechanism for conversation history compression."""

from __future__ import annotations

import logging
from typing import Optional

from .client import AnthropicClient
from .config import AgentConfig
from .types import (
    ContentBlock,
    Message,
    Role,
    TextBlock,
    ToolResultBlock,
    ToolUseBlock,
)

logger = logging.getLogger(__name__)

# Maximum consecutive compact failures before giving up
MAX_CONSECUTIVE_FAILURES = 3

# Minimum messages to preserve during compaction
MIN_PRESERVED_MESSAGES = 5

# Characters per token (rough estimation for English/mixed text)
CHARS_PER_TOKEN = 4.0

# Maximum output tokens for the summary
MAX_SUMMARY_TOKENS = 4096

COMPACT_SYSTEM_PROMPT = """You are a conversation summarizer. Your task is to create a concise but comprehensive summary of the conversation history.

Rules:
1. Preserve all key facts, decisions, and outcomes
2. Keep track of file operations performed (files read, written, updated)
3. Note any errors encountered and how they were resolved
4. Maintain the context needed for the assistant to continue the conversation
5. Keep the summary as concise as possible while retaining all important information
6. Do NOT include any meta-commentary about the summarization process

Output a single coherent summary paragraph or bullet points that captures the essence of the conversation."""


def estimate_tokens(messages: list[Message]) -> int:
    """Estimate the token count for a list of messages.

    Uses a simple heuristic of ~4 characters per token.
    """
    total_chars = 0
    for msg in messages:
        for block in msg.content:
            if isinstance(block, TextBlock):
                total_chars += len(block.text)
            elif isinstance(block, ToolUseBlock):
                # Tool name + input JSON
                total_chars += len(block.name)
                import json
                total_chars += len(json.dumps(block.input))
            elif isinstance(block, ToolResultBlock):
                total_chars += len(block.content)
            # ThinkingBlock is typically not sent to API, skip
    return int(total_chars / CHARS_PER_TOKEN)


def should_compact(
    messages: list[Message],
    context_window_size: int,
    threshold_pct: float,
) -> bool:
    """Check if the conversation history should be compacted."""
    if not messages:
        return False

    threshold = int(context_window_size * threshold_pct)
    estimated = estimate_tokens(messages)

    if estimated >= threshold:
        logger.info(
            "Auto compact triggered: estimated=%d tokens, threshold=%d tokens",
            estimated,
            threshold,
        )
        return True
    return False


def _find_compact_boundary(messages: list[Message]) -> int:
    """Find the boundary index for compaction.

    We want to keep the most recent messages intact. The boundary
    determines where we split: messages before the boundary get
    summarized, messages from the boundary onward are preserved.

    We ensure:
    1. At least MIN_PRESERVED_MESSAGES are kept
    2. Tool use/result pairs are not split
    """
    if len(messages) <= MIN_PRESERVED_MESSAGES:
        return 0

    # Start from the position that leaves MIN_PRESERVED_MESSAGES
    boundary = len(messages) - MIN_PRESERVED_MESSAGES

    # Ensure we don't split a tool_use/result pair
    # If the message at boundary is a user message with tool_result,
    # look backward for the corresponding assistant message with tool_use
    if boundary > 0:
        msg = messages[boundary]
        if msg.role == Role.USER:
            has_tool_result = any(
                isinstance(b, ToolResultBlock) for b in msg.content
            )
            if has_tool_result:
                # Move boundary back to include the tool_use pair
                while boundary > 0:
                    boundary -= 1
                    prev_msg = messages[boundary]
                    if prev_msg.role == Role.ASSISTANT:
                        has_tool_use = any(
                            isinstance(b, ToolUseBlock) for b in prev_msg.content
                        )
                        if has_tool_use:
                            break

    return max(0, boundary)


def _format_messages_for_summary(messages: list[Message]) -> str:
    """Format messages into a readable text for summarization."""
    parts = []
    for msg in messages:
        role_label = "User" if msg.role == Role.USER else "Assistant"
        for block in msg.content:
            if isinstance(block, TextBlock):
                parts.append(f"[{role_label}]: {block.text}")
            elif isinstance(block, ToolUseBlock):
                import json
                parts.append(
                    f"[{role_label} Tool Use ({block.name})]: "
                    f"{json.dumps(block.input, ensure_ascii=False)}"
                )
            elif isinstance(block, ToolResultBlock):
                status = "Error" if block.is_error else "Success"
                parts.append(
                    f"[User Tool Result ({status})]: {block.content[:2000]}"
                )
    return "\n".join(parts)


async def compact_conversation(
    client: AnthropicClient,
    messages: list[Message],
    max_summary_tokens: int = MAX_SUMMARY_TOKENS,
) -> Optional[list[Message]]:
    """Compact the conversation history by summarizing older messages.

    Returns the new message history with older messages replaced by a summary,
    or None if compaction fails.
    """
    if len(messages) <= MIN_PRESERVED_MESSAGES:
        return messages

    boundary = _find_compact_boundary(messages)
    if boundary == 0:
        return messages

    old_messages = messages[:boundary]
    recent_messages = messages[boundary:]

    # Format old messages for summarization
    conversation_text = _format_messages_for_summary(old_messages)

    if not conversation_text.strip():
        return messages

    # Create the summarization request
    summary_messages = [
        Message.user(
            f"Please summarize the following conversation history concisely, "
            f"preserving all key facts, decisions, file operations, and outcomes:\n\n"
            f"{conversation_text}"
        )
    ]

    try:
        response = await client.send_messages(
            summary_messages,
            system=COMPACT_SYSTEM_PROMPT,
            tools=[],
        )

        # Extract the summary text
        summary_text = ""
        content_blocks = response.get("content", [])
        for block in content_blocks:
            if block.get("type") == "text":
                summary_text += block.get("text", "")

        if not summary_text.strip():
            logger.warning("Compact: empty summary generated, skipping")
            return None

        # Build new message history: summary + recent messages
        compact_message = Message.user(
            "[Conversation History Summary]\n\n"
            f"{summary_text}\n\n"
            "[End of Summary - Recent conversation continues below]"
        )

        new_history = [compact_message] + recent_messages

        logger.info(
            "Compact: reduced from %d to %d messages",
            len(messages),
            len(new_history),
        )

        return new_history

    except Exception as e:
        logger.warning("Compact failed: %s", e)
        return None


async def auto_compact_if_needed(
    config: AgentConfig,
    client: AnthropicClient,
    message_history: list[Message],
    consecutive_failures: int,
) -> tuple[list[Message], int]:
    """Check if auto compact is needed and perform it.

    Returns (updated_history, updated_consecutive_failures).
    """
    if not config.auto_compact:
        return message_history, consecutive_failures

    # Circuit breaker: stop trying after too many consecutive failures
    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES:
        return message_history, consecutive_failures

    if not should_compact(
        message_history,
        config.context_window_size,
        config.auto_compact_threshold_pct,
    ):
        return message_history, consecutive_failures

    # Attempt compaction
    result = await compact_conversation(client, message_history)

    if result is not None:
        return result, 0  # Reset failure count on success
    else:
        return message_history, consecutive_failures + 1
