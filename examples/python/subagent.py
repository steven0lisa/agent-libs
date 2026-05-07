"""Example: Enable subagent support and let the model delegate tasks."""

import asyncio
import os

from agentlib import Agent, AgentConfig


async def main():
    config = AgentConfig(
        api_key=os.environ.get("ANTHROPIC_AUTH_TOKEN", ""),
        model=os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
        enable_subagent=True,
        subagent_max_turns=20,
    )

    agent = Agent(config)

    async for event in agent.run(
        "Explore this codebase. Use subagents to parallelize exploration of "
        "different directories, then combine the findings."
    ):
        if event.type.value == "message_delta":
            print(event.data["text"], end="", flush=True)
        elif event.type.value == "complete":
            print(f"\n\n[Complete] {event.data['final_content']}")
        elif event.type.value == "error":
            print(f"\n[Error] {event.data['message']}")


if __name__ == "__main__":
    asyncio.run(main())
